use std::fmt::Display;

use slint_interpreter::ValueType;

use crate::{Result, common::CthulockError};

#[derive(PartialEq, Clone)]
pub struct SlintProperty {
    name: String,
    value_type: ValueType,
}

impl SlintProperty {
    pub fn new(name: &str, value_type: ValueType) -> Self {
        Self {
            name: name.to_owned(),
            value_type
        }
    }
}

impl Display for SlintProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Property '{}' of type '{:?}'", self.name, self.value_type)
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
            value_type: value.1
        }
    }
}

pub fn check_propreties(required_properties: Vec<SlintProperty>, existing_properties: &Vec<SlintProperty>) -> Result<()> {
    let missing_properties: Vec<_> = required_properties
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

pub fn check_callbacks<T>(required_callbacks: &[T], existing_callbacks: &[T]) -> Result<()>
where T: AsRef<str> + PartialEq + Display {
    let missing_callbacks: Vec<_> = required_callbacks
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

pub fn get_required_callbacks() -> [std::string::String; 1] {
    ["submit".to_owned()]
}

pub fn get_required_properties() -> [SlintProperty; 1] {
    [SlintProperty::new("password", ValueType::String)]
}