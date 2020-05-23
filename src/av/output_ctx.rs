use std::ptr::null_mut;

use ffmpeg_dev::sys;

use super::utils;

pub struct OutputCtx {
    pub av: *mut sys::AVFormatContext,
    streams: Vec<*mut sys::AVStream>,
}

impl OutputCtx {
    pub unsafe fn new(path: &str) -> OutputCtx {
        let path_str = utils::str_to_c_str(path);

        let mut av: *mut sys::AVFormatContext = null_mut();

        sys::avformat_alloc_output_context2(&mut av, null_mut(), null_mut(), path_str.as_ptr());

        OutputCtx {
            av: av,
            streams: Vec::new(),
        }
    }

    pub unsafe fn add_stream(&mut self, params: *mut sys::AVCodecParameters) {
        let stream = sys::avformat_new_stream(self.av, null_mut());

        assert!(stream != null_mut(), "failed to allocate output stream");

        let response = sys::avcodec_parameters_copy((*stream).codecpar, params);

        utils::check_error(response);

        self.streams.push(stream);
    }

    pub unsafe fn open_file(&mut self, path: &str) {
        let path_str = utils::str_to_c_str(path);

        let response = sys::avio_open(
            &mut (*self.av).pb,
            path_str.as_ptr(),
            sys::AVIO_FLAG_WRITE as i32,
        );

        utils::check_error(response);

        let response = sys::avformat_write_header(self.av, null_mut());

        utils::check_error(response);
    }

    pub unsafe fn get_streams<'a>(&'a self) -> &'a [*mut sys::AVStream] {
        &self.streams[..]
    }

    pub unsafe fn get_stream(&self, i: usize) -> *mut sys::AVStream {
        self.get_streams()[i]
    }
}
