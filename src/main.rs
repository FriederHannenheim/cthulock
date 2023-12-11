use std::{sync::mpsc, thread};

use futures::executor::block_on;
use slint_interpreter::{ComponentCompiler, ComponentDefinition};
use ui::slint_types::{SlintProperty, check_propreties, check_callbacks};

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

    let style = load_style()?;

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

fn load_style() -> Result<ComponentDefinition> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("cthulock").map_err(|_| {
       CthulockError::Generic("Failed to get XDG-Directories. This can only happen on Windows. Cthulock is not a Windows program.".to_owned())
    })?;

    let theme_path = xdg_dirs.find_config_file("style.slint").ok_or(
        CthulockError::Generic("Could not find style.slint in config paths".to_owned())
    )?;
    
    let style = std::fs::read_to_string(theme_path).map_err(|e| {
        CthulockError::Generic(e.to_string())
    })?;


    let mut config_dirs = xdg_dirs.get_config_dirs();
    config_dirs.push(xdg_dirs.get_config_home());
    let mut compiler = ComponentCompiler::default();
    compiler.set_include_paths(config_dirs);

    let definition = block_on(compiler.build_from_source(style.into(), Default::default()));
    slint_interpreter::print_diagnostics(&compiler.diagnostics());
    let definition = definition.ok_or(
        CthulockError::Generic("Compiling the Slint code failed".to_owned())
    )?;

    let slint_properties: Vec<_> = definition.properties().map(SlintProperty::from).collect();
    check_propreties(ui::slint_types::get_required_properties().to_vec(), &slint_properties)?;

    let slint_callbacks: Vec<_> = definition.callbacks().collect();
    check_callbacks(&ui::slint_types::get_required_callbacks(), &slint_callbacks)?;

    Ok(definition)
}