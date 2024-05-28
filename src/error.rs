use crate::resolute::SUSPENDED;

#[derive(Debug)]
pub enum MdownError {
    IoError(std::io::Error, Option<String>),
    StatusError(reqwest::StatusCode),
    NetworkError(reqwest::Error),
    RegexError(regex::Error),
    JsonError(String),
    PoisonError(String),
    ConversionError(String),
    NotFoundError(String),
    ZipError(zip::result::ZipError),
    DatabaseError(rusqlite::Error),
    CustomError(String, String),
}
#[derive(Debug)]
pub enum Final {
    Final(MdownError),
}

impl std::fmt::Display for MdownError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MdownError::IoError(msg, name) => {
                match name {
                    Some(name) => write!(f, "Error: IO Error {} for file {}", msg, name),
                    None => write!(f, "Error: IO Error {}", msg),
                }
            }
            MdownError::StatusError(msg) => write!(f, "Error: {}", msg),
            MdownError::NetworkError(msg) => write!(f, "Error: {}", msg),
            MdownError::JsonError(msg) =>
                write!(f, "Error: either corrupt json file or not found item; {}", msg),
            MdownError::PoisonError(msg) => write!(f, "Error: Mutex PoisonError {}", msg),
            MdownError::ConversionError(msg) => write!(f, "Error: ConversionError {}", msg),
            MdownError::RegexError(msg) => write!(f, "Error: RegexError {}", msg),
            MdownError::ZipError(msg) => write!(f, "Error: ZipError {}", msg),
            MdownError::NotFoundError(msg) => write!(f, "Error: NotFoundError {}", msg),
            MdownError::DatabaseError(msg) => write!(f, "Error: DatabaseError {}", msg),
            MdownError::CustomError(msg, name) => write!(f, "Error: {} {}", name, msg),
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

impl MdownError {
    pub fn into(self) -> String {
        match self {
            MdownError::IoError(msg, _name) => msg.to_string(),
            MdownError::StatusError(msg) => msg.to_string(),
            MdownError::NetworkError(msg) => msg.to_string(),
            MdownError::JsonError(msg) => msg,
            MdownError::PoisonError(msg) => msg,
            MdownError::ConversionError(msg) => msg,
            MdownError::NotFoundError(msg) => msg,
            MdownError::ZipError(msg) => msg.to_string(),
            MdownError::RegexError(msg) => msg.to_string(),
            MdownError::DatabaseError(msg) => msg.to_string(),
            MdownError::CustomError(msg, name) => format!("Error: {} {}", name, msg),
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

pub(crate) fn handle_error(err: &MdownError, from: String) {
    match err {
        MdownError::IoError(err, name) => {
            match name {
                Some(name) => eprintln!("Error: IO Error {} in file {} ({})", err, name, from),
                None => eprintln!("Error: IO Error {} ({})", err, from),
            }
        }
        MdownError::StatusError(err) => eprintln!("Error: Network Error {} ({})", err, from),
        MdownError::NetworkError(err) => eprintln!("Error: Network Error {} ({})", err, from),
        MdownError::JsonError(err) => eprintln!("Error: Json Error {} ({})", err, from),
        MdownError::PoisonError(err) => eprintln!("Error: Mutex PoisonError {} ({})", err, from),
        MdownError::RegexError(err) => eprintln!("Error: RegexError {} ({})", err, from),
        MdownError::ZipError(err) => eprintln!("Error: ZipError {} ({})", err, from),
        MdownError::NotFoundError(err) => eprintln!("Error: NotFoundError {} ({})", err, from),
        MdownError::ConversionError(err) => eprintln!("Error: ConversionError {} ({})", err, from),
        MdownError::DatabaseError(err) => eprintln!("Error: DatabaseError {} ({})", err, from),
        MdownError::CustomError(err, name) => eprintln!("Error: {} {} ({})", name, err, from),
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

pub(crate) fn handle_final(err: &Final) {
    eprintln!("{}", err);
    handle_suspended();
}
