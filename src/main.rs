use std::{
    sync::mpsc,
    thread
};
use env_logger::Env;

use crate::{
    render_thread::render_thread,
    windowing_thread::windowing_thread,
    message::{
        RenderMessage,
        WindowingMessage,
    },
};

mod render_thread;
mod egl;
mod window_adapter;
mod platform;
mod message;
mod windowing_thread;

// TODO: Solve resize race condition
fn main() {

    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(
        Env::default().default_filter_or("debug")
    ).init();

    #[cfg(not(debug_assertions))]
    env_logger::init();

    let (sender_to_render, receiver_from_windowing) = mpsc::channel::<WindowingMessage>();
    let (sender_to_windowing, receiver_from_render) = mpsc::channel::<RenderMessage>();
    thread::spawn(move || {
        render_thread(sender_to_windowing, receiver_from_windowing);
    });

    thread::spawn(move || {
        windowing_thread(sender_to_render, receiver_from_render);
    }).join().unwrap();
}
