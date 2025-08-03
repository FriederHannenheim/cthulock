use std::{fmt::Display, ops::Deref};

use slint_interpreter::ValueType;

use crate::{common::CthulockError, Result};

#[derive(PartialEq, Clone)]
pub struct SlintProperty {
    name: String,
    value_type: ValueType,
}

impl SlintProperty {
    pub fn new(name: &str, value_type: ValueType) -> Self {
        Self {
            name: name.to_owned(),
            value_type,
        }
    }
}

impl Display for SlintProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Property '{}' of type '{:?}'",
            self.name, self.value_type
        )
    }
}

impl From<(&str, ValueType)> for SlintProperty {
    fn from(value: (&str, ValueType)) -> Self {
        Self {
            name: value.0.to_owned(),
            value_type: value.1,
        }
    }
}

impl From<(String, ValueType)> for SlintProperty {
    fn from(value: (String, ValueType)) -> Self {
        Self {
            name: value.0,
            value_type: value.1,
        }
    }
}

macro_rules! properties_check {
    (
        $enum_name:ident,
        $($enum_option:ident -> ($property_name:expr,$property_type:expr)),+
    ) => {
        pub(crate) enum $enum_name {
            $(
                $enum_option,
            )+
        }

        impl $enum_name {
            pub fn check_propreties(existing_properties: &[SlintProperty]) -> Result<()> {
                let property_options = vec![$(SlintProperty::new($property_name, $property_type),)+];
                let missing_properties: Vec<_> = property_options
                                                    .iter()
                                                    .filter(|value| !existing_properties.contains(value))
                                                    .map(ToString::to_string)
                                                    .collect();

                if missing_properties.is_empty() {
                    Ok(())
                } else {
                    Err(CthulockError::MissingProperties(missing_properties))
                }
            }

        }

        impl Deref for $enum_name {
            type Target = str;

            fn deref(&self) -> &str {
                match self {
                    $($enum_name::$enum_option => $property_name,)+
                }
            }
        }
    };
}

properties_check!(
    RequiredProperties,
    Password -> ("password", ValueType::String)
);

properties_check!(
    OptionalProperties,
    ClockText -> ("clock_text", ValueType::String),
    CheckingPassword -> ("checking_password", ValueType::Bool)
);

macro_rules! callbacks_check {
    (
        $enum_name:ident,
        $($enum_option:ident -> $callback_name:expr),+
    ) => {
        pub(crate) enum $enum_name {
            $(
                $enum_option,
            )+
        }

        impl $enum_name {
            pub fn check_callbacks(existing_callbacks: &[String]) -> Result<()> {
                let callback_options = vec![$($callback_name.to_string(),)+];
                let missing_callbacks: Vec<_> = callback_options
                                                    .iter()
                                                    .filter(|value| !existing_callbacks.contains(value))
                                                    .map(ToString::to_string)
                                                    .collect();

                if missing_callbacks.is_empty() {
                    Ok(())
                } else {
                    Err(CthulockError::MissingCallbacks(missing_callbacks))
                }
            }

        }

        impl Deref for $enum_name {
            type Target = str;

            fn deref(&self) -> &str {
                match self {
                    $($enum_name::$enum_option => $callback_name,)+
                }
            }
        }
    };
}

callbacks_check!(
    RequiredCallbacks,
    Submit -> "submit"
);
