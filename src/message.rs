use wayland_client::backend::ObjectId;

#[derive(Debug)]
pub enum CthulockMessage {
    SurfaceReady {
        display_id: ObjectId,
        surface_id: ObjectId,
        size: (u32, u32)
    }
}
