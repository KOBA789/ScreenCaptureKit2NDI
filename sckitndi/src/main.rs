use std::sync::Arc;

use cacao::{
    appkit::{
        window::{Window, WindowConfig},
        App, AppDelegate,
    },
    button::Button,
    control::Control,
    layout::{Layout, LayoutConstraint},
    notification_center::Dispatcher,
    view::{View, ViewDelegate},
};
use grabber::Grabber;

mod grabber;
mod ndi;

struct SCKitNDI {
    window: Window,
    content: View<GrabberView>,
    grabber: Arc<Grabber>,
}

impl AppDelegate for SCKitNDI {
    fn did_finish_launching(&self) {
        self.window.show();
    }
}

#[derive(Debug)]
enum Action {
    Start,
    GetShareableContent,
}

impl Action {
    pub fn dispatch_main(self) {
        App::<SCKitNDI, Self>::dispatch_main(self);
    }
}

impl Dispatcher for SCKitNDI {
    type Message = Action;

    fn on_ui_message(&self, message: Self::Message) {
        match message {
            Action::Start => {
                self.content
                    .delegate
                    .as_ref()
                    .unwrap()
                    .start
                    .set_enabled(false);
                self.grabber.start();
            }
            Action::GetShareableContent => {
                sckit::ShareableContent::get(|ret| {
                    let shareable_content = ret.unwrap();
                    let windows = shareable_content.windows();
                    for w in &windows {
                        let app = w.owning_application();
                        let bundle_id = app
                            .bundle_identifier()
                            .unwrap_or_else(|| "UNKNOWN".to_string());
                        let app_name = app
                            .application_name()
                            .unwrap_or_else(|| "UNKNOWN".to_string());
                        println!(
                            "[{}]{} *{}: #{} {}",
                            bundle_id,
                            app_name,
                            app.process_id(),
                            w.window_id(),
                            w.title()
                        );
                    }
                    let apps = shareable_content.applications();
                    for app in &apps {
                        let bundle_id = app
                            .bundle_identifier()
                            .unwrap_or_else(|| "UNKNOWN".to_string());
                        let app_name = app
                            .application_name()
                            .unwrap_or_else(|| "UNKNOWN".to_string());
                        println!("[{}]{} *{}", bundle_id, app_name, app.process_id());
                    }
                });
            }
        }
    }
}

struct GrabberView {
    start: Button,
    get_shareable_contents: Button,
}

impl GrabberView {
    fn new() -> Self {
        let mut start = Button::new("Start");
        start.set_action(|| {
            Action::Start.dispatch_main();
        });
        let mut get_shareable_contents = Button::new("Get Shareable Contents");
        get_shareable_contents.set_action(|| {
            Action::GetShareableContent.dispatch_main();
        });

        Self {
            start,
            get_shareable_contents,
        }
    }
}

impl ViewDelegate for GrabberView {
    const NAME: &'static str = stringify!(GrabberView);

    fn did_load(&mut self, view: View) {
        view.add_subview(&self.start);
        view.add_subview(&self.get_shareable_contents);

        LayoutConstraint::activate(&[
            self.start.top.constraint_equal_to(&view.top).offset(36.),
            self.get_shareable_contents
                .top
                .constraint_equal_to(&view.top)
                .offset(72.),
        ]);
    }
}

fn main() {
    let content = View::with(GrabberView::new());

    let mut config = WindowConfig::default();
    config.set_initial_dimensions(100., 100., 440., 400.);
    let window = Window::new(config);
    window.set_minimum_content_size(400., 400.);
    window.set_title("ScreenCaptureKit2NDI");
    window.set_content_view(&content);

    let grabber = Arc::new(Grabber::new());

    App::new(
        "com.koba789.sckitndi",
        SCKitNDI {
            window,
            content,
            grabber,
        },
    )
    .run();
}
