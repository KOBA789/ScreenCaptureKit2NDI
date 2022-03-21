use std::{
    ffi::CString,
    ptr::{null, null_mut},
};

use block::ConcreteBlock;
use cacao::{
    button::Button,
    core_graphics::display::{CGPoint, CGRect, CGSize},
    foundation::{id, NSInteger},
    layout::{Layout, LayoutConstraint},
    macos::{
        window::{Window, WindowConfig},
        App, AppDelegate,
    },
    view::{View, ViewDelegate},
};
use objc::{
    declare::ClassDecl,
    runtime::{Class, Object, Sel},
};
use once_cell::sync::{Lazy, OnceCell};
#[macro_use]
extern crate objc;

pub mod ndi;

struct SCKitNDI {
    window: Window,
    content: View<GrabberView>,
}

impl AppDelegate for SCKitNDI {
    fn did_finish_launching(&self) {
        self.window.set_minimum_content_size(400., 400.);
        self.window.set_title("A Basic Window");
        self.window.set_content_view(&self.content);

        self.window.show();
    }
}

static CAPTURE_DELEGATE: OnceCell<&'static Class> = OnceCell::new();

static GRABBER: Lazy<Grabber> = Lazy::new(Grabber::new);

fn start() {
    GRABBER.start();
}

struct GrabberView {
    button: Button,
}

impl GrabberView {
    fn new() -> Self {
        let mut button = Button::new("Start");
        button.set_action(start);

        Self { button }
    }
}

impl ViewDelegate for GrabberView {
    const NAME: &'static str = "GrabberView";

    fn did_load(&mut self, view: View) {
        view.add_subview(&self.button);

        LayoutConstraint::activate(&[self.button.top.constraint_equal_to(&view.top).offset(36.)]);
    }
}

struct Grabber {
    ndi_instance: ndi::NDIlib_send_instance_t,
}

unsafe impl Sync for Grabber {}
unsafe impl Send for Grabber {}

impl Grabber {
    fn new() -> Grabber {
        let ndi_name = CString::new("sckitndi").unwrap();
        let send_decr = ndi::NDIlib_send_create_t {
            p_ndi_name: ndi_name.as_ptr(),
            p_groups: null(),
            clock_video: false,
            clock_audio: false,
        };
        let ndi_instance = unsafe { ndi::NDIlib_send_create(&send_decr) };
        Self { ndi_instance }
    }

    fn start(&self) {
        let block = ConcreteBlock::new(move |shareable_content: id, _err: id| {
            CAPTURE_DELEGATE.get_or_init(|| {
                let mut decl = ClassDecl::new("GeneralCaptureDelegate", class!(NSObject)).unwrap();
                unsafe {
                    decl.add_method(
                        sel!(stream:didOutputSampleBuffer:ofType:),
                        capture_stream as extern "C" fn(&Object, _, id, id, NSInteger),
                    );
                }
                decl.register()
            });

            let displays: id = unsafe { msg_send![shareable_content, displays] };
            let display: id = unsafe { msg_send![displays, objectAtIndex:0] };
            let excluded: id = unsafe { msg_send![class!(NSArray), array] };
            let filter: id = unsafe {
                let filter: id = msg_send![class!(SCContentFilter), alloc];
                let _: () = msg_send![filter, initWithDisplay:display excludingWindows:excluded];
                filter
            };
            let stream_config: id = unsafe {
                let stream_config: id = msg_send![class!(SCStreamConfiguration), alloc];
                let stream_config: id = msg_send![stream_config, init];
                let source_rect = CGRect::new(
                    &CGPoint::new(1920. / 2., 1080. / 2.),
                    &CGSize::new(1920., 1080.),
                );
                let destination_rect =
                    CGRect::new(&CGPoint::new(0., 0.), &CGSize::new(1920., 1080.));
                let _: () = msg_send![stream_config, setWidth:1920];
                let _: () = msg_send![stream_config, setHeight:1080];
                let _: () = msg_send![stream_config, setSourceRect: source_rect];
                let _: () = msg_send![stream_config, setDestinationRect: destination_rect];
                let _: () = msg_send![stream_config, setQueueDepth:5];
                #[allow(non_upper_case_globals)]
                const kCVPixelFormatType_32BGRA: u32 = 1111970369;
                #[allow(non_upper_case_globals)]
                const kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange: u32 = 875704438;
                #[allow(non_upper_case_globals)]
                const kCVPixelFormatType_420YpCbCr8BiPlanarFullRange: u32 = 875704422;
                let _: () = msg_send![stream_config, setPixelFormat: kCVPixelFormatType_32BGRA];
                stream_config
            };
            let stream: id = unsafe {
                let stream: id = msg_send![class!(SCStream), alloc];
                let stream: id = msg_send![stream, init];
                let _: () = msg_send![stream, initWithFilter:filter configuration:stream_config delegate:null::<id>()];
                stream
            };
            let delegate: id = unsafe {
                let delegate: id = msg_send![class!(GeneralCaptureDelegate), alloc];
                msg_send![delegate, init]
            };
            let error: id = null_mut();
            let did_add_output: bool = unsafe {
                msg_send![stream, addStreamOutput:delegate type:0 sampleHandlerQueue:null::<id>() error:&error]
            };
            assert!(did_add_output);
            let block = ConcreteBlock::new(move |err: id| {
                assert!(err.is_null());
                println!("started");
            });
            let _: () = unsafe { msg_send![stream, startCaptureWithCompletionHandler: block] };
        });
        let block = block.copy();
        unsafe {
            let _: () = msg_send![
                class!(SCShareableContent),
                getShareableContentWithCompletionHandler: block
            ];
        }
    }
}

mod ffi {
    use std::ffi::c_void;

    use cacao::foundation::id;

    #[repr(C)]
    pub struct __CVBuffer(c_void);

    pub type CVBufferRef = *mut __CVBuffer;
    pub type CVImageBufferRef = CVBufferRef;
    pub type CVPixelBufferRef = CVImageBufferRef;

    pub type CVOptionFlags = u64;

    pub type CVReturn = i32;

    #[link(name = "CoreVideo", kind = "framework")]
    extern "C" {
        pub fn CVPixelBufferLockBaseAddress(
            pixelBuffer: CVPixelBufferRef,
            lockFlags: CVOptionFlags,
        ) -> CVReturn;
        pub fn CVPixelBufferUnlockBaseAddress(
            pixelBuffer: CVPixelBufferRef,
            unlockFlags: CVOptionFlags,
        ) -> CVReturn;
        pub fn CVPixelBufferGetBaseAddress(pixelBuffer: CVPixelBufferRef) -> *mut c_void;
        pub fn CVPixelBufferGetWidth(pixelBuffer: CVPixelBufferRef) -> usize;
        pub fn CVPixelBufferGetHeight(pixelBuffer: CVPixelBufferRef) -> usize;
    }

    #[link(name = "CoreMedia", kind = "framework")]
    extern "C" {
        pub fn CMSampleBufferGetImageBuffer(buffer: id) -> id;
    }
}

extern "C" fn capture_stream(
    _this: &Object,
    _: Sel,
    _stream: id,
    sample_buffer: id,
    _typ: NSInteger,
) {
    let pixel_buffer: id = unsafe { ffi::CMSampleBufferGetImageBuffer(sample_buffer) };
    let pixel_buffer = pixel_buffer as ffi::CVBufferRef;
    let width = unsafe { ffi::CVPixelBufferGetWidth(pixel_buffer) };
    let height = unsafe { ffi::CVPixelBufferGetHeight(pixel_buffer) };
    unsafe {
        ffi::CVPixelBufferLockBaseAddress(pixel_buffer, 1);
        let ptr = ffi::CVPixelBufferGetBaseAddress(pixel_buffer) as *mut u8;
        let video_data = ndi::NDIlib_video_frame_v2_t {
            xres: width as i32,
            yres: height as i32,
            FourCC: ndi::NDIlib_FourCC_video_type_e::NDIlib_FourCC_video_type_BGRA,
            frame_rate_N: 0,
            frame_rate_D: 0,
            picture_aspect_ratio: 0f32,
            frame_format_type:
                ndi::NDIlib_frame_format_type_e::NDIlib_frame_format_type_progressive,
            timecode: 0,
            p_data: ptr,
            __bindgen_anon_1: ndi::NDIlib_video_frame_v2_t__bindgen_ty_1 {
                line_stride_in_bytes: 0,
            },
            p_metadata: null(),
            timestamp: 0,
        };
        ndi::NDIlib_send_send_video_v2(GRABBER.ndi_instance, &video_data);
        ffi::CVPixelBufferUnlockBaseAddress(pixel_buffer, 1);
    }
}

fn main() {
    let mut config = WindowConfig::default();
    config.set_initial_dimensions(100., 100., 440., 400.);

    App::new(
        "com.koba789.sckitndi",
        SCKitNDI {
            window: Window::new(config),
            content: View::with(GrabberView::new()),
        },
    )
    .run();
}
