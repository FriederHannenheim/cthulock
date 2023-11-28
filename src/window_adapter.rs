use std::cell::Cell;
use std::rc::{Rc, Weak};
use slint::{
    Window,
    WindowSize,
    PhysicalSize,
    platform::{
        Renderer,
        WindowAdapter,
        WindowEvent,
        femtovg_renderer::FemtoVGRenderer,
    },
};


pub struct MinimalFemtoVGWindow {
    window: Window,
    renderer: FemtoVGRenderer,
    needs_redraw: Cell<bool>,
    size: Cell<PhysicalSize>,
}

impl MinimalFemtoVGWindow {
    pub fn new(renderer: FemtoVGRenderer) -> Rc<Self> {
        Rc::new_cyclic(|w: &Weak<Self>| Self {
            window: Window::new(w.clone()),
            renderer,
            needs_redraw: Default::default(),
            size: Default::default(),
        })
    }

    pub fn draw_if_needed(&self) {
        if self.needs_redraw.get() {
            log::debug!("drawing new frame");

            self.renderer.render().unwrap();
            self.needs_redraw.set(false);
        }
    }
}

impl WindowAdapter for MinimalFemtoVGWindow {
    fn window(&self) -> &Window {
        &self.window
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }

    fn size(&self) -> PhysicalSize {
        self.size.get()
    }

    fn set_size(&self, size: WindowSize) {
        self.size.set(size.to_physical(1.));
        self.window
            .dispatch_event(WindowEvent::Resized { size: size.to_logical(1.) })
    }

    fn request_redraw(&self) {
        self.needs_redraw.set(true);
    }
}

impl core::ops::Deref for MinimalFemtoVGWindow {
    type Target = Window;
    fn deref(&self) -> &Self::Target {
        &self.window
    }
}
