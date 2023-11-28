use std::{
    sync::mpsc::{Sender, Receiver},
    time::Duration,
};
use wayland_client::{
    protocol::{
        wl_buffer, wl_compositor, wl_keyboard, wl_seat,
        wl_surface, wl_output, wl_callback, wl_display, wl_pointer,
    },
    globals::{
        registry_queue_init,
    },
    Proxy, Connection, Dispatch, QueueHandle,
    delegate_noop,
};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1,
    ext_session_lock_v1,
    ext_session_lock_surface_v1,
};
use smithay_client_toolkit::{
    delegate_keyboard, delegate_pointer,
    delegate_seat, delegate_registry, registry_handlers,
    registry::{
        RegistryState, ProvidesRegistryState,
    },
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
};
use slint::{
    platform::{
        WindowEvent,
        PointerEventButton,
    },
    LogicalPosition,
    LogicalSize,
};
use crate::message::{
    RenderMessage,
    WindowingMessage,
};

// TODO: Rename windowing to window in windowing_thread and WindowingMessage
pub fn windowing_thread(sender: Sender<WindowingMessage>, receiver: Receiver<RenderMessage>) {
    let conn = Connection::connect_to_env().unwrap();

    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let (globals, _queue) = registry_queue_init::<AppData>(&conn).unwrap();

    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 1..=5, ()).unwrap();
    let wl_surface = compositor.create_surface(&qh, ());
    let output: wl_output::WlOutput = globals.bind(&qh, 1..=1, ()).unwrap();
    let session_lock_manager: ext_session_lock_manager_v1::ExtSessionLockManagerV1 = globals.bind(&qh, 1..=1, ()).expect("ext_session_lock_v1 not available");
    let session_lock = session_lock_manager.lock(&qh, ());
    let session_lock_surface = session_lock.get_lock_surface(&wl_surface, &output, &qh, ());

    let mut state = AppData::new(
        RegistryState::new(&globals),
        display,
        wl_surface,
        session_lock,
        session_lock_surface,
        SeatState::new(&globals, &qh),
        sender,
    );
    
    while state.running {
        event_queue.blocking_dispatch(&mut state).unwrap();
        
        while let Ok(message) = receiver.try_recv() {
            match message {
                RenderMessage::AckResize { serial } => {
                    log::debug!("ack configure serial: {serial}");
                    state.session_lock_surface.ack_configure(serial);
                    state.render_thread_sender.send(
                        WindowingMessage::SurfaceResizeAcked { serial }
                    ).unwrap();
                }
            }
        }
    }
}

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
    // TODO: Support multiple outputs
    session_lock: ext_session_lock_v1::ExtSessionLockV1,
    session_lock_surface: ext_session_lock_surface_v1::ExtSessionLockSurfaceV1,

    sync_callback: Option<wl_callback::WlCallback>,

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
        session_lock_surface: ext_session_lock_surface_v1::ExtSessionLockSurfaceV1,
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
            session_lock_surface,
            wl_display: display,
            sync_callback: None,
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

impl Dispatch<wl_callback::WlCallback, ()> for AppData {
    fn event(
        state: &mut Self,
        callback: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if state.sync_callback.is_none() {
            return;
        }
        match event {
            wl_callback::Event::Done { .. } => {
                if callback == state.sync_callback.as_ref().unwrap() {
                    state.running = false;
                    state.sync_callback = None;
                }
            }
            _ => {}
        }

    }
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
        match event {
            ext_session_lock_surface_v1::Event::Configure { serial, width, height } => {
                log::debug!("surface reconfigure serial: {serial}");

                state.width = width;
                state.height = height;

                let sender = &state.render_thread_sender;
                if !state.configured {
                    sender.send(WindowingMessage::SurfaceReady {
                        display_id: state.wl_display.id(),
                        surface_id: state.wl_surface.id(),
                        size: (width, height)
                    }).unwrap();
                    state.configured = true;
                    surface.ack_configure(serial);
                } else {
                    sender.send(WindowingMessage::SurfaceResize {
                        size: (width, height),
                        serial
                    }).unwrap()
                }

            }
            _ => {}
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
                .get_keyboard(
                    qh,
                    &seat,
                    None
                )
                .expect("Failed to create keyboard");

            self.keyboard = Some(keyboard);
        }
        
        if capability == Capability::Pointer && self.pointer.is_none() {
            log::debug!("got pointer capability");

            let pointer = self
                .seat_state
                .get_pointer(qh, &seat).expect("Failed to create pointer");
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
    ) {}

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {}

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        if let Some(text) = event.utf8 {
            self.render_thread_sender.send(
                WindowingMessage::SlintWindowEvent(
                    WindowEvent::KeyPressed {
                        text: text.into()
                    }
                )
            ).unwrap();
        }
    }

    fn release_key(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        if let Some(text) = event.utf8 {
            self.render_thread_sender.send(
                WindowingMessage::SlintWindowEvent(
                    WindowEvent::KeyReleased {
                        text: text.into()
                    }
                )
            ).unwrap();
        }
        // Debug exit once escape is pressed
        if event.keysym == Keysym::Escape {
            self.session_lock.unlock_and_destroy();
            let sync_callback = self.wl_display.sync(qh, ());
            self.sync_callback = Some(sync_callback);
        }
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _: Modifiers,
    ) {}
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
                    self.render_thread_sender.send(
                        WindowingMessage::SlintWindowEvent(WindowEvent::PointerExited)
                    ).unwrap();
                }
                Motion { .. } => {
                    self.render_thread_sender.send(
                        WindowingMessage::SlintWindowEvent(WindowEvent::PointerMoved {
                            position 
                        })
                    ).unwrap();
                }
                Press { button, .. } => {
                    self.render_thread_sender.send(
                        WindowingMessage::SlintWindowEvent(WindowEvent::PointerPressed {
                            position,
                            button: wl_pointer_button_to_slint(button),
                        })
                    ).unwrap();
                }
                Release { button, .. } => {
                    self.render_thread_sender.send(
                        WindowingMessage::SlintWindowEvent(WindowEvent::PointerReleased {
                            position,
                            button: wl_pointer_button_to_slint(button),
                        })
                    ).unwrap();
                }
                Axis { horizontal, vertical, .. } => {
                    self.render_thread_sender.send(
                        WindowingMessage::SlintWindowEvent(WindowEvent::PointerScrolled {
                            position,
                            delta_x: horizontal.absolute as f32,
                            delta_y: vertical.absolute as f32,
                        })
                    ).unwrap();
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
