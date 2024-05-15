use crate::resolute::SUSPENDED;

pub(crate) mod mdown {
    #[derive(Debug)]
    pub(crate) enum Error {
        IoError(std::io::Error, Option<String>),
        StatusError(reqwest::StatusCode),
        NetworkError(reqwest::Error),
        RegexError(regex::Error),
        JsonError(String),
        PoisonError(String),
        ConversionError(String),
        NotFoundError(String),
        ZipError(zip::result::ZipError),
        CustomError(String, String),
    }
    #[derive(Debug)]
    pub(crate) enum Final {
        Final(Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::IoError(msg, name) => {
                    match name {
                        Some(name) => write!(f, "Error: IO Error {} for file {}", msg, name),
                        None => write!(f, "Error: IO Error {}", msg),
                    }
                }
                Error::StatusError(msg) => write!(f, "Error: {}", msg),
                Error::NetworkError(msg) => write!(f, "Error: {}", msg),
                Error::JsonError(msg) =>
                    write!(f, "Error: either corrupt json file or not found item; {}", msg),
                Error::PoisonError(msg) => write!(f, "Error: Mutex PoisonError {}", msg),
                Error::ConversionError(msg) => write!(f, "Error: ConversionError {}", msg),
                Error::RegexError(msg) => write!(f, "Error: RegexError {}", msg),
                Error::ZipError(msg) => write!(f, "Error: ZipError {}", msg),
                Error::NotFoundError(msg) => write!(f, "Error: NotFoundError {}", msg),
                Error::CustomError(msg, name) => write!(f, "Error: {} {}", name, msg),
            }
        }
    }
    impl std::fmt::Display for Final {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Final::Final(msg) => write!(f, "{}", msg.to_string()),
            }
        }
    }

    impl Error {
        pub(crate) fn into(self) -> String {
            match self {
                Error::IoError(msg, _name) => msg.to_string(),
                Error::StatusError(msg) => msg.to_string(),
                Error::NetworkError(msg) => msg.to_string(),
                Error::JsonError(msg) => msg,
                Error::PoisonError(msg) => msg,
                Error::ConversionError(msg) => msg,
                Error::NotFoundError(msg) => msg,
                Error::ZipError(msg) => msg.to_string(),
                Error::RegexError(msg) => msg.to_string(),
                Error::CustomError(msg, name) => format!("Error: {} {}", name, msg),
            }
        }
    }
    // impl Final {
    //     pub(crate) fn into(self) -> String {
    //         match self {
    //             Final::Final(msg) => msg.into(),
    //         }
    //     }
    // }
}

pub(crate) fn handle_error(err: &mdown::Error, from: String) {
    match err {
        mdown::Error::IoError(err, name) => {
            match name {
                Some(name) => eprintln!("Error: IO Error {} in file {} ({})", err, name, from),
                None => eprintln!("Error: IO Error {} ({})", err, from),
            }
        }
        mdown::Error::StatusError(err) => eprintln!("Error: Network Error {} ({})", err, from),
        mdown::Error::NetworkError(err) => eprintln!("Error: Network Error {} ({})", err, from),
        mdown::Error::JsonError(err) => eprintln!("Error: Json Error {} ({})", err, from),
        mdown::Error::PoisonError(err) => eprintln!("Error: Mutex PoisonError {} ({})", err, from),
        mdown::Error::RegexError(err) => eprintln!("Error: RegexError {} ({})", err, from),
        mdown::Error::ZipError(err) => eprintln!("Error: ZipError {} ({})", err, from),
        mdown::Error::NotFoundError(err) => eprintln!("Error: NotFoundError {} ({})", err, from),
        mdown::Error::ConversionError(err) =>
            eprintln!("Error: ConversionError {} ({})", err, from),
        mdown::Error::CustomError(err, name) => eprintln!("Error: {} {} ({})", name, err, from),
    }
}

pub(crate) fn handle_suspended() {
    match SUSPENDED.lock() {
        Ok(suspended) => {
            if !suspended.is_empty() {
                println!("Suspended errors:");
                for i in suspended.iter() {
                    handle_error(i, String::from("suspended"));
                }
            }
        }
        Err(err) => {
            eprintln!("Error locking SUSPENDED: {}", err);
        }
    }
}

pub(crate) fn handle_final(err: &mdown::Final) {
    eprintln!("{}", err);
    handle_suspended();
}
