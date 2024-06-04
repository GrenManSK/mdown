#[macro_export]
macro_rules! log {
    ($message:expr) => {
        {
        tracing::info!("@{}  {}", crate::resolute::HANDLE_ID.lock(), $message);
        crate::resolute::LOGS.lock().push(crate::metadata::LOG::new($message));
        }
    };
    ($message:expr, $name:expr, $to_write:expr) => {
        {
        if $to_write {
            tracing::info!("@{}  {}", crate::resolute::HANDLE_ID.lock().clone().into_string(), $message);
        }
        crate::resolute::LOGS.lock().push(crate::metadata::LOG::new_with_name($message, $name));
        }
    };
    ($message:expr, $name:expr) => {
        {
        tracing::info!("@{}  {}", $name, $message);
        if *args::ARGS_LOG {
            crate::resolute::LOGS.lock().push(crate::metadata::LOG::new_with_handle_id($message, $name));
        }
        }
    };
}

#[macro_export]
macro_rules! get_saver {
    () => {
        match *crate::resolute::SAVER.lock() {
            true => String::from("dataSaver"),
            false => String::from("data"),
        }
    };
    ($invert:expr) => {
        if $invert {
            match *resolute::SAVER.lock() {
                true => String::from("data"),
                false => String::from("dataSaver"),
            }
        } else  {
            match *crate::resolute::SAVER.lock()  {
                true => String::from("dataSaver"),
                false => String::from("data"),
            }
        }
    };
}
