pub struct Args {
    pub fallback_config: bool,
}

pub fn parse_args() -> std::result::Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut fallback_config = true;
    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Long("fallback") => {
                fallback_config = true;
            }
            Long("no-fallback") => {
                fallback_config = false;
            }
            Short('h') | Long("help") => {
                println!(
                    "Usage: cthulock [OPTIONS]

Options:
--fallback              show a fallback lockscreen if loading your component fails (default)
--no-fallback           don't show a fallback, use only in testing"
                );
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(Args { fallback_config })
}
