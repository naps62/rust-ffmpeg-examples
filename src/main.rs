mod av;

use std::path::PathBuf;

fn main() {
    let path = "samples/small_bunny_1080p_60fps.mp4";
    assert!(PathBuf::from(path).exists());

    unsafe {
        let ctx = av::open_file(&path);

        av::find_stream_info(ctx);
        av::debug_ctx(ctx);

        let mut stream_ctx = av::open_video_stream(ctx, 0);

        av::read_frame(&mut stream_ctx);
    }
}
