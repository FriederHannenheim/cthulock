use chrono::Local;
use std::{
    sync::mpsc::{Receiver, Sender},
    time::Duration, rc::Rc,
};
use crate::{
    message::{UiMessage, WindowingMessage},
    ui:: {
        egl::OpenGLContext,
        platform::CthulockSlintPlatform,
        window_adapter::MinimalFemtoVGWindow
    }, Result, common::CthulockError,
};
use slint::{
    platform::{femtovg_renderer::FemtoVGRenderer, WindowEvent},
    LogicalSize, PhysicalSize,
};
use slint_interpreter::{
    Value,
    SharedString,
    ComponentHandle, ComponentInstance, ComponentDefinition
};

mod egl;
mod platform;
mod window_adapter;

pub fn ui_thread(style: ComponentDefinition, sender: Sender<UiMessage>, receiver: Receiver<WindowingMessage>) -> Result<()>{
    let slint_window = wait_for_configure_and_set_platform(&receiver)?;
    
    let ui = create_ui(sender.clone(), style)?;
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
                    slint_window.dispatch_event(WindowEvent::Resized {
                        size: LogicalSize::new(size.0 as f32, size.1 as f32),
                    });
                    sender.send(UiMessage::AckResize { serial }).unwrap();
                    last_serial = serial as i64;
                }
                WindowingMessage::SurfaceResizeAcked { serial } => {
                    last_acked_serial = serial as i64;
                }
                WindowingMessage::UnlockFailed =>  {
                    ui.set_property("checking_password", false.into()).map_err(|_| {
                        CthulockError::property_fail("checking_password")
                    })?;
                    ui.set_property("password", SharedString::from("").into()).map_err(|_| {
                        CthulockError::property_fail("password")
                    })?;
                },
                WindowingMessage::Quit => {
                    log::info!("quitting UI thread...");
                    return Ok(());
                },
                WindowingMessage::SurfaceReady { .. } => panic!("surface already configured"),
            }
        }
        let time = Local::now();
        let _ = ui.set_property("clock_text", SharedString::from(time.format("%H:%M").to_string()).into());

        if last_serial == last_acked_serial {
            slint_window.draw_if_needed();
        }

        if !slint_window.has_active_animations() {
            let duration = slint::platform::duration_until_next_timer_update()
                .map_or(Duration::from_millis(8), |d| {
                    d.min(Duration::from_millis(8))
                });
            std::thread::sleep(duration);
        }
    }

    Ok(())
}


fn create_ui(sender: Sender<UiMessage>, style: ComponentDefinition) -> Result<ComponentInstance> {
    let ui = style.create().unwrap();

    let sender_clone = sender.clone();
    let ui_ref = ui.as_weak();
    ui.set_callback("submit", move |args: &[Value]| -> Value {
        let ui = ui_ref.upgrade().unwrap();
        let Value::String(password) = args[0].clone() else {
            panic!("Value in submit callback is not a String");
        };

        ui.set_property("checking_password", true.into()).map_err(|_| {
            CthulockError::property_fail("checking_password")
        }).unwrap();
        sender_clone
            .send(UiMessage::UnlockWithPassword {
                password: password.to_string(),
            })
            .unwrap();
        Value::Void
    }).map_err(|_| {
        CthulockError::callback_bind_fail("submit")
    })?;

    Ok(ui)
}

fn wait_for_configure_and_set_platform(receiver: &Receiver<WindowingMessage>) -> Result<Rc<MinimalFemtoVGWindow>> {
    let (display_id, surface_id, size) = match receiver.recv().unwrap() {
        WindowingMessage::SurfaceReady {
            display_id,
            surface_id,
            size,
        } => (display_id, surface_id, size),
        message => panic!(
            "First message sent to render thread is not SurfaceReady. Is {:?}",
            message
        ),
    };

    let context = OpenGLContext::new(display_id, surface_id, size);
    let renderer = FemtoVGRenderer::new(context).unwrap();
    let slint_window = MinimalFemtoVGWindow::new(renderer);
    slint_window.set_size(slint::WindowSize::Physical(PhysicalSize::new(
        size.0, size.1,
    )));

    let platform = CthulockSlintPlatform::new(slint_window.clone());
    slint::platform::set_platform(Box::new(platform)).unwrap();

    Ok(slint_window)
}