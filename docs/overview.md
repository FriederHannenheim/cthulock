# Architectural Overview
Cthulock is split into 2 threads, the render thread and the windowing thread. These threads communitcate using message passing. The render thread sends `RenderMessage`s and recieves `WindowingMessage`s. For the windowing thread it's the other way around.

## Render thread
Creates an opengl context and implements a Slint backend.

## Windowing thread
Handles communication with the Wayland compositor. The first `WindowingMessage` sent is a `ẀindowingMessage::SurfaceReady` with this the Id of the `wl_display` and the `wl_surface` are sent which the render thread uses to create the OpenGL context.

After this, events for input and resize events are sent for the render thread to handle.
