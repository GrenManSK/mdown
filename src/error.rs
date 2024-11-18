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
use smallvec::{ SmallVec, smallvec };

use crate::{ MAXPOINTS, resolute::INITSCR_INIT, string };

lazy_static! {
    pub static ref SUSPENDED: Mutex<SmallVec<[MdownError; 3]>> = Mutex::new(smallvec![]);
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
    #[error("I/O error: {0} ({1}) ({2})")]
    IoError(std::io::Error, String, u32),

    /// Represents an HTTP status error, capturing the HTTP status code.
    #[error("Status error: {0} ({1})")]
    StatusError(reqwest::StatusCode, u32),

    /// Represents a network-related error, capturing the underlying `reqwest::Error`.
    #[error("Network error: {0} ({1})")]
    NetworkError(reqwest::Error, u32),

    /// Represents an error related to regular expressions, capturing the `regex::Error`.
    #[error("Regex error: {0} ({1})")]
    RegexError(regex::Error, u32),

    /// Represents a JSON parsing or serialization error with an associated message.
    #[error("Json error: {0} ({1})")]
    JsonError(String, u32),

    /// Represents a data conversion error with an associated message.
    #[error("Conversion error: {0} ({1})")]
    ConversionError(String, u32),

    /// Represents a "not found" error with a description of what was not found.
    #[error("NotFound error: Didn't found {0} ({1})")]
    NotFoundError(String, u32),

    /// Represents a ZIP file processing error, capturing the `zip::result::ZipError`.
    #[error("Zip error: {0} ({1})")]
    ZipError(zip::result::ZipError, u32),

    /// Represents a database-related error, capturing the `rusqlite::Error`.
    #[error("Database error: {0} ({1})")]
    DatabaseError(rusqlite::Error, u32),

    /// Represents a custom error with a message and an associated error name.
    #[error("{1} error: {0} ({2})")]
    CustomError(String, String, u32),
}

impl MdownError {
    /// Converts the `MdownError` into a `String` representation, based on the type of error.
    pub fn into(self) -> String {
        match self {
            MdownError::IoError(msg, _name, err_code) =>
                format!("{} Code: {}", msg.to_string(), err_code),
            MdownError::StatusError(msg, err_code) =>
                format!("{} Code: {}", msg.to_string(), err_code),
            MdownError::NetworkError(msg, err_code) =>
                format!("{} Code: {}", msg.to_string(), err_code),
            MdownError::JsonError(msg, err_code) => format!("{} Code: {}", msg, err_code),
            MdownError::ConversionError(msg, err_code) => format!("{} Code: {}", msg, err_code),
            MdownError::NotFoundError(msg, err_code) => format!("{} Code: {}", msg, err_code),
            MdownError::ZipError(msg, err_code) =>
                format!("{} Code: {}", msg.to_string(), err_code),
            MdownError::RegexError(msg, err_code) =>
                format!("{} Code: {}", msg.to_string(), err_code),
            MdownError::DatabaseError(msg, err_code) =>
                format!("{} Code: {}", msg.to_string(), err_code),
            MdownError::CustomError(msg, name, err_code) =>
                format!("Error: {} {} Code {}", name, msg, err_code),
        }
    }

    pub fn code(&self) -> i32 {
        *(match self {
            MdownError::IoError(_, _, err_code) => err_code,
            MdownError::StatusError(_, err_code) => err_code,
            MdownError::NetworkError(_, err_code) => err_code,
            MdownError::JsonError(_, err_code) => err_code,
            MdownError::ConversionError(_, err_code) => err_code,
            MdownError::NotFoundError(_, err_code) => err_code,
            MdownError::ZipError(_, err_code) => err_code,
            MdownError::RegexError(_, err_code) => err_code,
            MdownError::DatabaseError(_, err_code) => err_code,
            MdownError::CustomError(_, _, err_code) => err_code,
        }) as i32
    }
    /// Creates a new `MdownError` of type `CustomError` with a default message and error name.
    #[allow(dead_code)]
    pub fn new() -> MdownError {
        MdownError::CustomError(
            String::from("Nothing to worry about"),
            String::from("TestError"),
            11000
        )
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
        MdownError::IoError(err, name, err_code) => {
            match name.as_str() {
                "" => eprintln!("Error: IO Error {} ({}) Code: {}", err, to, err_code),
                name =>
                    eprintln!("Error: IO Error {} in file {}{} Code: {}", err, name, to, err_code),
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
            let err_code = $crate::error::handle_error($err, None);
            err_code
        }
    };
    ($err:expr, $from:expr) => {
        {
            let err_code = $crate::error::handle_error($err, Some($from));
            err_code
        }
    };
}

/// Handles all suspended errors by printing them out. The suspended errors are those
/// that were previously added to the `SUSPENDED` list using `suspend_error`.
pub(crate) fn handle_suspended() {
    let suspended = SUSPENDED.lock();
    if !suspended.is_empty() {
        if *INITSCR_INIT.lock() {
            let start = MAXPOINTS.max_y - 1 - (suspended.len() as u32);
            string(start - 1, 0, "Suspended errors:");
            for (times, err) in suspended.iter().enumerate() {
                let to = " (suspended)";
                let message = match err {
                    MdownError::IoError(err, name, err_code) => {
                        match name.as_str() {
                            "" => format!("Error: IO Error {} ({}) Code: {}", err, to, err_code),
                            name =>
                                format!(
                                    "Error: IO Error {} in file {}{} Code: {}",
                                    err,
                                    name,
                                    to,
                                    err_code
                                ),
                        }
                    }
                    error => format!("Error: {}{}", error, to),
                };
                string(start + (times as u32), 0, &message);
            }
        } else {
            println!("Suspended errors:");
            for i in suspended.iter() {
                handle_error!(i, String::from("suspended"));
            }
        }
    }
}

/// Handles a final error and any suspended errors by printing their messages.
/// The function first handles the provided error and then processes any errors
/// that were previously suspended.
pub(crate) fn handle_final(err: &MdownError) -> i32 {
    let err_code = err.code();
    handle_error!(err);
    handle_suspended();
    err_code
}
