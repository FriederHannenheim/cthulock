use slint::platform::WindowEvent;
use wayland_client::backend::ObjectId;

#[derive(Debug)]
pub enum WindowingMessage {
    SurfaceReady {
        display_id: ObjectId,
        surface_id: ObjectId,
        size: (u32, u32),
    },
    SurfaceResize {
        size: (u32, u32),
        serial: u32,
    },
    SurfaceResizeAcked {
        serial: u32,
    },
    SlintWindowEvent(WindowEvent),
    UnlockFailed,
    Quit,
}

#[derive(Debug)]
pub enum UiMessage {
    AckResize { serial: u32 },
    UnlockWithPassword { password: String },
}
