use std::{
    sync::mpsc,
    thread
};
use crate::{
    render_thread::render_thread,
    windowing_thread::windowing_thread,
    message::CthulockMessage,
};

mod render_thread;
mod egl;
mod window_adapter;
mod platform;
mod message;
mod windowing_thread;


// TODO: Logging
// TODO: Early init of surface to get image on screen sooner
fn main() {
    let (sender, receiver) = mpsc::channel::<CthulockMessage>();
    thread::spawn(move || {
        render_thread(receiver);
    });

    thread::spawn(move || {
        windowing_thread(sender);
    }).join().unwrap();
}
