use std::path::PathBuf;
use std::ptr::null_mut;

use ffmpeg_dev::sys;

use crate::av::input_ctx::InputCtx;
use crate::av::output_ctx::OutputCtx;
use crate::opts;

pub fn run(args: opts::Transmux) {
    let input_path = args.input.as_str();
    let output_path = args.output.as_str();

    assert!(
        PathBuf::from(input_path).exists(),
        "file {} does not exist",
        input_path
    );

    unsafe {
        let input = InputCtx::new(input_path);
        let mut output = OutputCtx::new(output_path);

        let in_streams = input.get_streams();
        let mut out_streams = Vec::new();

        for i in 0..in_streams.len() {
            let in_stream = in_streams[i];
            let codec = sys::avcodec_find_decoder((*(*in_stream).codecpar).codec_id);
            let codec_ctx = sys::avcodec_alloc_context3(codec);

            // open codec
            sys::avcodec_parameters_to_context(codec_ctx, (*in_stream).codecpar);
            sys::avcodec_open2(codec_ctx, codec, null_mut());

            // create output stream
            let out_stream = sys::avformat_new_stream(output.av, null_mut());
            sys::avcodec_parameters_copy((*out_stream).codecpar, (*in_stream).codecpar);

            if (*(*output.av).oformat).flags & sys::AVFMT_GLOBALHEADER as i32 > 0 {
                (*output.av).flags |= sys::AV_CODEC_FLAG_GLOBAL_HEADER as i32;
            }

            out_streams.push(out_stream);
        }

        output.open_file(output_path);

        loop {
            let ret = sys::av_read_frame(input.av, input.packet);

            if ret < 0 {
                break;
            }

            let index = (*input.packet).stream_index as usize;

            let in_stream = in_streams[index];
            let out_stream = out_streams[index];

            sys::av_packet_rescale_ts(
                input.packet,
                (*in_stream).time_base,
                (*out_stream).time_base,
            );
            sys::av_interleaved_write_frame(output.av, input.packet);
        }

        sys::av_write_trailer(output.av);
    }
}
