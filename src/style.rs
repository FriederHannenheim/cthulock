use std::path::PathBuf;
use futures::executor::block_on;
use slint_interpreter::{ComponentDefinition, ComponentCompiler};

use crate::{Result, common::CthulockError, ui::slint_types::{SlintProperty, RequiredProperties, OptionalProperties, RequiredCallbacks}, args::Args};

pub(crate) const FALLBACK_STYLE: &str = include_str!("../docs/fallback_config.slint");

pub fn load_style_or_fallback(args: &Args) -> Result<ComponentDefinition> {
    let (style_string, config_dirs) = get_style_and_include_paths()?;
    let style = load_style(style_string, config_dirs, false);
    if let Err(e) = style {
        if args.fallback_config {
            log::error!("Loading cthulock config failed. Loading fallback config. Errors {e}");
            load_style(FALLBACK_STYLE.to_owned(), vec![], true)
        } else {
            Err(e)
        }
    } else {
        style
    }
}

fn get_style_and_include_paths() -> Result<(String, Vec<PathBuf>)> {
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
    Ok((style, config_dirs))
}

/// Load a slint style from a string with the include paths and check all the required properties and callbacks are present
fn load_style(style: String, include_paths: Vec<PathBuf>, supress_warnings: bool) -> Result<ComponentDefinition> {
    let mut compiler = ComponentCompiler::default();
    compiler.set_include_paths(include_paths);

    let definition = block_on(compiler.build_from_source(style.into(), Default::default()));
    slint_interpreter::print_diagnostics(&compiler.diagnostics());
    let definition = definition.ok_or(
        CthulockError::Generic("Compiling the Slint code failed".to_owned())
    )?;

    let slint_properties: Vec<_> = definition.properties().map(SlintProperty::from).collect();
    RequiredProperties::check_propreties(&slint_properties)?;
    if let Err(CthulockError::MissingProperties(properties)) = OptionalProperties::check_propreties(&slint_properties) {
        if !supress_warnings {
            log::info!("The following optional properties are not set: {properties:?}");
        }
    }

    let slint_callbacks: Vec<_> = definition.callbacks().collect();
    RequiredCallbacks::check_callbacks(&slint_callbacks)?;

    Ok(definition)
}

#[cfg(test)]
mod tests {
    use crate::Result;

    use super::{load_style, FALLBACK_STYLE};

    #[test]
    fn test_fallback_config() -> Result<()> {
        load_style(FALLBACK_STYLE.to_owned(), vec![], true)?;
        Ok(())
    }
}