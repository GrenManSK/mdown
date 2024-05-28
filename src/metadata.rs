use chrono::Utc;
use std::collections::HashMap;
use serde_json::{ json, Value };

use crate::resolute;

#[derive(Clone, Debug)]
pub(crate) struct ChapterMetadata {
    pub(crate) updated_at: String,
    pub(crate) number: String,
    pub(crate) id: String,
}

impl ChapterMetadata {
    pub(crate) fn new(number: &str, updated_at: &str, id: &str) -> ChapterMetadata {
        ChapterMetadata {
            updated_at: updated_at.to_owned(),
            number: number.to_owned(),
            id: id.to_owned(),
        }
    }

    pub(crate) fn json(&self) -> HashMap<String, String> {
        let mut json = HashMap::new();
        json.insert("number".to_owned(), self.number.clone());
        json.insert("updatedAt".to_owned(), self.updated_at.clone());
        json.insert("id".to_owned(), self.id.clone());
        json
    }

    pub(crate) fn value(&self) -> Value {
        json!({
            "number": self.number.clone(),
            "updatedAt": self.updated_at.clone(),
            "id": self.id.clone(),
        })
    }
}

impl std::fmt::Display for ChapterMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\"number\": {}, \"updatedAt\": {}, \"id\": {}",
            self.number,
            self.updated_at,
            self.id
        )
    }
}
#[derive(Clone, Debug)]
pub(crate) struct TagMetadata {
    pub(crate) name: String,
    pub(crate) id: String,
}

impl TagMetadata {
    pub(crate) fn new(name: &str, id: &str) -> TagMetadata {
        TagMetadata {
            name: name.to_owned(),
            id: id.to_owned(),
        }
    }

    pub(crate) fn json(&self) -> HashMap<String, String> {
        let mut json = HashMap::new();
        json.insert("name".to_owned(), self.name.clone());
        json.insert("id".to_owned(), self.id.clone());
        json
    }
}

impl std::fmt::Display for TagMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"name\": {} \"id\": {}", self.name, self.id)
    }
}
#[derive(Clone, Debug)]
pub(crate) struct LOG {
    pub(crate) handle_id: String,
    pub(crate) message: String,
    pub(crate) time: String,
    pub(crate) name: String,
}
impl LOG {
    pub(crate) fn new(message: &str) -> LOG {
        let name = resolute::CURRENT_CHAPTER.lock().clone();
        LOG {
            handle_id: resolute::HANDLE_ID.lock().to_string(),
            message: message.to_owned(),
            time: Utc::now().to_rfc3339(),
            name: name,
        }
    }

    pub(crate) fn new_with_name(message: &str, name: &str) -> LOG {
        LOG {
            handle_id: resolute::HANDLE_ID.lock().to_string(),
            message: message.to_owned(),
            time: Utc::now().to_rfc3339(),
            name: name.to_string(),
        }
    }
    pub(crate) fn new_with_handle_id(message: &str, handle_id: Box<str>) -> LOG {
        LOG {
            handle_id: handle_id.into_string(),
            message: message.to_owned(),
            time: Utc::now().to_rfc3339(),
            name: resolute::CURRENT_CHAPTER.lock().clone(),
        }
    }
}

pub(crate) struct MaxPoints {
    pub(crate) max_x: u32,
    pub(crate) max_y: u32,
}
