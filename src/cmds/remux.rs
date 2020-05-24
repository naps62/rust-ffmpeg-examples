use std::path::PathBuf;
use std::ptr::null_mut;

use ffmpeg_dev::sys;

use crate::av::input_ctx::InputCtx;
use crate::av::output_ctx::OutputCtx;
use crate::av::utils;
use crate::opts;

pub fn run(args: opts::Remux) {
    let input_path = args.input.as_str();
    let output_path = args.output.as_str();

    assert!(
        PathBuf::from(input_path).exists(),
        "file {} does not exist",
        input_path
    );

    unsafe {
        let mut input = InputCtx::new(input_path);
        let mut output = OutputCtx::new(output_path);

        let in_streams = input.get_streams();
        let mut out_streams = Vec::new();

        for i in 0..in_streams.len() {
            let in_stream = in_streams[i];

            let out_stream = sys::avformat_new_stream(output.av, null_mut());
            assert!(out_stream != null_mut(), "failed to allocate output stream");
            let response =
                sys::avcodec_parameters_copy((*out_stream).codecpar, (*in_stream).codecpar);

            utils::check_error(response);

            out_streams.push(out_stream);
        }

        output.open_file(output_path);

        loop {
            let ret = sys::av_read_frame(input.av, input.packet);
            // let packet = *input.packet;

            if ret < 0 {
                break;
            }

            let index = (*input.packet).stream_index as usize;

            if index >= out_streams.len() {
                sys::av_packet_unref(input.packet);
                continue;
            }

            let in_stream = in_streams[index];
            let out_stream = out_streams[index];

            (*input.packet).pts = sys::av_rescale_q_rnd(
                (*input.packet).pts,
                (*in_stream).time_base,
                (*out_stream).time_base,
                sys::AVRounding_AV_ROUND_NEAR_INF | sys::AVRounding_AV_ROUND_PASS_MINMAX,
            );
            (*input.packet).dts = sys::av_rescale_q_rnd(
                (*input.packet).dts,
                (*in_stream).time_base,
                (*out_stream).time_base,
                sys::AVRounding_AV_ROUND_NEAR_INF | sys::AVRounding_AV_ROUND_PASS_MINMAX,
            );
            (*input.packet).duration = sys::av_rescale_q(
                (*input.packet).duration,
                (*in_stream).time_base,
                (*out_stream).time_base,
            );
            (*input.packet).pos = -1;

            let ret = sys::av_interleaved_write_frame(output.av, input.packet);

            utils::check_error(ret);

            sys::av_packet_unref(input.packet);
        }

        sys::av_write_trailer(output.av);
    }
}
