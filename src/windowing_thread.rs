use crate::{
    common::CthulockError,
    message::{UiMessage, WindowingMessage},
    Result,
};
use pam_client::{conv_mock::Conversation, Context, Flag};
use slint::{
    platform::{Key, PointerEventButton, WindowEvent},
    LogicalPosition, SharedString,
};
use smithay_client_toolkit::{
    delegate_keyboard, delegate_pointer, delegate_registry, delegate_seat,
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
};
use std::sync::mpsc::{Receiver, Sender};
use wayland_client::{
    delegate_noop,
    globals::registry_queue_init,
    protocol::{
        wl_buffer, wl_compositor, wl_display, wl_keyboard, wl_output, wl_pointer, wl_seat,
        wl_surface,
    },
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1, ext_session_lock_surface_v1, ext_session_lock_v1,
};

pub fn windowing_thread(
    sender: Sender<WindowingMessage>,
    receiver: Receiver<UiMessage>,
) -> Result<()> {
    let conn = Connection::connect_to_env()
        .map_err(|_| CthulockError::Generic("Failed to connect to wayland.".to_owned()))?;

    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let (globals, _queue) = registry_queue_init::<AppData>(&conn).unwrap();

    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 1..=5, ()).unwrap();
    let wl_surface = compositor.create_surface(&qh, ());
    let output: wl_output::WlOutput = globals.bind(&qh, 1..=1, ()).unwrap();
    let session_lock_manager: ext_session_lock_manager_v1::ExtSessionLockManagerV1 = globals.bind(&qh, 1..=1, ()).map_err(|_| {
        CthulockError::Generic("Could not bind ext-session-lock-v1. Your compositor probably does not support this.".to_owned())
    })?;
    let session_lock = session_lock_manager.lock(&qh, ());
    // set surface role as session lock surface
    session_lock.get_lock_surface(&wl_surface, &output, &qh, ());

    let mut state = AppData::new(
        RegistryState::new(&globals),
        display,
        wl_surface,
        session_lock,
        SeatState::new(&globals, &qh),
        sender,
    );

    while state.running {
        event_queue.blocking_dispatch(&mut state).unwrap();

        while let Ok(message) = receiver.try_recv() {
            match message {
                UiMessage::UnlockWithPassword { password } => {
                    let mut context = Context::new(
                        "cthulock",
                        None,
                        Conversation::with_credentials(whoami::username(), password),
                    )
                    .expect("Failed to initialize PAM context");

                    if context.authenticate(Flag::NONE).is_ok()
                        && context.acct_mgmt(Flag::NONE).is_ok()
                    {
                        log::info!("authentication successfull, quitting...");
                        state
                            .render_thread_sender
                            .send(WindowingMessage::Quit)
                            .unwrap();
                        state.session_lock.unlock_and_destroy();
                        event_queue.roundtrip(&mut state).unwrap();
                        state.running = false;
                    } else {
                        state
                            .render_thread_sender
                            .send(WindowingMessage::UnlockFailed)
                            .unwrap();
                    }
                }
            }
        }
    }
    Ok(())
}

// TODO: Support multiple outputs
// This struct represents the state of our app
struct AppData {
    running: bool,
    locked: bool,
    configured: bool,

    width: u32,
    height: u32,

    registry_state: RegistryState,
    wl_surface: wl_surface::WlSurface,
    wl_display: wl_display::WlDisplay,
    session_lock: ext_session_lock_v1::ExtSessionLockV1,

    seat_state: SeatState,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,

    render_thread_sender: Sender<WindowingMessage>,
}

impl AppData {
    fn new(
        registry_state: RegistryState,
        display: wl_display::WlDisplay,
        surface: wl_surface::WlSurface,
        session_lock: ext_session_lock_v1::ExtSessionLockV1,
        seat_state: SeatState,
        sender: Sender<WindowingMessage>,
    ) -> Self {
        Self {
            running: true,
            locked: false,
            configured: false,
            registry_state,
            wl_surface: surface,
            width: 0,
            height: 0,
            session_lock,
            wl_display: display,
            seat_state,
            keyboard: None,
            pointer: None,
            render_thread_sender: sender,
        }
    }
}

// Ignore events from these object types
delegate_noop!(AppData: ignore wl_compositor::WlCompositor);
delegate_noop!(AppData: ignore wl_surface::WlSurface);
delegate_noop!(AppData: ignore wl_buffer::WlBuffer);
delegate_noop!(AppData: ignore wl_output::WlOutput);
delegate_noop!(AppData: ignore ext_session_lock_manager_v1::ExtSessionLockManagerV1);
// Delegate input
delegate_seat!(AppData);
delegate_keyboard!(AppData);
delegate_pointer!(AppData);
delegate_registry!(AppData);

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![SeatState,];
}

impl Dispatch<ext_session_lock_v1::ExtSessionLockV1, ()> for AppData {
    fn event(
        state: &mut Self,
        _: &ext_session_lock_v1::ExtSessionLockV1,
        event: ext_session_lock_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_session_lock_v1::Event::Locked => {
                state.locked = true;
            }
            ext_session_lock_v1::Event::Finished => {
                state.running = false;
            }
            _ => {}
        };
    }
}

impl Dispatch<ext_session_lock_surface_v1::ExtSessionLockSurfaceV1, ()> for AppData {
    fn event(
        state: &mut Self,
        surface: &ext_session_lock_surface_v1::ExtSessionLockSurfaceV1,
        event: ext_session_lock_surface_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ext_session_lock_surface_v1::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            log::debug!("surface reconfigure serial: {serial}");

            state.width = width;
            state.height = height;

            let sender = &state.render_thread_sender;
            if !state.configured {
                sender
                    .send(WindowingMessage::SurfaceReady {
                        display_id: state.wl_display.id(),
                        surface_id: state.wl_surface.id(),
                        size: (width, height),
                    })
                    .unwrap();
                state.configured = true;
                surface.ack_configure(serial);
            }
        }
    }
}

impl SeatHandler for AppData {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            log::debug!("got keyboard capability");

            let keyboard = self
                .seat_state
                .get_keyboard(qh, &seat, None)
                .expect("Failed to create keyboard");

            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.pointer.is_none() {
            log::debug!("got pointer capability");

            let pointer = self
                .seat_state
                .get_pointer(qh, &seat)
                .expect("Failed to create pointer");
            self.pointer = Some(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_some() {
            log::debug!("unset keyboard capability");
            self.keyboard.take().unwrap().release();
        }

        if capability == Capability::Pointer && self.pointer.is_some() {
            log::debug!("unset pointer capability");
            self.pointer.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for AppData {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _keysyms: &[Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        if let Some(text) = sctk_key_event_to_slint(event) {
            self.render_thread_sender
                .send(WindowingMessage::SlintWindowEvent(
                    WindowEvent::KeyPressed { text },
                ))
                .unwrap();
        }
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        if let Some(text) = sctk_key_event_to_slint(event) {
            self.render_thread_sender
                .send(WindowingMessage::SlintWindowEvent(
                    WindowEvent::KeyReleased { text },
                ))
                .unwrap();
        }
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _: Modifiers,
    ) {
    }
}

fn sctk_key_event_to_slint(event: KeyEvent) -> Option<SharedString> {
    match event.keysym {
        Keysym::BackSpace => Some(Key::Backspace.into()),
        Keysym::Tab => Some(Key::Tab.into()),
        Keysym::Return => Some(Key::Return.into()),
        Keysym::Delete => Some(Key::Delete.into()),
        Keysym::Shift_L | Keysym::Shift_R => Some(Key::Shift.into()),
        Keysym::Control_L | Keysym::Control_R => Some(Key::Control.into()),
        Keysym::Alt_L | Keysym::Alt_R => Some(Key::Alt.into()),
        Keysym::Caps_Lock => Some(Key::CapsLock.into()),
        Keysym::Up => Some(Key::UpArrow.into()),
        Keysym::Down => Some(Key::DownArrow.into()),
        Keysym::Left => Some(Key::LeftArrow.into()),
        Keysym::Right => Some(Key::RightArrow.into()),
        Keysym::Insert => Some(Key::Insert.into()),
        Keysym::Home => Some(Key::Home.into()),
        Keysym::End => Some(Key::End.into()),
        _ => event.utf8.map(String::into),
    }
}

impl PointerHandler for AppData {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        use PointerEventKind::*;
        for event in events {
            let position = LogicalPosition::new(event.position.0 as f32, event.position.1 as f32);

            match event.kind {
                Enter { .. } => {}
                Leave { .. } => {
                    self.render_thread_sender
                        .send(WindowingMessage::SlintWindowEvent(
                            WindowEvent::PointerExited,
                        ))
                        .unwrap();
                }
                Motion { .. } => {
                    self.render_thread_sender
                        .send(WindowingMessage::SlintWindowEvent(
                            WindowEvent::PointerMoved { position },
                        ))
                        .unwrap();
                }
                Press { button, .. } => {
                    self.render_thread_sender
                        .send(WindowingMessage::SlintWindowEvent(
                            WindowEvent::PointerPressed {
                                position,
                                button: wl_pointer_button_to_slint(button),
                            },
                        ))
                        .unwrap();
                }
                Release { button, .. } => {
                    self.render_thread_sender
                        .send(WindowingMessage::SlintWindowEvent(
                            WindowEvent::PointerReleased {
                                position,
                                button: wl_pointer_button_to_slint(button),
                            },
                        ))
                        .unwrap();
                }
                Axis {
                    horizontal,
                    vertical,
                    ..
                } => {
                    self.render_thread_sender
                        .send(WindowingMessage::SlintWindowEvent(
                            WindowEvent::PointerScrolled {
                                position,
                                delta_x: horizontal.absolute as f32,
                                delta_y: vertical.absolute as f32,
                            },
                        ))
                        .unwrap();
                }
            }
        }
    }
}

fn wl_pointer_button_to_slint(button: u32) -> PointerEventButton {
    match button {
        272 => PointerEventButton::Left,
        273 => PointerEventButton::Right,
        274 => PointerEventButton::Middle,
        _ => PointerEventButton::Other,
    }
}
