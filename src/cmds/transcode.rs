use std::path::PathBuf;
use std::ptr::null_mut;
use std::slice;

use ffmpeg_dev::extra::defs::{averror, averror_eof, eagain};
use ffmpeg_dev::sys;

use crate::av::input_ctx::InputCtx;
use crate::av::output_ctx::OutputCtx;
use crate::av::utils;
use crate::opts;

pub fn run(args: opts::Transcode) {
    let input_path = args.input.as_str();
    let output_path = args.output.as_str();

    assert!(
        PathBuf::from(input_path).exists(),
        "file {} does not exist",
        input_path
    );

    unsafe {
        sys::av_register_all();

        let input = InputCtx::new(input_path);
        let mut output = OutputCtx::new(output_path);

        let in_streams = input.get_streams();
        let mut out_streams = Vec::new();
        let video_stream_index = 0;
        let mut decoder_codec: *mut sys::AVCodec;
        let mut decoder_codec_ctx: *mut sys::AVCodecContext = null_mut();
        let mut encoder_codec: *mut sys::AVCodec;
        let mut encoder_codec_ctx: *mut sys::AVCodecContext = null_mut();

        for i in 0..in_streams.len() {
            let in_stream = in_streams[i];
            let out_stream = sys::avformat_new_stream(output.av, null_mut());

            // if this is the video stream
            if (*(*in_stream).codecpar).codec_type == sys::AVMediaType_AVMEDIA_TYPE_VIDEO {
                // open decoder codec
                decoder_codec = sys::avcodec_find_decoder((*(*in_stream).codecpar).codec_id);
                decoder_codec_ctx = sys::avcodec_alloc_context3(decoder_codec);
                sys::avcodec_parameters_to_context(decoder_codec_ctx, (*in_stream).codecpar);
                sys::avcodec_open2(decoder_codec_ctx, decoder_codec, null_mut());

                // open h265 codec
                let encoder_codec_name = utils::str_to_c_str("libx265");
                let encoder_codec_priv_key = utils::str_to_c_str("x265-params");
                let encoder_codec_priv_value =
                    utils::str_to_c_str("keyint=60:min-keyint=60:scenecut=0");

                encoder_codec = sys::avcodec_find_encoder_by_name(encoder_codec_name.as_ptr());
                encoder_codec_ctx = sys::avcodec_alloc_context3(encoder_codec);

                sys::av_opt_set(
                    (*encoder_codec_ctx).priv_data,
                    encoder_codec_priv_key.as_ptr(),
                    encoder_codec_priv_value.as_ptr(),
                    0,
                );

                // encoder codec params
                (*encoder_codec_ctx).height = (*decoder_codec_ctx).height;
                (*encoder_codec_ctx).width = (*decoder_codec_ctx).width;
                let pix_fmts_array = slice::from_raw_parts((*encoder_codec).pix_fmts, 1);
                (*encoder_codec_ctx).pix_fmt = pix_fmts_array[0];

                // control rate
                (*encoder_codec_ctx).bit_rate = 2 * 1000 * 1000;
                (*encoder_codec_ctx).rc_buffer_size = 4 * 1000 * 1000;
                (*encoder_codec_ctx).rc_max_rate = 2 * 1000 * 1000;
                (*encoder_codec_ctx).rc_min_rate = (2.5f64 * 1000f64 * 1000f64) as i64;

                // time base
                let input_framerate = sys::av_guess_frame_rate(input.av, in_stream, null_mut());
                let time_base = utils::av_inv_q(input_framerate);
                (*encoder_codec_ctx).time_base = time_base;
                (*out_stream).time_base = time_base;

                sys::avcodec_open2(encoder_codec_ctx, encoder_codec, null_mut());
                sys::avcodec_parameters_from_context((*out_stream).codecpar, encoder_codec_ctx);

            // and for all other streams
            // just copy codec params
            } else {
                sys::avcodec_parameters_copy((*out_stream).codecpar, (*in_stream).codecpar);

                if (*(*output.av).oformat).flags & sys::AVFMT_GLOBALHEADER as i32 > 0 {
                    (*output.av).flags |= sys::AV_CODEC_FLAG_GLOBAL_HEADER as i32;
                }
            }

            out_streams.push(out_stream);
        }

        output.open_file(output_path);

        while sys::av_read_frame(input.av, input.packet) >= 0 {
            let index = (*input.packet).stream_index as usize;

            let in_stream = in_streams[index];
            let out_stream = out_streams[index];

            // if this is a video packet
            if index == video_stream_index {
                let response = sys::avcodec_send_packet(decoder_codec_ctx, input.packet);

                while response >= 0 {
                    let response = sys::avcodec_receive_frame(decoder_codec_ctx, input.frame);

                    if response == averror(eagain()) || response == averror_eof() {
                        break;
                    } else if response < 0 {
                        return;
                    }

                    if response >= 0 {
                        encode(
                            output.av,
                            input.frame,
                            in_stream,
                            out_stream,
                            encoder_codec_ctx,
                            (*input.packet).stream_index,
                        );
                    }
                }
            } else {
                sys::av_packet_rescale_ts(
                    input.packet,
                    (*in_stream).time_base,
                    (*out_stream).time_base,
                );
                sys::av_interleaved_write_frame(output.av, input.packet);
            }
        }

        sys::av_write_trailer(output.av);
    }
}

unsafe fn encode(
    av_ctx: *mut sys::AVFormatContext,
    frame: *mut sys::AVFrame,
    in_stream: *mut sys::AVStream,
    out_stream: *mut sys::AVStream,
    codec_ctx: *mut sys::AVCodecContext,
    index: i32,
) -> i32 {
    let mut packet = sys::av_packet_alloc();
    let mut response = sys::avcodec_send_frame(codec_ctx, frame);

    while response >= 0 {
        response = sys::avcodec_receive_packet(codec_ctx, packet);

        if response == averror(eagain()) || response == averror_eof() {
            break;
        } else if response < 0 {
            return -1;
        }

        (*packet).stream_index = index;

        let out_time = (*out_stream).time_base;
        let frame_rate = (*in_stream).avg_frame_rate;

        (*packet).duration =
            (out_time.den as i64) / (out_time.num as i64) / (frame_rate.num as i64)
                * (frame_rate.den as i64);
        sys::av_packet_rescale_ts(packet, (*in_stream).time_base, (*out_stream).time_base);
        response = sys::av_interleaved_write_frame(av_ctx, packet);
    }

    sys::av_packet_unref(packet);
    sys::av_packet_free(&mut packet);

    return 0;
}
