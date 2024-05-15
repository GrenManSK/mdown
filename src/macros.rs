#[macro_export]
macro_rules! log {
    ($message:expr) => {
        {
        let handle_id = match crate::resolute::HANDLE_ID.lock() {
            Ok(value) => value.clone().into_string(),
            Err(_err) => String::from(""),
        };
        tracing::info!("@{}  {}", handle_id, $message);
        match crate::resolute::LOGS.lock() {
            Ok(mut value) => value.push(crate::metadata::LOG::new($message)),
            Err(_err) => std::process::exit(1)
        }
        }
    };
    ($message:expr, $name:expr, $to_write:expr) => {
        {
        let handle_id = match crate::resolute::HANDLE_ID.lock() {
            Ok(value) => value.clone().into_string(),
            Err(_err) => String::from(""),
        };
        if $to_write {
            tracing::info!("@{}  {}", handle_id, $message);
        }
        match crate::resolute::LOGS.lock() {
            Ok(mut value) => value.push(crate::metadata::LOG::new_with_name($message, $name)),
            Err(_err) => std::process::exit(1)
        }
        }
    };
    ($message:expr, $name:expr) => {
        {
        let handle_id = match crate::resolute::HANDLE_ID.lock() {
            Ok(value) => value.clone().into_string(),
            Err(_err) => String::from(""),
        };
        tracing::info!("@{}  {}", handle_id, $message);
        if crate::ARGS.log {
            match crate::resolute::LOGS.lock() {
                Ok(mut value) => value.push(crate::metadata::LOG::new_with_handle_id($message, $name)),
                Err(_err) => std::process::exit(1)
            }
        }
        }
    };
}

#[macro_export]
macro_rules! get_saver {
    () => {
        match crate::resolute::SAVER.lock() {
            Ok(value) =>
                match *value {
                    true => String::from("dataSaver"),
                    false => String::from("data"),
                }
            Err(_err) => String::from("data")
        }
    };
    ($invert:expr) => {
        if $invert {
            match resolute::SAVER.lock() {
                Ok(value) =>
                    match *value {
                        true => String::from("data"),
                        false => String::from("dataSaver"),
                    }
                Err(_err) => String::from("dataSaver")
            }
        } else  {
            match crate::resolute::SAVER.lock() {
                Ok(value) =>
                    match *value {
                        true => String::from("dataSaver"),
                        false => String::from("data"),
                    }
                Err(_err) => String::from("data")
            }
        }
    };
}
