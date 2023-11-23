use wayland_client::backend::ObjectId;
use glutin::{
    api::egl::{
        surface::Surface,
        context::NotCurrentContext,
    },
    surface::{
        WindowSurface,
    },
};

#[derive(Debug)]
pub enum CthulockMessage {
    SurfaceReady {
        display_id: ObjectId,
        surface_id: ObjectId,
        size: (u32, u32)
    }
}
