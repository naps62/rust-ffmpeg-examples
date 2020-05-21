extern crate ffmpeg_dev;

use std::os::raw::c_char;
use std::slice;

use ffmpeg_dev::extra::defs::{averror, averror_eof, eagain};
use ffmpeg_dev::sys::{
    self, AVCodec, AVCodecContext, AVCodecParameters, AVDictionary, AVFormatContext, AVFrame,
    AVInputFormat, AVPacket, AVStream,
};
use std::ffi::{CStr, CString};
use std::ptr::null_mut;

#[derive(Debug)]
pub struct VideoCtx {
    ctx: *mut AVFormatContext,
    codec_ctx: *mut AVCodecContext,
    packet: *mut AVPacket,
    frame: *mut AVFrame,
}

pub unsafe fn open_file(path: &str) -> *mut AVFormatContext {
    let mut ctx: *mut AVFormatContext = sys::avformat_alloc_context();
    let path_str = CString::new(path).expect("could not alloc CString");
    let input_format: *mut AVInputFormat = null_mut();
    let mut options: *mut AVDictionary = null_mut();

    sys::avformat_open_input(&mut ctx, path_str.as_ptr(), input_format, &mut options);

    ctx
}

pub unsafe fn find_stream_info(ctx: *mut AVFormatContext) {
    sys::avformat_find_stream_info(ctx, null_mut());
}

pub unsafe fn debug_ctx(ctx: *mut AVFormatContext) {
    let c = *ctx;

    let name = c_str_to_string((*c.iformat).long_name);
    let streams = get_streams(ctx);

    println!("Format {} | # Streams {}", name, streams.len());

    for (i, stream_ptr) in streams.iter().enumerate() {
        println!("\nStream #{}:", i);

        let stream = *stream_ptr;
        let codec_params_ptr: *mut AVCodecParameters = (*stream).codecpar;
        let codec_params = *codec_params_ptr;

        let codec: *mut AVCodec = sys::avcodec_find_decoder(codec_params.codec_id);
        let codec_name = c_str_to_string((*codec).long_name);

        println!(
            "Codec {}, ID {}, bit rate {}",
            codec_name,
            (*codec).id,
            codec_params.bit_rate
        );
        match codec_params.codec_type {
            sys::AVMediaType_AVMEDIA_TYPE_VIDEO => {
                println!(
                    "Video codec: resolution {} x {}",
                    codec_params.width, codec_params.height
                );
            }
            sys::AVMediaType_AVMEDIA_TYPE_AUDIO => {
                println!(
                    "Audio codec: {} channels, sample rate {}",
                    codec_params.channels, codec_params.sample_rate
                );
            }
            sys::AVMediaType_AVMEDIA_TYPE_SUBTITLE => {
                println!("Subtitles track");
            }

            x => unreachable!("Found unexpected codec type {:?}", x),
        }
    }
}

pub unsafe fn get_streams<'a>(ctx: *mut AVFormatContext) -> &'a [*mut AVStream] {
    let ptr = (*ctx).streams;
    let count = (*ctx).nb_streams as usize;

    slice::from_raw_parts(ptr, count)
}

pub unsafe fn get_stream(ctx: *mut AVFormatContext, i: usize) -> *mut AVStream {
    let streams = get_streams(ctx);

    streams[i]
}

pub unsafe fn open_video_stream(ctx: *mut AVFormatContext, i: usize) -> VideoCtx {
    let stream = get_stream(ctx, i);

    let codec_params: *mut AVCodecParameters = (*stream).codecpar;

    assert_eq!(
        (*codec_params).codec_type,
        sys::AVMediaType_AVMEDIA_TYPE_VIDEO,
        "stream #{} is not a video stream",
        i
    );

    let codec = sys::avcodec_find_decoder((*codec_params).codec_id);

    let codec_ctx: *mut AVCodecContext = sys::avcodec_alloc_context3(codec);

    sys::avcodec_parameters_to_context(codec_ctx, codec_params);

    sys::avcodec_open2(codec_ctx, codec, null_mut());

    let packet = sys::av_packet_alloc();
    let frame = sys::av_frame_alloc();

    VideoCtx {
        ctx,
        codec_ctx,
        packet,
        frame,
    }
}

pub unsafe fn read_frame(ctx: &mut VideoCtx) {
    let mut packets_to_process = 7;

    while sys::av_read_frame(ctx.ctx, ctx.packet) >= 0 {
        if (*ctx.packet).stream_index == 0 {
            println!("AVPacket->pts {}", (*ctx.packet).pts);
            let response = decode_packet(ctx);

            if response < 0 {
                break;
            }

            if packets_to_process == 0 {
                break;
            }
            packets_to_process = packets_to_process - 1;
        }
    }
}

pub unsafe fn decode_packet(ctx: &mut VideoCtx) -> i32 {
    let mut response;

    response = sys::avcodec_send_packet(ctx.codec_ctx, ctx.packet);

    if response < 0 {
        println!(
            "error {} sending packet to decoder: {}",
            response,
            averror_to_str(response)
        );
        return response;
    }

    while response >= 0 {
        response = sys::avcodec_receive_frame(ctx.codec_ctx, ctx.frame);

        // if (response == AVERROR(EAGAIN) || response == AVERROR_EOF) {
        //   break;
        // } else if (response < 0) {
        //   logging("Error while receiving a frame from the decoder: %s", av_err2str(response));
        //   return response;
        // }

        if response == averror(eagain()) || response == averror(averror_eof()) {
            break;
        } else if response < 0 {
            println!(
                "Error receiving frame from decoder. {}",
                averror_to_str(response)
            );
            return response;
        }

        if response >= 0 {
            let frame: AVFrame = *ctx.frame;

            let type_char =
                std::char::from_u32(sys::av_get_picture_type_char(frame.pict_type) as u32).unwrap();

            save_gray_frame(ctx.frame, (*ctx.codec_ctx).frame_number);

            println!(
                "Frame {:?} (type={} sized={} bytes) pts {} key_frame {} [DTS {}]",
                (*ctx.codec_ctx).frame_number,
                type_char,
                frame.pkt_size,
                frame.pts,
                frame.key_frame,
                frame.coded_picture_number,
            );
        }
    }

    0
}

pub unsafe fn save_gray_frame(frame_ptr: *mut AVFrame, number: i32) {
    use std::fs::File;
    use std::io::prelude::*;

    let name = format!("frame-{}.pmg", number);

    let frame = *frame_ptr;

    let width = frame.width;
    let height = frame.height;
    let linesize = frame.linesize[0];
    let gray_channel = slice::from_raw_parts(frame.data[0], (width * linesize) as usize);

    println!("width {}, height {}, linesize {}", width, height, linesize);

    let mut file = File::create(name).unwrap();
    write!(file, "P5\n{} {}\n{}\n", width, height, 255).unwrap();

    for i in 0..height {
        let start = (linesize * i) as usize;
        let end = start + width as usize;

        let line = &gray_channel[start..end];

        file.write_all(line).unwrap();
    }
}

unsafe fn averror_to_str(error: i32) -> String {
    let c_str = sys::strerror(error);

    c_str_to_string(c_str)
}

unsafe fn c_str_to_string(c_str: *const c_char) -> String {
    CStr::from_ptr(c_str).to_str().unwrap().to_string()
}
