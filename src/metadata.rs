use chrono::Utc;
use serde::{ Deserialize, Serialize };
use std::collections::BTreeMap;

use crate::resolute;

/// Represents settings for the application, such as folder paths.
/// Don't forget to change utils::show_settings
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Settings {
    pub(crate) folder: String,
    pub(crate) stat: bool,
    pub(crate) backup: bool,
    #[cfg(feature = "music")]
    pub(crate) music: Option<Option<String>>,
}

/// Contains metadata for a specific manga chapter.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterMetadata {
    pub(crate) updated_at: String,
    pub(crate) number: String,
    pub(crate) id: String,
}

impl ChapterMetadata {
    /// Creates a new instance of `ChapterMetadata`.
    ///
    /// # Parameters
    ///
    /// - `number: &str`: The chapter number.
    /// - `updated_at: &str`: The date and time when the chapter was last updated.
    /// - `id: &str`: The unique identifier for the chapter.
    ///
    /// # Returns
    ///
    /// A `ChapterMetadata` instance with the provided values.
    #[inline]
    pub(crate) fn new(number: &str, updated_at: &str, id: &str) -> ChapterMetadata {
        ChapterMetadata {
            updated_at: updated_at.to_owned(),
            number: number.to_owned(),
            id: id.to_owned(),
        }
    }
}

/// Contains metadata used for generating chapter information.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterMetadataIn {
    pub(crate) name: String,
    pub(crate) id: String,
    pub(crate) manga_id: String,
    pub(crate) saver: bool,
    pub(crate) title: String,
    pub(crate) pages: String,
    pub(crate) chapter: String,
    pub(crate) volume: String,
    pub(crate) scanlation: ScanlationMetadata,
}

/// Contains metadata about the scanlation group.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct ScanlationMetadata {
    pub(crate) name: String,
    pub(crate) website: String,
}

/// Contains tag metadata, typically for genres or themes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct TagMetadata {
    pub(crate) name: String,
    pub(crate) id: String,
}

impl TagMetadata {
    /// Creates a new instance of `TagMetadata`.
    ///
    /// # Parameters
    ///
    /// - `name: &str`: The name of the tag.
    /// - `id: &str`: The unique identifier for the tag.
    ///
    /// # Returns
    ///
    /// A `TagMetadata` instance with the provided values.
    #[inline]
    pub(crate) fn new(name: &str, id: &str) -> TagMetadata {
        TagMetadata {
            name: name.to_owned(),
            id: id.to_owned(),
        }
    }
}

/// Represents a log file for the application.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct LogsMetadata {
    pub(crate) id: String,
    pub(crate) logs: BTreeMap<String, Vec<String>>,
    pub(crate) mwd: String,
    pub(crate) name: String,
    pub(crate) time_end: String,
    pub(crate) time_start: String,
    pub(crate) r#type: String,
}

impl LogsMetadata {
    #[inline]
    pub(crate) fn new(
        id: &str,
        logs: BTreeMap<String, Vec<String>>,
        mwd: &str,
        name: &str,
        time_end: &str,
        time_start: &str,
        r#type: &str
    ) -> LogsMetadata {
        LogsMetadata {
            id: id.to_string(),
            logs,
            mwd: mwd.to_string(),
            name: name.to_string(),
            time_end: time_end.to_string(),
            time_start: time_start.to_string(),
            r#type: r#type.to_string(),
        }
    }
}

/// Represents a log file for the application.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct LogMetadata {
    #[serde(default)]
    pub(crate) logs: BTreeMap<String, LogsMetadata>,
}

/// Represents a log entry for the application.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct Log {
    pub(crate) handle_id: String,
    pub(crate) message: String,
    pub(crate) time: String,
    pub(crate) name: String,
}

impl Log {
    /// Creates a new log entry with a message and the current time.
    ///
    /// # Parameters
    ///
    /// - `message: &str`: The log message.
    ///
    /// # Returns
    ///
    /// A `Log` instance with the current time and provided message.
    pub(crate) fn new(message: &str) -> Log {
        let name = resolute::CURRENT_CHAPTER.lock().clone();
        let handle_id = match resolute::HANDLE_ID.try_lock() {
            Some(handle) => handle.to_string(),
            None => String::new(),
        };
        Log {
            handle_id,
            message: message.to_owned(),
            time: Utc::now().to_rfc3339(),
            name,
        }
    }

    /// Creates a new log entry with a message, a custom name, and the current time.
    ///
    /// # Parameters
    ///
    /// - `message: &str`: The log message.
    /// - `name: &str`: The name associated with the log entry.
    ///
    /// # Returns
    ///
    /// A `Log` instance with the current time, provided message, and name.
    pub(crate) fn new_with_name(message: &str, name: &str) -> Log {
        let handle_id = match resolute::HANDLE_ID.try_lock() {
            Some(handle) => handle.to_string(),
            None => String::new(),
        };
        Log {
            handle_id,
            message: message.to_owned(),
            time: Utc::now().to_rfc3339(),
            name: name.to_string(),
        }
    }

    /// Creates a new log entry with a message, a custom handle ID, and the current time.
    ///
    /// # Parameters
    ///
    /// - `message: &str`: The log message.
    /// - `handle_id: Box<str>`: The handle ID associated with the log entry.
    ///
    /// # Returns
    ///
    /// A `Log` instance with the current time, provided message, and handle ID.
    #[cfg(feature = "web")]
    pub(crate) fn new_with_handle_id(message: &str, handle_id: Box<str>) -> Log {
        Log {
            handle_id: handle_id.into_string(),
            message: message.to_owned(),
            time: Utc::now().to_rfc3339(),
            name: resolute::CURRENT_CHAPTER.lock().clone(),
        }
    }
}

/// Represents a database of items.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct DB {
    pub(crate) files: Vec<DBItem>,
}

/// Represents an item in the database.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct DBItem {
    pub(crate) r#type: String,
    pub(crate) url: String,
    pub(crate) name: String,
    pub(crate) db_name: String,
    pub(crate) dmca: String,
    pub(crate) dependencies: Vec<String>,
}

/// Contains data about manga, including metadata and version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct Dat {
    pub(crate) data: Vec<MangaMetadata>,
    pub(crate) version: String,
}

/// Contains metadata for manga, including chapters and tags.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct MangaMetadata {
    pub(crate) name: String,
    pub(crate) id: String,
    pub(crate) chapters: Vec<ChapterMetadata>,
    pub(crate) mwd: String,
    pub(crate) cover: bool,
    pub(crate) date: Vec<String>,
    pub(crate) available_languages: Vec<String>,
    pub(crate) current_language: String,
    pub(crate) theme: Vec<TagMetadata>,
    pub(crate) genre: Vec<TagMetadata>,
}

/// Defines the maximum coordinates for points.
#[derive(Debug, Clone)]
pub(crate) struct MaxPoints {
    pub(crate) max_x: u32,
    pub(crate) max_y: u32,
}

/// Represents the response from a manga API request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct MangaResponse {
    pub(crate) result: String,
    pub(crate) response: String,
    pub(crate) data: Vec<ChapterResponse>,
    pub(crate) limit: u64,
    pub(crate) offset: u64,
    pub(crate) total: u64,
}

/// Contains information about a specific chapter in the API response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterResponse {
    pub(crate) id: String,
    pub(crate) r#type: String,
    pub(crate) attributes: ChapterAttrResponse,
    pub(crate) relationships: Vec<ChapterRelResponse>,
}

/// Contains relationship information for chapters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterRelResponse {
    pub(crate) id: String,
    pub(crate) r#type: String,
}

/// Contains attributes for chapters in the API response.
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterAttrResponse {
    pub(crate) volume: Option<String>,
    pub(crate) chapter: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) translatedLanguage: Option<String>,
    pub(crate) externalUrl: Option<String>,
    pub(crate) publishAt: String,
    pub(crate) readableAt: String,
    pub(crate) createdAt: String,
    pub(crate) updatedAt: String,
    pub(crate) pages: u64,
    pub(crate) version: u64,
}

/// Contains data about a chapter, including image URLs.
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterData {
    pub(crate) result: String,
    pub(crate) baseUrl: String,
    pub(crate) chapter: ChapterDataImages,
}

/// Contains image data for a chapter.
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterDataImages {
    pub(crate) hash: String,
    pub(crate) data: Vec<String>,
    pub(crate) dataSaver: Option<Vec<String>>,
}

/// Contains statistics for a manga.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct Statistics {
    pub(crate) comments: Comment,
    pub(crate) rating: Rating,
    pub(crate) follows: u64,
}

/// Contains comment statistics.
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct Comment {
    pub(crate) threadId: u64,
    pub(crate) repliesCount: u64,
}

/// Contains rating information for a manga.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct Rating {
    pub(crate) average: f64,
    pub(crate) bayesian: f64,
    pub(crate) distribution: RatingDistribution,
}

/// Contains the distribution of ratings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct RatingDistribution {
    #[serde(rename = "1")]
    pub(crate) one: u64,
    #[serde(rename = "2")]
    pub(crate) two: u64,
    #[serde(rename = "3")]
    pub(crate) three: u64,
    #[serde(rename = "4")]
    pub(crate) four: u64,
    #[serde(rename = "5")]
    pub(crate) five: u64,
    #[serde(rename = "6")]
    pub(crate) six: u64,
    #[serde(rename = "7")]
    pub(crate) seven: u64,
    #[serde(rename = "8")]
    pub(crate) eight: u64,
    #[serde(rename = "9")]
    pub(crate) nine: u64,
    #[serde(rename = "10")]
    pub(crate) ten: u64,
}

#[allow(non_camel_case_types)]
/// Enum to specify the type of data saving.
pub(crate) enum Saver {
    data,
    dataSaver,
}

#[cfg(feature = "music")]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MusicStage {
    None,
    Init,
    Start,
    End,
}
