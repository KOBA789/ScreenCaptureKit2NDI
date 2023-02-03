use std::{ffi::CString, ptr::null};

pub struct Sender {
    ndi_send_instance: ndi_sys::NDIlib_send_instance_t,
}

unsafe impl Sync for Sender {}
unsafe impl Send for Sender {}

impl Sender {
    pub fn new(ndi_name: &str) -> Self {
        let ndi_name = CString::new(ndi_name).unwrap();
        let send_decr = ndi_sys::NDIlib_send_create_t {
            p_ndi_name: ndi_name.as_ptr(),
            p_groups: null(),
            clock_video: false,
            clock_audio: false,
        };
        let send_instance = unsafe { ndi_sys::NDIlib_send_create(&send_decr) };
        Self { ndi_send_instance: send_instance }
    }

    pub unsafe fn send_video(&self, width: i32, height: i32, data: *mut u8) {
        let video_frame = ndi_sys::NDIlib_video_frame_v2_t {
            xres: width as i32,
            yres: height as i32,
            FourCC: ndi_sys::NDIlib_FourCC_video_type_e::NDIlib_FourCC_video_type_BGRX,
            frame_format_type: ndi_sys::NDIlib_frame_format_type_e::NDIlib_frame_format_type_progressive,
            p_data: data,
            ..Default::default()
        };
        ndi_sys::NDIlib_send_send_video_v2(self.ndi_send_instance, &video_frame);
    }
}
