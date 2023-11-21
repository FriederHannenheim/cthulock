use std::rc::Rc;
use wayland_client::{
    delegate_noop,
    protocol::{
        wl_buffer, wl_compositor, wl_keyboard, wl_seat, wl_registry,
        wl_surface, wl_output, wl_callback, wl_display
    },
    Connection, Dispatch, QueueHandle, WEnum,
};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1,
    ext_session_lock_v1,
    ext_session_lock_surface_v1,
};
use slint::{
    platform::{
        femtovg_renderer::FemtoVGRenderer,
        WindowAdapter,
    }, PhysicalSize,
};

use crate::egl::OpenGLContext;
use crate::window_adapter::MinimalFemtoVGWindow;
use crate::platform::CthuluSlintPlatform;

mod egl;
mod window_adapter;
mod platform;


// This struct represents the state of our app
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

    slint_window: Option<Rc<MinimalFemtoVGWindow>>,
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

// Ignore events from these object types
delegate_noop!(AppData: ignore wl_compositor::WlCompositor);
delegate_noop!(AppData: ignore wl_surface::WlSurface);
delegate_noop!(AppData: ignore wl_buffer::WlBuffer);
delegate_noop!(AppData: ignore wl_output::WlOutput);
delegate_noop!(AppData: ignore ext_session_lock_manager_v1::ExtSessionLockManagerV1);

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
        app_state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key { key, .. } = event {
            // TODO: Send input to slint
            if key == 1 {
                // ESC key
                app_state.session_lock.as_ref().unwrap().unlock_and_destroy();
                let sync_callback = app_state.wl_display.as_ref().unwrap().sync(qh, ());
                app_state.sync_callback = Some(sync_callback);
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
                println!("surface reconfigure");
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

slint::slint!{
    export component HelloWorld {
        Text {
            text: "hello world";
            color: black;
        }
    }
}

// TODO: Logging
// TODO: Multithreading with Channels
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
    
    let mut slint_setup_done = false;
    

    while state.running {
        if let Some(guard) = event_queue.prepare_read() {
            guard.read().ok();
        }
        event_queue.dispatch_pending(&mut state).unwrap();
        event_queue.flush().unwrap();
        if !slint_setup_done && state.configured {
            // TODO: set initial opengl suface size correctly

            let base_surface = state.base_surface.as_ref().unwrap();
            let wl_display = state.wl_display.as_ref().unwrap();
            let gl_context = OpenGLContext::new(wl_display, base_surface, (state.width, state.height));
            let renderer = FemtoVGRenderer::new(gl_context).unwrap();
            let slint_window = MinimalFemtoVGWindow::new(renderer);
            slint_window.set_size(slint::WindowSize::Physical(PhysicalSize::new(state.width, state.height)));

            let platform = CthuluSlintPlatform::new(slint_window.clone());

            state.slint_window = Some(slint_window);
            slint::platform::set_platform(Box::new(platform)).unwrap();
            let _ui = HelloWorld::new().expect("Failed to load UI").show();
            slint_setup_done = true;
        }
        if slint_setup_done {
            slint::platform::update_timers_and_animations();
            state.slint_window.as_ref().unwrap().draw_if_needed();
        }
    }
}
