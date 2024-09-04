#[macro_export]
/// Logs a message with optional additional parameters.
///
/// This macro allows you to log messages with different levels of detail based on the provided arguments. It uses the `tracing` crate for logging and pushes the log entry into the `LOGS` collection.
///
/// # Parameters
///
/// - `$message:expr`: The message to be logged.
/// - `$name:expr`: An optional name to associate with the log entry.
/// - `$to_write:expr`: A boolean flag that determines whether to log the message using `tracing::info!`.
///
/// # Examples
///
/// ```rust
/// log!("This is a log message");
/// log!("This is a log message", "MyName", true);
/// log!("This is a log message", "MyName");
/// ```
macro_rules! log {
    ($message:expr) => {
        {
            tracing::info!("@{}  {}", $crate::resolute::HANDLE_ID.lock(), $message);
            $crate::resolute::LOGS.lock().push($crate::metadata::Log::new($message));
        }
    };
    ($message:expr, $name:expr, $to_write:expr) => {
        {
            if $to_write {
                tracing::info!("@{}  {}", $crate::resolute::HANDLE_ID.lock().clone().into_string(), $message);
            }
            $crate::resolute::LOGS.lock().push($crate::metadata::Log::new_with_name($message, $name));
        }
    };
    ($message:expr, $name:expr) => {
        {
            tracing::info!("@{}  {}", $name, $message);
            if *$crate::args::ARGS_LOG {
                $crate::resolute::LOGS.lock().push($crate::metadata::Log::new_with_handle_id($message, $name));
            }
        }
    };
}

#[macro_export]
/// Debug macro for logging messages to the console and optionally to a file.
///
/// This macro prints debug messages to the standard output and, if configured, also writes the messages to a file named `debug.log`. It checks if the debug flags are set before logging.
///
/// # Parameters
///
/// - `$($arg:tt)*`: The format string and arguments for the message to be logged.
///
/// # Examples
///
/// ```rust
/// debug!("This is a debug message with value: {}", 42);
/// ```
macro_rules! debug {
    ($($arg:tt)*) => {
        {
            use std::io::Write;
            if *$crate::args::ARGS_DEBUG || *$crate::args::ARGS_DEBUG_FILE {
                println!($($arg)*);
            }
            
            if *$crate::args::ARGS_DEBUG_FILE {
                if let Ok(mut file_inst) = $crate::fs::OpenOptions::new().create(true).append(true).open("debug.log") {
                    writeln!(file_inst, $($arg)*).expect("Failed to write to debug.log");
                }
            }
        }
    };
}

#[macro_export]
/// Retrieves the current saver setting, with optional inversion.
///
/// This macro determines the current data saver setting based on a locked state. It can optionally invert the result based on the provided boolean flag.
///
/// # Parameters
///
/// - `()` : No additional parameters. Returns the current saver setting.
/// - `($invert:expr)` : A boolean flag to invert the saver setting if true.
///
/// # Examples
///
/// ```rust
/// let saver = get_saver!();
/// let inverted_saver = get_saver!(true);
/// ```
macro_rules! get_saver {
    () => {
        match *$crate::resolute::SAVER.lock() {
            true => $crate::metadata::Saver::dataSaver,
            false => $crate::metadata::Saver::data,
        }
    };
    ($invert:expr) => {
        if $invert {
            match *$crate::resolute::SAVER.lock() {
                true => $crate::metadata::Saver::data,
                false => $crate::metadata::Saver::dataSaver,
            }
        } else  {
            match *$crate::resolute::SAVER.lock()  {
                true => $crate::metadata::Saver::dataSaver,
                false => $crate::metadata::Saver::data,
            }
        }
    };
}
