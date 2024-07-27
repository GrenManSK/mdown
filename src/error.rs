use thiserror::Error;

use crate::resolute::SUSPENDED;
#[derive(Debug, Error)]
pub enum MdownError {
    #[error("I/O error: {0} ({1})")] IoError(std::io::Error, String),
    #[error("Status error: {0}")] StatusError(reqwest::StatusCode),
    #[error("Network error: {0}")] NetworkError(reqwest::Error),
    #[error("Regex error: {0}")] RegexError(regex::Error),
    #[error("Json error: {0}")] JsonError(String),
    #[error("Conversion error: {0}")] ConversionError(String),
    #[error("NotFound error: Didn't found {0}")] NotFoundError(String),
    #[error("Zip error: {0}")] ZipError(zip::result::ZipError),
    #[error("Database error: {0}")] DatabaseError(rusqlite::Error),
    #[error("{1} error: {0}")] CustomError(String, String),
}

impl MdownError {
    pub fn into(self) -> String {
        match self {
            MdownError::IoError(msg, _name) => msg.to_string(),
            MdownError::StatusError(msg) => msg.to_string(),
            MdownError::NetworkError(msg) => msg.to_string(),
            MdownError::JsonError(msg) => msg,
            MdownError::ConversionError(msg) => msg,
            MdownError::NotFoundError(msg) => msg,
            MdownError::ZipError(msg) => msg.to_string(),
            MdownError::RegexError(msg) => msg.to_string(),
            MdownError::DatabaseError(msg) => msg.to_string(),
            MdownError::CustomError(msg, name) => format!("Error: {} {}", name, msg),
        }
    }
}

pub(crate) fn handle_error(err: &MdownError, from: Option<String>) {
    let to = match from {
        Some(value) => format!(" ({})", value),
        None => String::new(),
    };
    match err {
        MdownError::IoError(err, name) => {
            match name.as_str() {
                "" => eprintln!("Error: IO Error {} ({})", err, to),
                name => eprintln!("Error: IO Error {} in file {}{}", err, name, to),
            }
        }
        error => eprintln!("Error: {}{}", error, to),
    }
}

#[macro_export]
macro_rules! handle_error {
    ($err:expr) => {
        {
            crate::error::handle_error($err, None);
        }
    };
    ($err:expr, $from:expr) => {
        {
            crate::error::handle_error($err, Some($from));
        }
    };
}

pub(crate) fn handle_suspended() {
    let suspended = SUSPENDED.lock();
    if !suspended.is_empty() {
        println!("Suspended errors:");
        for i in suspended.iter() {
            handle_error!(i, String::from("suspended"));
        }
    }
}

pub(crate) fn handle_final(err: &MdownError) {
    handle_error!(err.into());
    handle_suspended();
}
