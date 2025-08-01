use futures_lite::future::block_on;
use slint_interpreter::{Compiler, ComponentDefinition};
use std::path::PathBuf;

use crate::{
    args::Args,
    common::CthulockError,
    ui::slint_types::{OptionalProperties, RequiredCallbacks, RequiredProperties, SlintProperty},
    Result,
};

pub(crate) const FALLBACK_STYLE: &str = include_str!("../docs/fallback_config.slint");

pub fn load_style_or_fallback(args: &Args) -> Result<ComponentDefinition> {
    let style = get_style_and_include_paths()
        .and_then(|(style_string, config_dirs)| load_style(style_string, config_dirs, false));
    if let Err(e) = style {
        if args.fallback_config {
            log::error!("Loading cthulock config failed. Loading fallback config. Errors: \n{e}");
            load_style(FALLBACK_STYLE.to_owned(), vec![], true)
        } else {
            Err(e)
        }
    } else {
        style
    }
}

fn get_style_and_include_paths() -> Result<(String, Vec<PathBuf>)> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("cthulock");

    let theme_path = xdg_dirs
        .find_config_file("style.slint")
        .ok_or(CthulockError::Generic(
            "Could not find style.slint in config paths".to_owned(),
        ))?;

    let style =
        std::fs::read_to_string(theme_path).map_err(|e| CthulockError::Generic(e.to_string()))?;

    let mut config_dirs = xdg_dirs.get_config_dirs();
    config_dirs.push(xdg_dirs.get_config_home().ok_or(CthulockError::Generic(
        "Failed to get XDG-Directories. This can only happen on Windows. Cthulock is not a Windows program.".to_owned(),
    ))?);
    Ok((style, config_dirs))
}

/// Load a slint style from a string with the include paths and check all the required properties and callbacks are present
fn load_style(
    style: String,
    include_paths: Vec<PathBuf>,
    supress_warnings: bool,
) -> Result<ComponentDefinition> {
    let mut compiler = Compiler::default();
    compiler.set_include_paths(include_paths);

    let result = block_on(compiler.build_from_source(style, Default::default()));
    result.print_diagnostics();
    let definition = result.component(result.component_names().next().unwrap_or_default());
    let definition = definition.ok_or(CthulockError::Generic(
        "Compiling the Slint code failed".to_owned(),
    ))?;

    let slint_properties: Vec<_> = definition.properties().map(SlintProperty::from).collect();
    RequiredProperties::check_propreties(&slint_properties)?;
    if let Err(CthulockError::MissingProperties(properties)) =
        OptionalProperties::check_propreties(&slint_properties)
    {
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
