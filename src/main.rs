use std::{sync::mpsc, thread};

use crate::{
    message::{RenderMessage, WindowingMessage},
    render_thread::render_thread,
    windowing_thread::windowing_thread,
};

mod egl;
mod message;
mod platform;
mod render_thread;
mod window_adapter;
mod windowing_thread;

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    #[cfg(not(debug_assertions))]
    env_logger::init();

    let (sender_to_render, receiver_from_windowing) = mpsc::channel::<WindowingMessage>();
    let (sender_to_windowing, receiver_from_render) = mpsc::channel::<RenderMessage>();
    thread::spawn(move || {
        render_thread(sender_to_windowing, receiver_from_windowing);
    });

    windowing_thread(sender_to_render, receiver_from_render);
}
