use core::fmt;
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
    #[error("")]
    WindowingThreadQuit,
}