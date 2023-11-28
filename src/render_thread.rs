use std::sync::mpsc::{
    Sender,
    Receiver,
};
use slint::{
    platform::{
        femtovg_renderer::FemtoVGRenderer,
        WindowEvent,
    },
    PhysicalSize,
    LogicalSize,
};
use crate::{
    window_adapter::MinimalFemtoVGWindow,
    platform::CthuluSlintPlatform,
    message::{
        WindowingMessage,
        RenderMessage,
    },
    egl::OpenGLContext,
};

slint::slint!{
    // import { Image } from "std-widgets.slint";
    export component HelloWorld {
        in property<string> clock_text;
        Image {
            // image-fit: fill;
            width: parent.width;
            height: parent.height;
            source: @image-url("/home/fried/.config/wallpaper.png");
            Text {
                text: clock_text;
            }
        }
    }
}

pub fn render_thread(sender: Sender<RenderMessage>, receiver: Receiver<WindowingMessage>) {
    let (display_id, surface_id, size) = match receiver.recv().unwrap() {
        WindowingMessage::SurfaceReady{ display_id, surface_id, size} => (display_id, surface_id, size),
        message => panic!("First message sent to render thread is not ContextCreated. Is {:?}", message),
    };

    let context = OpenGLContext::new(display_id, surface_id, size);
    let renderer = FemtoVGRenderer::new(context).unwrap();
    let slint_window = MinimalFemtoVGWindow::new(renderer);
    slint_window.set_size(slint::WindowSize::Physical(PhysicalSize::new(size.0, size.1)));

    let platform = CthuluSlintPlatform::new(slint_window.clone());

    slint::platform::set_platform(Box::new(platform)).unwrap();
    let ui = HelloWorld::new().expect("Failed to load UI");
    ui.set_clock_text("jeek".into());
    ui.show().unwrap();

    let running = true;
    let mut last_serial = -1;
    let mut last_acked_serial = -1;
    while running {
        // handle messages
        while let Ok(message) = receiver.try_recv() {
            match message {
                WindowingMessage::SlintWindowEvent(event) => slint_window.dispatch_event(event),
                WindowingMessage::SurfaceResize { size, serial } => {
                    slint_window.dispatch_event(
                        WindowEvent::Resized { 
                            size: LogicalSize::new(size.0 as f32, size.1 as f32)
                        }
                    );
                    sender.send(
                        RenderMessage::AckResize { serial }
                    ).unwrap();
                    last_serial = serial as i64;
                },
                WindowingMessage::SurfaceResizeAcked { serial } => {
                    last_acked_serial = serial as i64;
                },
                WindowingMessage::SurfaceReady { .. } => panic!("surface already configured"),
            }
        }
        
        slint::platform::update_timers_and_animations();
        if last_serial == last_acked_serial {
            slint_window.draw_if_needed();
        }
    }

}
