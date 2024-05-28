#[macro_export]
macro_rules! log {
    ($message:expr) => {
        {
        let handle_id = crate::resolute::HANDLE_ID.lock();
        tracing::info!("@{}  {}", handle_id, $message);
        crate::resolute::LOGS.lock().push(crate::metadata::LOG::new($message));
        }
    };
    ($message:expr, $name:expr, $to_write:expr) => {
        {
        let handle_id = crate::resolute::HANDLE_ID.lock().clone().into_string();
        if $to_write {
            tracing::info!("@{}  {}", handle_id, $message);
        }
        crate::resolute::LOGS.lock().push(crate::metadata::LOG::new_with_name($message, $name));
        }
    };
    ($message:expr, $name:expr) => {
        {
        let handle_id = crate::resolute::HANDLE_ID.lock().clone().into_string();
        tracing::info!("@{}  {}", handle_id, $message);
        if crate::ARGS.log {
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
