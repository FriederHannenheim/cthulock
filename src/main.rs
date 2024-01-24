use std::{sync::mpsc, thread};

use crate::{
    common::CthulockError,
    message::{UiMessage, WindowingMessage},
    style::load_style_or_fallback,
    ui::ui_thread,
    windowing_thread::windowing_thread,
};

type Result<T> = std::result::Result<T, CthulockError>;

mod args;
mod common;
mod message;
mod style;
mod ui;
mod windowing_thread;

// TODO: Better Error formatting
fn main() -> Result<()> {
    init_logger();

    let args = args::parse_args().map_err(CthulockError::ArgParseFail)?;

    let style = load_style_or_fallback(&args)?;

    let (sender_to_render, receiver_from_windowing) = mpsc::channel::<WindowingMessage>();
    let (sender_to_windowing, receiver_from_render) = mpsc::channel::<UiMessage>();

    thread::spawn(move || {
        if windowing_thread(sender_to_render.clone(), receiver_from_render).is_err() {
            sender_to_render.send(WindowingMessage::Quit).unwrap();
        }
    });

    ui_thread(style, sender_to_windowing, receiver_from_windowing)?;

    Ok(())
}

fn init_logger() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    #[cfg(not(debug_assertions))]
    env_logger::init();
}
