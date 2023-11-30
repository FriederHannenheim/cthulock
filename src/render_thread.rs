use std::{sync::{mpsc::{
    Sender,
    Receiver,
}, Mutex, Arc}, thread, time::Duration};
use chrono::Local;
use image::RgbImage;
use slint::{
    platform::{
        femtovg_renderer::FemtoVGRenderer,
        WindowEvent,
    },
    PhysicalSize,
    LogicalSize, SharedPixelBuffer,
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
    import { LineEdit , TextEdit} from "std-widgets.slint";
    export component HelloWorld {
        in property<string> clock_text;
        in property<bool> checking_password;
        callback submit <=> password.accepted;
        forward-focus: password;
        states [
            checking when checking-password : {
                password.enabled: false;
            }
        ]

        Image {
            width: parent.width;
            height: parent.height;
            source: @image-url("/home/fried/.config/wallpaper.png");
            HorizontalLayout {
                VerticalLayout {
                    alignment: end;
                    spacing: 10px;
                    padding: 40px;
                    width: 350px;
                    Text {
                        text: clock_text;
                        horizontal-alignment: center;
                        font-size: 60pt;
                        color: white;
                    }
                    password := LineEdit {
                        enabled: true;
                        horizontal-alignment: left;
                        input-type: InputType.password;
                        placeholder-text: "password...";
                    }
                }
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

    let sender_clone = sender.clone();
    let ui_ref = ui.as_weak();
    ui.on_submit(move |pw| {
        let ui = ui_ref.upgrade().unwrap();
        ui.set_checking_password(true);
        sender_clone.send(RenderMessage::UnlockWithPassword { password: pw.to_string() }).unwrap();
    });
    ui.show().unwrap();

    let running = true;
    let mut last_serial = -1;
    let mut last_acked_serial = -1;
    while running {
        slint::platform::update_timers_and_animations();

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
                WindowingMessage::UnlockFailed => ui.set_checking_password(false),
                WindowingMessage::SurfaceReady { .. } => panic!("surface already configured"),
            }
        }
        let time = Local::now();
        ui.set_clock_text(time.format("%H:%M").to_string().into());

        if last_serial == last_acked_serial {
            slint_window.draw_if_needed();
        }

        if !slint_window.has_active_animations() {
            let duration = slint::platform::duration_until_next_timer_update()
                                        .map_or(Duration::from_millis(8), |d| d.min(Duration::from_millis(8)));
            std::thread::sleep(duration);
        }
    }

}
