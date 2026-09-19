use uuid::Uuid;

use crate::error::{CoreError, Result};

pub(crate) fn required_text(field: &'static str, value: &str, max_len: usize) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Validation {
            field,
            message: "must not be empty".into(),
        });
    }
    if trimmed.chars().count() > max_len {
        return Err(CoreError::Validation {
            field,
            message: format!("must be at most {max_len} characters"),
        });
    }
    Ok(trimmed.to_owned())
}

pub(crate) fn optional_text(field: &'static str, value: &str, max_len: usize) -> Result<String> {
    if value.chars().count() > max_len {
        return Err(CoreError::Validation {
            field,
            message: format!("must be at most {max_len} characters"),
        });
    }
    Ok(value.trim().to_owned())
}

pub(crate) fn choice(field: &'static str, value: &str, allowed: &[&str]) -> Result<String> {
    if allowed.contains(&value) {
        Ok(value.to_owned())
    } else {
        Err(CoreError::Validation {
            field,
            message: format!("must be one of: {}", allowed.join(", ")),
        })
    }
}

pub(crate) fn uuid(field: &'static str, value: &str) -> Result<String> {
    Uuid::parse_str(value).map_err(|_| CoreError::Validation {
        field,
        message: "must be a UUID".into(),
    })?;
    Ok(value.to_owned())
}

pub(crate) fn optional_uuid(field: &'static str, value: Option<&str>) -> Result<Option<String>> {
    value.map(|candidate| uuid(field, candidate)).transpose()
}
