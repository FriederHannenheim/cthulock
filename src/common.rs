use core::fmt;


// TODO: Proper Error handling everywhere
#[derive(Debug, Clone)]
pub(crate) struct CthulockError {
    message: String
}

impl CthulockError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_owned()
        }
    }

    pub fn callback_bind_fail(name: &str) -> Self {
        Self {
            message: format!("\
Failed to bind slint '{name}' callback.\
The Cthulock Slint component needs to have a '{name}' callback.\
Consult the documentation for further information.")
        }
    }

    pub fn property_fail(name: &str) -> Self {
        Self {
            message: format!("\
Failed to get or set '{name}' slint property.\
The Cthulock Slint component needs to have a '{name}' property.\
Consult the documentation for further information.")
        }
    }
}

impl fmt::Display for CthulockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}