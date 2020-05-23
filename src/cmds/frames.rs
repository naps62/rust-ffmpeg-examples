use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::slice;

use ffmpeg_dev::sys;

use crate::av::input_ctx::InputCtx;
use crate::opts;

pub fn run(args: opts::Frames) {
    let path = args.input.as_str();

    assert!(PathBuf::from(path).exists(), "file {} does not exist", path);

    unsafe {
        let mut ctx = InputCtx::new(path);

        ctx.open_video_stream(0);

        for _i in 0..args.number {
            ctx.read_video_frame();
            debug_frame(&ctx);
            save_gray_frame(&ctx);
        }
    }
}

pub unsafe fn debug_frame(ctx: &InputCtx) {
    let frame = *ctx.frame;
    let codec = *ctx.codec;

    let c_type_char = sys::av_get_picture_type_char(frame.pict_type) as u32;

    let type_char = std::char::from_u32(c_type_char).unwrap();

    println!(
        "Frame {:?} (type={} sized={} bytes) pts {} key_frame {} [DTS {}]",
        codec.frame_number,
        type_char,
        frame.pkt_size,
        frame.pts,
        frame.key_frame,
        frame.coded_picture_number
    )
}

pub unsafe fn save_gray_frame(ctx: &InputCtx) {
    let frame = *ctx.frame;
    let number = (*ctx.codec).frame_number;

    let name = format!("frames/{}.pmg", number);

    let width = frame.width;
    let height = frame.height;
    let linesize = frame.linesize[0];
    let gray_channel = slice::from_raw_parts(frame.data[0], (width * linesize) as usize);

    println!("Saving frame {} into {}", number, name);

    let mut file = File::create(name).unwrap();
    write!(file, "P5\n{} {}\n{}\n", width, height, 255).unwrap();

    for i in 0..height {
        let start = (linesize * i) as usize;
        let end = start + width as usize;

        let line = &gray_channel[start..end];

        file.write_all(line).unwrap();
    }
}
