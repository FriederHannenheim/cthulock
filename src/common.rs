use thiserror::Error;

// TODO: Proper Error handling everywhere
#[derive(Error, Debug, Clone)]
pub(crate) enum CthulockError {
    #[error("{0}")]
    Generic(String),
    #[error("Failed to bind slint '{0}' callback.\
The Cthulock Slint component needs to have a '{0}' callback.\
Consult the documentation for further information.")]
    CallbackBindFail(String),
    #[error("Failed to get or set '{0}' slint property.\
The Cthulock Slint component needs to have a '{0}' property.\
Consult the documentation for further information.")]
    PropertyFail(String),
    #[error("The following Properties are missing:\n {0:?} \nCheck if they exist and have the correct type")]
    MissingProperties(Vec<String>),
    #[error("The following Callbacks are missing:\n {0:?}")]
    MissingCallbacks(Vec<String>),
    #[error("")]
    WindowingThreadQuit,
}