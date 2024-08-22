#[macro_export]
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
macro_rules! debug {
    ($($arg:tt)*) => {
        {
            if *$crate::args::ARGS_DEBUG {
                println!($($arg)*);
            }
        }
    };
}

#[macro_export]
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
