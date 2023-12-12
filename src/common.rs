use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub(crate) enum CthulockError {
    #[error("{0}")]
    Generic(String),
    #[error("The following Properties are missing:\n {0:?} \nCheck if they exist and have the correct type")]
    MissingProperties(Vec<String>),
    #[error("The following Callbacks are missing:\n {0:?}")]
    MissingCallbacks(Vec<String>),
    #[error("")]
    WindowingThreadQuit,
}