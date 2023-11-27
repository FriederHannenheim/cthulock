use std::sync::mpsc::Sender;
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
use crate::message::CthulockMessage;


pub fn windowing_thread(sender: Sender<CthulockMessage>) {
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
    let _session_lock_surface = session_lock.get_lock_surface(&wl_surface, &output, &qh, ());

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

    sync_callback: Option<wl_callback::WlCallback>,

    seat_state: SeatState,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,

    render_thread_sender: Sender<CthulockMessage>,
}

impl AppData {
    fn new(
        registry_state: RegistryState,
        display: wl_display::WlDisplay,
        surface: wl_surface::WlSurface,
        session_lock: ext_session_lock_v1::ExtSessionLockV1,
        seat_state: SeatState,
        sender: Sender<CthulockMessage>,
    ) -> Self {
        Self {
            running: true,
            locked: false,
            configured: false,
            registry_state,
            wl_surface: surface,
            width: 0,
            height: 0,
            session_lock: session_lock,
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
                println!("surface reconfigure");
                state.width = width;
                state.height = height;
                surface.ack_configure(serial);
                state.configured = true;

                let message = CthulockMessage::SurfaceReady {
                    display_id: state.wl_display.id(), surface_id: state.wl_surface.id(), size: (width, height)
                };

                state.render_thread_sender.send(message).unwrap();
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
            println!("Unset keyboard capability");
            self.keyboard.take().unwrap().release();
        }

        if capability == Capability::Pointer && self.pointer.is_some() {
            println!("Unset pointer capability");
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
        println!("Key press: {event:?}");

    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        println!("Key release: {event:?}");
        if event.keysym == Keysym::Escape {
            
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
            match event.kind {
                Enter { .. } => {
                    println!("Pointer entered @{:?}", event.position);
                }
                Leave { .. } => {
                    println!("Pointer left");
                }
                Motion { .. } => {}
                Press { button, .. } => {
                    println!("Press {:x} @ {:?}", button, event.position);
                }
                Release { button, .. } => {
                    println!("Release {:x} @ {:?}", button, event.position);
                }
                Axis { horizontal, vertical, .. } => {
                    println!("Scroll H:{horizontal:?}, V:{vertical:?}");
                }
            }
        }
    }
}
