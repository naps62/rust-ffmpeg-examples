extern crate ffmpeg_dev;

use ffmpeg_dev::sys::{self, AVDictionary, AVFormatContext, AVInputFormat};
use std::ffi::{CStr, CString};
use std::path::PathBuf;
use std::ptr::null_mut;

fn main() {
    let path = "samples/TRA3106.avi";
    assert!(PathBuf::from(path).exists());

    let ctx = open_file(&path);

    let long_name = read_format_long_name(ctx);
    let duration: i64 = read_duration(ctx);

    println!("Format {}, duration {} us", long_name, duration);
}

fn open_file(path: &str) -> *mut AVFormatContext {
    let mut ctx: *mut AVFormatContext = unsafe { sys::avformat_alloc_context() };
    let path_str = CString::new(path).expect("could not alloc CString");
    let input_format: *mut AVInputFormat = null_mut();
    let mut options: *mut AVDictionary = null_mut();

    unsafe {
        sys::avformat_open_input(&mut ctx, path_str.as_ptr(), input_format, &mut options);
    }

    ctx
}

fn read_format_long_name(ctx: *mut AVFormatContext) -> String {
    unsafe {
        let iformat = *(*ctx).iformat;

        CStr::from_ptr(iformat.long_name)
    }
    .to_str()
    .unwrap()
    .to_string()
}

fn read_duration(ctx: *mut AVFormatContext) -> i64 {
    unsafe { (*ctx) }.duration
}
