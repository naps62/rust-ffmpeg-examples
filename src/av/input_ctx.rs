use std::ptr::null_mut;
use std::slice;

use ffmpeg_dev::extra::defs::{averror, averror_eof, eagain};
use ffmpeg_dev::sys;

use super::{debug, utils};

pub struct InputCtx {
    pub av: *mut sys::AVFormatContext,
    pub video_stream_index: i32,
    pub codec: *mut sys::AVCodecContext,
    pub frame: *mut sys::AVFrame,
    pub packet: *mut sys::AVPacket,
}

impl InputCtx {
    pub unsafe fn new(path: &str) -> InputCtx {
        let mut av = sys::avformat_alloc_context();
        let path_str = utils::str_to_c_str(path);

        // open input file
        sys::avformat_open_input(&mut av, path_str.as_ptr(), null_mut(), null_mut());

        // load stream info
        sys::avformat_find_stream_info(av, null_mut());

        InputCtx {
            av,
            video_stream_index: 0,
            codec: null_mut(),
            packet: sys::av_packet_alloc(),
            frame: sys::av_frame_alloc(),
        }
    }

    #[allow(dead_code)]
    pub unsafe fn debug(&self) {
        debug::debug_input_ctx(self);
    }

    pub unsafe fn get_streams<'a>(&self) -> &'a [*mut sys::AVStream] {
        let ptr = (*self.av).streams;
        let count = (*self.av).nb_streams as usize;

        slice::from_raw_parts(ptr, count)
    }

    pub unsafe fn get_stream(&self, i: usize) -> *mut sys::AVStream {
        self.get_streams()[i]
    }

    pub unsafe fn open_video_stream(&mut self, i: i32) {
        let stream = self.get_stream(i as usize);

        let codec_params = (*stream).codecpar;

        assert_eq!(
            (*codec_params).codec_type,
            sys::AVMediaType_AVMEDIA_TYPE_VIDEO,
            "stream #{} is not a video stream",
            i
        );

        // find codec
        let codec = sys::avcodec_find_decoder((*codec_params).codec_id);

        // alloc ctx for codec
        let codec_ctx = sys::avcodec_alloc_context3(codec);

        // set codec params
        sys::avcodec_parameters_to_context(codec_ctx, codec_params);

        // open stream
        sys::avcodec_open2(codec_ctx, codec, null_mut());

        self.video_stream_index = i;
        self.codec = codec_ctx;
    }

    pub unsafe fn read_video_frame(&mut self) {
        while sys::av_read_frame(self.av, self.packet) >= 0 {
            if (*self.packet).stream_index == self.video_stream_index {
                let response = self.decode_packet();

                if response < 0 {
                    break;
                }
            }
        }
    }

    unsafe fn decode_packet(&mut self) -> i32 {
        let mut response;

        // decode packet
        response = sys::avcodec_send_packet(self.codec, self.packet);

        if utils::check_error(response) {
            return response;
        }

        while response >= 0 {
            response = sys::avcodec_receive_frame(self.codec, self.frame);

            // eagain -> need to try again
            // eof -> input is over, not an actual error here
            if response == averror(eagain()) || response == averror(averror_eof()) {
                break;
            } else if utils::check_error(response) {
                return response;
            }

            if response >= 0 {
                return -1;
            }
        }

        0
    }
}
