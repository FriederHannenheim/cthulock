use std::fs::File;
use std::os::fd::AsFd;
use wayland_client::{
    delegate_noop,
    protocol::{
        wl_buffer, wl_compositor, wl_keyboard, wl_shm, wl_seat, wl_registry, wl_shm_pool,
        wl_surface, wl_output, wl_callback, wl_display
    },
    Connection, Dispatch, QueueHandle, WEnum,
};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1,
    ext_session_lock_v1,
    ext_session_lock_surface_v1,
};


// This struct represents the state of our app. This simple app does not
// need any state, by this type still supports the `Dispatch` implementations.
#[derive(Default)]
struct AppData {
    running: bool,
    locked: bool,
    configured: bool,
    base_surface: Option<wl_surface::WlSurface>,
    width: u32,
    height: u32,
    // TODO: Support multiple outputs
    output: Option<wl_output::WlOutput>,
    buffer: Option<wl_buffer::WlBuffer>,
    session_lock: Option<ext_session_lock_v1::ExtSessionLockV1>,
    session_lock_surface: Option<ext_session_lock_surface_v1::ExtSessionLockSurfaceV1>,

    wl_display: Option<wl_display::WlDisplay>,
    sync_callback: Option<wl_callback::WlCallback>,
}

impl AppData {
    fn init_session_lock_surface(&mut self, qh: &QueueHandle<Self>) -> Option<()> {
        let session_lock = self.session_lock.as_ref()?;
        let base_surface = self.base_surface.as_ref()?;

        let output = self.output.as_ref()?;

        let session_lock_surface = session_lock.get_lock_surface(base_surface, output, qh, ());
        self.session_lock_surface = Some(session_lock_surface);
        
        Some(())
    }
}

// Implement `Dispatch<WlRegistry, ()> for out state. This provides the logic
// to be able to process events for the wl_registry interface.
impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<AppData>,
    ) {
        if let wl_registry::Event::Global { name, interface, .. } = event {
            println!("got global {}", interface);
            match &interface[..] {
                "wl_compositor" => {
                    let compositor = registry.bind::<wl_compositor::WlCompositor, _, _>(name, 1, qh, ());
                    let surface = compositor.create_surface(qh, ());

                    state.base_surface = Some(surface);

                    let _ = state.init_session_lock_surface(qh);
                },
                "wl_seat" => {
                    registry.bind::<wl_seat::WlSeat, _, _>(name, 1, qh, ());
                },
                "wl_output" => {
                    let output = registry.bind::<wl_output::WlOutput, _, _>(name, 1, qh, ());
                    state.output = Some(output);

                    let _ = state.init_session_lock_surface(qh);
                },
                "wl_shm" => {
                    let shm = registry.bind::<wl_shm::WlShm, _, _>(name, 1, qh, ());

                    let (init_w, init_h) = (1920, 1080);

                    let mut file = tempfile::tempfile().unwrap();
                    draw(&mut file, (init_w, init_h));
                    let pool = shm.create_pool(file.as_fd(), (init_w * init_h * 4) as i32, qh, ());
                    let buffer = pool.create_buffer(
                        0,
                        init_w as i32,
                        init_h as i32,
                        (init_w * 4) as i32,
                        wl_shm::Format::Argb8888,
                        qh,
                        (),
                    );
                    state.buffer = Some(buffer.clone());

                    if state.configured {
                        let surface = state.base_surface.as_ref().unwrap();
                        surface.attach(Some(&buffer), 0, 0);
                        surface.commit();
                    }
                },
                "ext_session_lock_manager_v1" => {
                    let session_lock_manager = registry.bind::<ext_session_lock_manager_v1::ExtSessionLockManagerV1, _, _>(name, 1, qh, ());
                    let session_lock = session_lock_manager.lock(qh, ());
                    state.session_lock = Some(session_lock);

                    let _ = state.init_session_lock_surface(qh);
                },
                _ => {}
            }
        }
    }
}

// Ignore events from these object types in this example.
delegate_noop!(AppData: ignore wl_compositor::WlCompositor);
delegate_noop!(AppData: ignore wl_surface::WlSurface);
delegate_noop!(AppData: ignore wl_shm::WlShm);
delegate_noop!(AppData: ignore wl_shm_pool::WlShmPool);
delegate_noop!(AppData: ignore wl_buffer::WlBuffer);
delegate_noop!(AppData: ignore wl_output::WlOutput);
delegate_noop!(AppData: ignore ext_session_lock_manager_v1::ExtSessionLockManagerV1);

fn draw(tmp: &mut File, (buf_x, buf_y): (u32, u32)) {
    use std::{cmp::min, io::Write};
    let mut buf = std::io::BufWriter::new(tmp);
    for y in 0..buf_y {
        for x in 0..buf_x {
            let a = 0xFF;
            let r = min(((buf_x - x) * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let g = min((x * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let b = min(((buf_x - x) * 0xFF) / buf_x, (y * 0xFF) / buf_y);

            let color = (a << 24) + (r << 16) + (g << 8) + b;
            buf.write_all(&color.to_ne_bytes()).unwrap();
        }
    }
    buf.flush().unwrap();
}

impl Dispatch<wl_seat::WlSeat, ()> for AppData {
    fn event(
        _: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities: WEnum::Value(capabilities) } = event {
            if capabilities.contains(wl_seat::Capability::Keyboard) {
                seat.get_keyboard(qh, ());
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for AppData {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key { key, .. } = event {
            if key == 1 {
                // ESC key
                state.session_lock.as_ref().unwrap().unlock_and_destroy();
                let sync_callback = state.wl_display.as_ref().unwrap().sync(qh, ());
                state.sync_callback = Some(sync_callback);
            }
        }
    }
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
                state.width = width;
                state.height = height;
                surface.ack_configure(serial);
                state.configured = true;
                let surface = state.base_surface.as_ref().unwrap();
                if let Some(ref buffer) = state.buffer {
                    surface.attach(Some(buffer), 0, 0);
                    surface.commit();
                }
            }
            _ => {}
        }
    }
}



// The main function of our program
fn main() {
    let conn = Connection::connect_to_env().unwrap();

    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let _registry = display.get_registry(&qh, ());

    let mut state = AppData {
        running: true,
        wl_display: Some(display),
        .. Default::default()
    };
    
    while state.running {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }
}
