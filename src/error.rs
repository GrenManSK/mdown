//! # mdown-error Crate
//!
//! The `mdown-error` crate is a utility library designed to manage and handle errors in the
//! `mdown` manga downloader application. It provides:
//!
//! - **Custom Error Types**: The `MdownError` enum defines a variety of error types that can occur
//!   during the manga downloading process. This includes network errors, I/O errors, JSON parsing
//!   errors, and more. Each error type includes a detailed error message for easier debugging and resolution.
//!
//! - **Global Error Suspension**: This crate includes a globally accessible, thread-safe list of
//!   suspended errors, `SUSPENDED`, which is implemented using a `Mutex` from the `parking_lot` crate.
//!   This allows errors that occur during various stages of the download process to be collected and
//!   handled later in a centralized manner.
//!
//! - **Error Handling Utilities**: Functions such as `handle_error`, `handle_suspended`, and
//!   `handle_final`, along with the `handle_error!` macro, simplify the process of printing and
//!   managing error messages, whether they are immediate or suspended for later handling.
//!
//! ## Usage
//!
//! ### Suspending Errors
//! You can suspend errors for later handling during the manga download process:
//!
//! ```rust
//! use error::{MdownError, suspend_error, handle_final};
//!
//! let error = MdownError::new();
//! suspend_error(error);
//! ```
//!
//! ### Handling Suspended Errors
//! Later, you can process all suspended errors, for example, after a download batch has completed:
//!
//! ```rust
//! handle_final(&MdownError::CustomError(String::from("Final error"), String::from("Test")));
//! ```
//!
//! ## Features
//!
//! - **Thread Safety**: The use of `Mutex` ensures that the suspended error list is safe to access
//!   from multiple threads concurrently, which is crucial when handling multiple downloads in parallel.
//! - **Detailed Error Messages**: Each error type in `MdownError` is associated with a descriptive
//!   error message that provides context, helping you to quickly identify and resolve issues.
//! - **Flexible Error Management**: The crate allows you to either handle errors immediately as they
//!   occur or suspend them for later processing, offering flexibility in how errors are managed in
//!   your manga downloader application.
//!
//! ## Example
//!
//! ```rust
//! use error::{MdownError, handle_error};
//!
//! fn download_chapter(chapter_url: &str) -> Result<(), MdownError> {
//!     let response = reqwest::get(chapter_url).map_err(|e| MdownError::NetworkError(e))?;
//!     // Further processing...
//!     Ok(())
//! }
//!
//! fn main() {
//!     if let Err(err) = download_chapter("https://example.com/manga/chapter1") {
//!         handle_error!(&err, String::from("main"));
//!     }
//! }
//! ```
//!
//! This example shows how to manage network errors that may occur during the download of a manga chapter.

use lazy_static::lazy_static;
use parking_lot::Mutex;
use thiserror::Error;

lazy_static! {
    pub static ref SUSPENDED: Mutex<Vec<MdownError>> = Mutex::new(Vec::new());
}

/// Suspends an error by adding it to the global `SUSPENDED` list.
///
/// # Arguments
///
/// * `err` - The `MdownError` instance to be suspended.
pub fn suspend_error(err: MdownError) {
    SUSPENDED.lock().push(err);
}

/// An enumeration representing different types of errors that can occur within the application.
/// Each variant of the `MdownError` enum represents a different kind of error with a corresponding
/// error message format.
#[derive(Debug, Error)]
pub enum MdownError {
    /// Represents an I/O error, with an associated message and file name.
    #[error("I/O error: {0} ({1})")]
    IoError(std::io::Error, String),

    /// Represents an HTTP status error, capturing the HTTP status code.
    #[error("Status error: {0}")]
    StatusError(reqwest::StatusCode),

    /// Represents a network-related error, capturing the underlying `reqwest::Error`.
    #[error("Network error: {0}")]
    NetworkError(reqwest::Error),

    /// Represents an error related to regular expressions, capturing the `regex::Error`.
    #[error("Regex error: {0}")]
    RegexError(regex::Error),

    /// Represents a JSON parsing or serialization error with an associated message.
    #[error("Json error: {0}")]
    JsonError(String),

    /// Represents a data conversion error with an associated message.
    #[error("Conversion error: {0}")]
    ConversionError(String),

    /// Represents a "not found" error with a description of what was not found.
    #[error("NotFound error: Didn't found {0}")]
    NotFoundError(String),

    /// Represents a ZIP file processing error, capturing the `zip::result::ZipError`.
    #[error("Zip error: {0}")]
    ZipError(zip::result::ZipError),

    /// Represents a database-related error, capturing the `rusqlite::Error`.
    #[error("Database error: {0}")]
    DatabaseError(rusqlite::Error),

    /// Represents a custom error with a message and an associated error name.
    #[error("{1} error: {0}")]
    CustomError(String, String),
}

impl MdownError {
    /// Converts the `MdownError` into a `String` representation, based on the type of error.
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

    /// Creates a new `MdownError` of type `CustomError` with a default message and error name.
    #[allow(dead_code)]
    pub fn new() -> MdownError {
        MdownError::CustomError(String::from("Nothing to worry about"), String::from("TestError"))
    }
}

/// Handles and prints errors of type `MdownError`.
///
/// This function formats and prints error messages based on the type of `MdownError` and optional context information.
/// It distinguishes between IO errors and other types of errors, providing detailed messages as appropriate.
///
/// # Arguments
/// * `err` - The `MdownError` to be handled and printed.
/// * `from` - An optional string providing additional context about where the error occurred. This will be included in the error message if provided.
///
/// # Examples
/// ```rust
/// use std::io;
///
/// let io_error = MdownError::IoError(io::Error::new(io::ErrorKind::NotFound, "File not found"), "file.txt".to_string());
/// let other_error = MdownError::OtherError("An unexpected error occurred".to_string());
///
/// // Handle and print IO error
/// handle_error(&io_error, Some("Reading file".to_string()));
///
/// // Handle and print other error
/// handle_error(&other_error, None);
/// ```
///
/// # Notes
/// * The function uses `eprintln!` to print error messages to the standard error output.
/// * The optional `from` argument provides additional context that helps in understanding where the error occurred.
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

/// A macro to simplify error handling by calling `handle_error` with optional origin information.
///
/// # Usage
///
/// ```rust
/// handle_error!(error_instance);
/// handle_error!(error_instance, String::from("some origin"));
/// ```
#[macro_export]
macro_rules! handle_error {
    ($err:expr) => {
        {
            $crate::error::handle_error($err, None);
        }
    };
    ($err:expr, $from:expr) => {
        {
            $crate::error::handle_error($err, Some($from));
        }
    };
}

/// Handles all suspended errors by printing them out. The suspended errors are those
/// that were previously added to the `SUSPENDED` list using `suspend_error`.
pub(crate) fn handle_suspended() {
    let suspended = SUSPENDED.lock();
    if !suspended.is_empty() {
        println!("Suspended errors:");
        for i in suspended.iter() {
            handle_error!(i, String::from("suspended"));
        }
    }
}

/// Handles a final error and any suspended errors by printing their messages.
/// The function first handles the provided error and then processes any errors
/// that were previously suspended.
pub(crate) fn handle_final(err: &MdownError) {
    handle_error!(err);
    handle_suspended();
}
