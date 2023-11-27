use std::sync::mpsc::Receiver;

use slint::{
    platform::femtovg_renderer::FemtoVGRenderer,
    PhysicalSize,
};
use crate::{
    window_adapter::MinimalFemtoVGWindow,
    platform::CthuluSlintPlatform,
    message::CthulockMessage,
    egl::OpenGLContext,
};

slint::slint!{
    import { Button } from "std-widgets.slint";
    export component HelloWorld {
        Button {
            text: "hello world";
            clicked => { self.text = "yekyekyek"; }
        }
    }
}

pub fn render_thread(receiver: Receiver<CthulockMessage>) {
    let (display_id, surface_id, size) = match receiver.recv().unwrap() {
        CthulockMessage::SurfaceReady{ display_id, surface_id, size} => (display_id, surface_id, size),
        message => panic!("First message sent to render thread is not ContextCreated. Is {:?}", message),
    };

    let context = OpenGLContext::new(display_id, surface_id, size);
    let renderer = FemtoVGRenderer::new(context).unwrap();
    let slint_window = MinimalFemtoVGWindow::new(renderer);
    slint_window.set_size(slint::WindowSize::Physical(PhysicalSize::new(size.0, size.1)));

    let platform = CthuluSlintPlatform::new(slint_window.clone());

    slint::platform::set_platform(Box::new(platform)).unwrap();
    let _ui = HelloWorld::new().expect("Failed to load UI").show();

    let running = true;
    while running {
        // handle messages
        while let Ok(_message) = receiver.try_recv() {
            
        }

        slint::platform::update_timers_and_animations();
        slint_window.draw_if_needed();
    }

}
