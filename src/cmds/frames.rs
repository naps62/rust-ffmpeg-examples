use crate::{av, opts};

use std::path::PathBuf;

pub fn run(args: opts::Frames) {
    let path = args.input.as_str();

    assert!(PathBuf::from(path).exists(), "file {} does not exist", path);

    unsafe {
        let ctx = av::open_file(path);

        av::find_stream_info(ctx);
        av::debug_ctx(ctx);

        let mut stream_ctx = av::open_video_stream(ctx, 0);

        av::read_frame(&mut stream_ctx);
    }
}
