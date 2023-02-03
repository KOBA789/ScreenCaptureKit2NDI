use std::sync::Arc;

use cocoa_foundation::foundation::NSInteger;
use core_graphics_types::geometry::{CGPoint, CGRect, CGSize};

use framework_sys as fw_sys;
use sckit::{ContentFilter, ShareableContent, Stream, StreamConfig, StreamOutput};

use crate::ndi;

pub struct Grabber {
    sender: ndi::Sender,
}

impl Grabber {
    pub fn new() -> Grabber {
        let sender = ndi::Sender::new("sckitndi");
        Self { sender }
    }

    pub fn start(self: &Arc<Self>) {
        let this = self.clone();
        ShareableContent::get(move |ret| {
            let this = this.clone();
            let shareable_content = ret.unwrap();
            let displays = shareable_content.displays();
            let first_display = &displays[0];

            let apps = shareable_content.applications();
            let excluding_applications = apps.iter().filter(|a| {
                a.bundle_identifier()
                    .map(|id| {
                        [
                            "com.koba789.sckitndi",
                            "com.apple.controlcenter",
                            "com.apple.dock",
                            "com.apple.TextInputMenuAgent",
                            "com.1password.1password",
                            "com.getdropbox.dropbox",
                            "com.getdropbox.dropbox",
                            "com.apple.notificationcenterui",
                            "com.justsystems.inputmethod.atok32",
                            "com.apple.systemuiserver",
                            "com.newtek.Application-Mac-NDI-StudioMonitor",
                            "com.hnc.Discord",
                        ]
                        .iter()
                        .any(|&block| block == id)
                    })
                    .unwrap_or(false)
            });
            let filter = ContentFilter::init_with_display_excluding_applications_excepting_windows(
                first_display,
                excluding_applications,
                [],
            );
            let mut stream_config = StreamConfig::default();
            let source_rect = CGRect::new(
                &CGPoint::new(1920. / 2., 1080. / 2.),
                &CGSize::new(1920., 1080.),
            );
            let destination_rect = CGRect::new(&CGPoint::new(0., 0.), &source_rect.size);
            stream_config.set_width(source_rect.size.width as usize);
            stream_config.set_height(source_rect.size.height as usize);
            stream_config.set_source_rect(source_rect);
            stream_config.set_destination_rect(destination_rect);
            stream_config.set_queue_depth(5);
            let stream = Stream::new(filter, stream_config);
            let did_add_output = stream.add_stream_output(this as Arc<dyn StreamOutput>, 0);
            assert!(did_add_output);
            stream.start_capture(|ret| {
                ret.unwrap();
                println!("started");
            });
        });
    }
}

impl StreamOutput for Grabber {
    fn did_output_sample_buffer_of_type(
        &self,
        _stream: Stream,
        sample_buffer: fw_sys::CMSampleBufferRef,
        _type: NSInteger,
    ) {
        unsafe {
            let pixel_buffer = fw_sys::CMSampleBufferGetImageBuffer(sample_buffer);
            let width = fw_sys::CVPixelBufferGetWidth(pixel_buffer);
            let height = fw_sys::CVPixelBufferGetHeight(pixel_buffer);
            fw_sys::CVPixelBufferLockBaseAddress(pixel_buffer, 1);
            let data = fw_sys::CVPixelBufferGetBaseAddress(pixel_buffer) as *mut u8;
            self.sender.send_video(width as i32, height as i32, data);
            fw_sys::CVPixelBufferUnlockBaseAddress(pixel_buffer, 1);
        }
    }
}
