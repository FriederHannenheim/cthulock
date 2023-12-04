use std::{sync::mpsc, thread};

use crate::{
    message::{UiMessage, WindowingMessage},
    ui::ui_thread,
    windowing_thread::windowing_thread,
    common::CthulockError,
};

type Result<T> = std::result::Result<T, CthulockError>;

mod common;
mod message;
mod ui;
mod windowing_thread;

fn main() -> Result<()> {
    init_logger();

    let theme = load_theme()?;

    let (sender_to_render, receiver_from_windowing) = mpsc::channel::<WindowingMessage>();
    let (sender_to_windowing, receiver_from_render) = mpsc::channel::<UiMessage>();
    thread::spawn(move || {
        ui_thread(&theme, sender_to_windowing, receiver_from_windowing).unwrap();
    });

    windowing_thread(sender_to_render, receiver_from_render);

    Ok(())
}


fn init_logger() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    #[cfg(not(debug_assertions))]
    env_logger::init();
}

fn load_theme() -> Result<String> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("cthulock").map_err(|_| {
       CthulockError::new("Failed to get XDG-Directories. This can only happen on Windows. Cthulock is not a Windows program.")
    })?;

    let theme_path = xdg_dirs.find_config_file("style.slint").ok_or(
        CthulockError::new("Could not find style.slint in config paths")
    )?;
    
    std::fs::read_to_string(theme_path).map_err(|e| {
        CthulockError::new(&e.to_string())
    })
}