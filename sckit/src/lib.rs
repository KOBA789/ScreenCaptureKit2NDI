#![allow(clippy::let_unit_value)]

#[macro_use]
extern crate objc;

use std::{
    ffi::c_void,
    fmt::Debug,
    ptr::{null, null_mut, NonNull},
    sync::Arc,
};

use anyhow::Result;
use block::ConcreteBlock;
use cocoa_foundation::{
    base::{id, nil},
    foundation::{NSArray, NSInteger, NSString},
};
use core_graphics_types::geometry::CGRect;
use objc::{
    declare::ClassDecl,
    rc::StrongPtr,
    runtime::{Class, Object, Sel},
};
use once_cell::sync::Lazy;

use framework_sys as fw_sys;

static STREAM_OUTPUT_DELEGATE: Lazy<&'static Class> = Lazy::new(|| {
    let mut decl = ClassDecl::new("StreamOutputDelegate", class!(NSObject)).unwrap();
    decl.add_ivar::<*const c_void>("_inner");
    unsafe {
        decl.add_method(sel!(setInner:), set_inner as extern "C" fn(&mut _, _, _));
        decl.add_method(
            sel!(stream:didOutputSampleBuffer:ofType:),
            did_output_sample_buffer_of_type as extern "C" fn(&_, _, _, _, _),
        );
    }
    decl.register()
});

extern "C" fn set_inner(this: &mut Object, _: Sel, inner_ptr: *mut c_void) {
    unsafe {
        this.set_ivar("_inner", inner_ptr);
    }
}

extern "C" fn did_output_sample_buffer_of_type(
    this: &Object,
    _: Sel,
    stream: id,
    sample_buffer: id,
    type_: NSInteger,
) {
    let sample_buffer = sample_buffer as fw_sys::CMSampleBufferRef;
    unsafe {
        let stream = Stream(StrongPtr::retain(stream));
        let inner_ptr = *this.get_ivar::<*mut c_void>("_inner") as *mut Arc<dyn StreamOutput>;
        let boxed_inner = Box::from_raw(inner_ptr);
        boxed_inner.did_output_sample_buffer_of_type(stream, sample_buffer, type_);
        // forget
        let _ = Box::into_raw(boxed_inner);
    }
}

pub trait StreamOutput {
    fn did_output_sample_buffer_of_type(
        &self,
        stream: Stream,
        sample_buffer: fw_sys::CMSampleBufferRef,
        type_: NSInteger,
    );
}

pub struct StreamConfig(StrongPtr);

impl StreamConfig {
    pub fn width(&self) -> usize {
        unsafe { msg_send![*self.0, width] }
    }
    pub fn set_width(&mut self, width: usize) {
        unsafe { msg_send![*self.0, setWidth: width] }
    }

    pub fn height(&self) -> usize {
        unsafe { msg_send![*self.0, height] }
    }
    pub fn set_height(&mut self, height: usize) {
        unsafe { msg_send![*self.0, setHeight: height] }
    }

    pub fn source_rect(&self) -> CGRect {
        unsafe { msg_send![*self.0, sourceRect] }
    }
    pub fn set_source_rect(&mut self, source_rect: CGRect) {
        unsafe { msg_send![*self.0, setSourceRect: source_rect] }
    }

    pub fn destination_rect(&self) -> CGRect {
        unsafe { msg_send![*self.0, destinationRect] }
    }
    pub fn set_destination_rect(&mut self, destination_rect: CGRect) {
        unsafe { msg_send![*self.0, setDestinationRect: destination_rect] }
    }

    pub fn queue_depth(&self) -> NSInteger {
        unsafe { msg_send![*self.0, queueDepth] }
    }
    pub fn set_queue_depth(&self, queue_depth: NSInteger) {
        unsafe { msg_send![*self.0, setQueueDepth: queue_depth] }
    }

    pub fn minimum_frame_interval(&self) -> fw_sys::CMTime {
        unsafe { msg_send![*self.0, minimumFrameInterval] }
    }
    pub fn set_minimum_frame_interval(&mut self, minimum_frame_interval: fw_sys::CMTime) {
        unsafe { msg_send![*self.0, setMinimumFrameInterval: minimum_frame_interval] }
    }
}

impl Default for StreamConfig {
    fn default() -> Self {
        let stream_config = unsafe {
            let stream_config: id = msg_send![class!(SCStreamConfiguration), alloc];
            let stream_config = StrongPtr::new(msg_send![stream_config, init]);
            let _: () = msg_send![
                *stream_config,
                setPixelFormat: fw_sys::kCVPixelFormatType_32BGRA
            ];
            let _: () = msg_send![*stream_config, setColorSpaceName: fw_sys::kCGColorSpaceSRGB];
            stream_config
        };
        Self(stream_config)
    }
}

pub struct NSArrayIter {
    ns_array: id,
    pos: u64,
}
impl NSArrayIter {
    fn new(ns_array: id) -> Self {
        Self { ns_array, pos: 0 }
    }
}
impl Iterator for NSArrayIter {
    type Item = id;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < unsafe { self.ns_array.count() } {
            let pos = self.pos;
            self.pos += 1;
            Some(unsafe { self.ns_array.objectAtIndex(pos) })
        } else {
            None
        }
    }
}

fn to_rust_string(ns_string: id) -> String {
    let s = unsafe {
        let len = ns_string.len();
        let bytes = std::slice::from_raw_parts(ns_string.UTF8String() as *const u8, len);
        std::str::from_utf8_unchecked(bytes)
    };
    s.to_string()
}

pub struct ShareableContent(StrongPtr);
unsafe impl Send for ShareableContent {}
unsafe impl Sync for ShareableContent {}
impl ShareableContent {
    pub fn get(callback: impl Fn(Result<ShareableContent>) + 'static) {
        let callback = std::sync::Mutex::new(Some(callback));
        let block = ConcreteBlock::new(move |shareable_content: id, err: id| {
            if let Some(callback) = callback.lock().unwrap().take() {
                if err.is_null() {
                    callback(Ok(unsafe { Self::retain(shareable_content) }));
                } else {
                    callback(Err(anyhow::anyhow!("Failed to get ShareableContent")));
                }
            }
        });
        let block = block.copy();
        unsafe {
            let _: () = msg_send![
                class!(SCShareableContent),
                getShareableContentExcludingDesktopWindows:false onScreenWindowsOnly:true completionHandler:block
            ];
        }
    }

    pub unsafe fn retain(shareable_content: id) -> Self {
        Self(StrongPtr::retain(shareable_content))
    }

    pub fn displays(&self) -> Vec<Display> {
        let displays: id = unsafe { msg_send![*self.0, displays] };
        NSArrayIter::new(displays)
            .map(|d| Display(unsafe { StrongPtr::retain(d) }))
            .collect()
    }

    pub fn windows(&self) -> Vec<Window> {
        let windows: id = unsafe { msg_send![*self.0, windows] };
        NSArrayIter::new(windows)
            .map(|w| Window(unsafe { StrongPtr::retain(w) }))
            .collect()
    }

    pub fn applications(&self) -> Vec<RunningApplication> {
        let applications: id = unsafe { msg_send![*self.0, applications] };
        NSArrayIter::new(applications)
            .map(|a| RunningApplication(unsafe { StrongPtr::retain(a) }))
            .collect()
    }
}
impl Debug for ShareableContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ShareableContent").field(&*self.0).finish()
    }
}

#[derive(Clone)]
pub struct Display(StrongPtr);

#[derive(Clone)]
pub struct Window(StrongPtr);

impl Window {
    pub fn window_id(&self) -> u32 {
        unsafe { msg_send![*self.0, windowID] }
    }

    pub fn title(&self) -> String {
        let title: id = unsafe { msg_send![*self.0, title] };
        to_rust_string(title)
    }

    pub fn owning_application(&self) -> RunningApplication {
        let application = unsafe { msg_send![*self.0, owningApplication] };
        RunningApplication(unsafe { StrongPtr::retain(application) })
    }
}

#[derive(Clone)]
pub struct RunningApplication(StrongPtr);

impl RunningApplication {
    pub fn process_id(&self) -> u32 {
        unsafe { msg_send![*self.0, processID] }
    }

    pub fn bundle_identifier(&self) -> Option<String> {
        let bundle_identifier: id = unsafe { msg_send![*self.0, bundleIdentifier] };
        NonNull::new(bundle_identifier).map(|non_null| to_rust_string(non_null.as_ptr()))
    }

    pub fn application_name(&self) -> Option<String> {
        let application_name = unsafe { msg_send![*self.0, applicationName] };
        NonNull::new(application_name).map(|non_null| to_rust_string(non_null.as_ptr()))
    }
}

fn windows_to_nsarray<'a>(into_iter: impl IntoIterator<Item = &'a Window>) -> id {
    let windows: Vec<id> = into_iter.into_iter().map(|w| *w.0).collect();
    unsafe { NSArray::arrayWithObjects(nil, &windows) }
}

fn apps_to_nsarray<'a>(into_iter: impl IntoIterator<Item = &'a RunningApplication>) -> id {
    let apps: Vec<id> = into_iter.into_iter().map(|a| *a.0).collect();
    unsafe { NSArray::arrayWithObjects(nil, &apps) }
}

pub struct ContentFilter(StrongPtr);
impl ContentFilter {
    pub fn with_desktop_independent_window(window: &Window) -> Self {
        let filter = unsafe {
            let filter: id = msg_send![class!(SCContentFilter), alloc];
            StrongPtr::new(msg_send![filter, initWithDesktopIndependentWindow:*window.0])
        };
        Self(filter)
    }

    pub fn init_with_display_including_windows<'a>(
        display: &'a Display,
        excluding_windows: impl IntoIterator<Item = &'a Window>,
    ) -> Self {
        let filter = unsafe {
            let filter: id = msg_send![class!(SCContentFilter), alloc];
            StrongPtr::new(msg_send![
                filter,
                initWithDisplay:*display.0
                includingWindows:windows_to_nsarray(excluding_windows)
            ])
        };
        Self(filter)
    }

    pub fn init_with_display_excluding_windows<'a>(
        display: &'a Display,
        excluding_windows: impl IntoIterator<Item = &'a Window>,
    ) -> Self {
        let filter = unsafe {
            let filter: id = msg_send![class!(SCContentFilter), alloc];
            StrongPtr::new(msg_send![
                filter,
                initWithDisplay:*display.0
                excludingWindows:windows_to_nsarray(excluding_windows)
            ])
        };
        Self(filter)
    }

    pub fn init_with_display_including_applications_excepting_windows<'a>(
        display: &'a Display,
        including_applications: impl IntoIterator<Item = &'a RunningApplication>,
        excepting_windows: impl IntoIterator<Item = &'a Window>,
    ) -> Self {
        let filter = unsafe {
            let filter: id = msg_send![class!(SCContentFilter), alloc];
            StrongPtr::new(msg_send![
                filter,
                initWithDisplay:*display.0
                includingApplications:apps_to_nsarray(including_applications)
                exceptingWindows:windows_to_nsarray(excepting_windows)
            ])
        };
        Self(filter)
    }

    pub fn init_with_display_excluding_applications_excepting_windows<'a>(
        display: &'a Display,
        excluding_applications: impl IntoIterator<Item = &'a RunningApplication>,
        excepting_windows: impl IntoIterator<Item = &'a Window>,
    ) -> Self {
        let filter = unsafe {
            let filter: id = msg_send![class!(SCContentFilter), alloc];
            StrongPtr::new(msg_send![
                filter,
                initWithDisplay:*display.0
                excludingApplications:apps_to_nsarray(excluding_applications)
                exceptingWindows:windows_to_nsarray(excepting_windows)
            ])
        };
        Self(filter)
    }
}

#[derive(Clone)]
pub struct Stream(StrongPtr);

impl Stream {
    pub fn new(filter: ContentFilter, config: StreamConfig) -> Self {
        let stream = unsafe {
            let stream: id = msg_send![class!(SCStream), alloc];
            let stream = StrongPtr::new(msg_send![stream, init]);
            let _: () = msg_send![*stream, initWithFilter:filter.0 configuration:config.0 delegate:null::<id>()];
            stream
        };
        Self(stream)
    }

    pub fn start_capture(&self, callback: impl Fn(Result<()>) + 'static) {
        let block = ConcreteBlock::new(move |err: id| {
            if err.is_null() {
                callback(Ok(()));
            } else {
                callback(Err(anyhow::anyhow!("Failed to start capture")));
            }
        });
        let _: () = unsafe { msg_send![*self.0, startCaptureWithCompletionHandler: block] };
    }

    pub fn add_stream_output(
        &self,
        stream_output: Arc<dyn StreamOutput>,
        type_: NSInteger,
    ) -> bool {
        let stream_output = Box::new(stream_output);
        let delegate = unsafe {
            let delegate: id = msg_send![*STREAM_OUTPUT_DELEGATE, alloc];
            let delegate: id = msg_send![delegate, init];
            let inner_ptr = Box::into_raw(stream_output) as *const c_void;
            let _: () = msg_send![delegate, setInner: inner_ptr];
            StrongPtr::new(delegate)
        };
        let error: id = null_mut();
        unsafe {
            msg_send![*self.0, addStreamOutput:delegate type:type_ sampleHandlerQueue:null::<id>() error:&error]
        }
    }
}
