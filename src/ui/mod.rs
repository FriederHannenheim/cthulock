use crate::{
    common::CthulockError,
    message::{UiMessage, WindowingMessage},
    ui::{
        egl::OpenGLContext,
        platform::CthulockSlintPlatform,
        slint_types::{OptionalProperties, RequiredProperties},
        window_adapter::MinimalFemtoVGWindow,
    },
    Result,
};
use chrono::Local;
use slint::{platform::femtovg_renderer::FemtoVGRenderer, PhysicalSize};
use slint_interpreter::{
    ComponentDefinition, ComponentHandle, ComponentInstance, SharedString, Value,
};
use std::{
    rc::Rc,
    sync::mpsc::{Receiver, Sender, TryRecvError},
    time::Duration,
};

use self::slint_types::RequiredCallbacks;

mod egl;
mod platform;
pub(crate) mod slint_types;
mod window_adapter;

pub fn ui_thread(
    style: ComponentDefinition,
    sender: Sender<UiMessage>,
    receiver: Receiver<WindowingMessage>,
) -> Result<()> {
    let slint_window = wait_for_configure_and_set_platform(&receiver)?;

    let ui = create_ui(sender.clone(), style)?;
    ui.show().unwrap();

    loop {
        slint::platform::update_timers_and_animations();

        if receive_messages(&receiver, Rc::clone(&slint_window), &ui).is_err() {
            return Ok(());
        }

        let time = Local::now();
        let _ = ui.set_property(
            &OptionalProperties::ClockText,
            SharedString::from(time.format("%H:%M").to_string()).into(),
        );

        slint_window.draw_if_needed();

        if !slint_window.has_active_animations() {
            let duration = slint::platform::duration_until_next_timer_update()
                .map_or(Duration::from_millis(8), |d| {
                    d.min(Duration::from_millis(8))
                });
            std::thread::sleep(duration);
        }
    }
}

fn handle_message(
    message: WindowingMessage,
    slint_window: Rc<MinimalFemtoVGWindow>,
    ui: &ComponentInstance
) -> Result<()> {
    match message {
        WindowingMessage::SlintWindowEvent(event) => slint_window.dispatch_event(event),
        WindowingMessage::UnlockFailed => {
            let _ = ui.set_property(&OptionalProperties::CheckingPassword, false.into());
            let _ =
                ui.set_property(&RequiredProperties::Password, SharedString::from("").into());
        }
        WindowingMessage::Quit => {
            log::info!("quitting UI thread...");
            return Err(CthulockError::WindowingThreadQuit);
        }
        WindowingMessage::SurfaceReady { .. } => panic!("surface already configured"),
    }
    Ok(())
}

fn receive_messages(
    receiver: &Receiver<WindowingMessage>,
    slint_window: Rc<MinimalFemtoVGWindow>,
    ui: &ComponentInstance,
) -> Result<()> {
    loop {
        let message = receiver.try_recv();
        match message {
            Ok(message) => {
                handle_message(message, slint_window.clone(), ui)?;
            },
            Err(TryRecvError::Empty) => {
                return Ok(())
            }
            Err(TryRecvError::Disconnected) => {
                return Err(CthulockError::WindowingThreadQuit);
            }
        }
    }
}

fn create_ui(sender: Sender<UiMessage>, style: ComponentDefinition) -> Result<ComponentInstance> {
    let ui = style.create().unwrap();

    let sender_clone = sender.clone();
    let ui_ref = ui.as_weak();
    ui.set_callback(&RequiredCallbacks::Submit, move |args: &[Value]| -> Value {
        let ui = ui_ref.upgrade().unwrap();
        let Value::String(password) = args[0].clone() else {
            panic!("Value in submit callback is not a String");
        };

        let _ = ui.set_property(&OptionalProperties::CheckingPassword, true.into());
        sender_clone
            .send(UiMessage::UnlockWithPassword {
                password: password.to_string(),
            })
            .unwrap();
        Value::Void
    })
    .unwrap();

    Ok(ui)
}

fn wait_for_configure_and_set_platform(
    receiver: &Receiver<WindowingMessage>,
) -> Result<Rc<MinimalFemtoVGWindow>> {
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

