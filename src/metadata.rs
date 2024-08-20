use chrono::Utc;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;

use crate::resolute;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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
}

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

impl ChapterMetadataIn {
    pub(crate) fn new(
        name: String,
        id: String,
        manga_id: String,
        saver: bool,
        title: String,
        pages: String,
        chapter: String,
        volume: String,
        scanlation: ScanlationMetadata
    ) -> ChapterMetadataIn {
        ChapterMetadataIn {
            name: name,
            id: id,
            manga_id: manga_id,
            saver: saver,
            title: title,
            pages: pages,
            chapter: chapter,
            volume: volume,
            scanlation: scanlation,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct ScanlationMetadata {
    pub(crate) name: String,
    pub(crate) website: String,
}

impl ScanlationMetadata {
    pub(crate) fn new(name: &str, website: &str) -> ScanlationMetadata {
        ScanlationMetadata {
            name: name.to_owned(),
            website: website.to_owned(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct LOG {
    pub(crate) handle_id: String,
    pub(crate) message: String,
    pub(crate) time: String,
    pub(crate) name: String,
}
impl LOG {
    pub(crate) fn new(message: &str) -> LOG {
        let name = resolute::CURRENT_CHAPTER.lock().clone();
        let handle_id = match resolute::HANDLE_ID.try_lock() {
            Some(handle) => handle.to_string(),
            None => String::new(),
        };
        LOG {
            handle_id: handle_id,
            message: message.to_owned(),
            time: Utc::now().to_rfc3339(),
            name: name,
        }
    }

    pub(crate) fn new_with_name(message: &str, name: &str) -> LOG {
        let handle_id = match resolute::HANDLE_ID.try_lock() {
            Some(handle) => handle.to_string(),
            None => String::new(),
        };
        LOG {
            handle_id: handle_id,
            message: message.to_owned(),
            time: Utc::now().to_rfc3339(),
            name: name.to_string(),
        }
    }
    #[cfg(feature = "web")]
    pub(crate) fn new_with_handle_id(message: &str, handle_id: Box<str>) -> LOG {
        LOG {
            handle_id: handle_id.into_string(),
            message: message.to_owned(),
            time: Utc::now().to_rfc3339(),
            name: resolute::CURRENT_CHAPTER.lock().clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct DB {
    pub(crate) files: Vec<DBItem>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct DBItem {
    pub(crate) r#type: String,
    pub(crate) url: String,
    pub(crate) name: String,
    pub(crate) dmca: String,
    pub(crate) dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct DAT {
    pub(crate) data: Vec<MangaMetadata>,
    pub(crate) version: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub(crate) struct MangaDownloadLogs {
    pub(crate) id: String,
    pub(crate) logs: HashMap<String, Vec<String>>,
    pub(crate) mwd: String,
    pub(crate) name: String,
    pub(crate) time_end: String,
    pub(crate) time_start: String,
    pub(crate) r#type: String,
}

pub(crate) type MdownLogs = HashMap<String, MangaDownloadLogs>;

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

impl MangaMetadata {
    pub(crate) fn new(
        name: &str,
        id: &str,
        chapters: Vec<ChapterMetadata>,
        mwd: &str,
        cover: bool,
        date: Vec<String>,
        available_languages: Vec<String>,
        current_language: &str,
        theme: Vec<TagMetadata>,
        genre: Vec<TagMetadata>
    ) -> MangaMetadata {
        MangaMetadata {
            name: name.to_owned(),
            id: id.to_owned(),
            chapters: chapters,
            mwd: mwd.to_owned(),
            cover: cover,
            date: date,
            available_languages: available_languages,
            current_language: current_language.to_owned(),
            theme: theme,
            genre: genre,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MaxPoints {
    pub(crate) max_x: u32,
    pub(crate) max_y: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct MangaResponse {
    pub(crate) result: String,
    pub(crate) response: String,
    pub(crate) data: Vec<ChapterResponse>,
    pub(crate) limit: u64,
    pub(crate) offset: u64,
    pub(crate) total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterResponse {
    pub(crate) id: String,
    pub(crate) r#type: String,
    pub(crate) attributes: ChapterAttrResponse,
    pub(crate) relationships: Vec<ChapterRelResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterRelResponse {
    pub(crate) id: String,
    pub(crate) r#type: String,
}
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
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterData {
    pub(crate) result: String,
    pub(crate) baseUrl: String,
    pub(crate) chapter: ChapterDataImages,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterDataImages {
    pub(crate) hash: String,
    pub(crate) data: Vec<String>,
    pub(crate) dataSaver: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct Statistics {
    pub(crate) comments: Comment,
    pub(crate) rating: Rating,
    pub(crate) follows: u64,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct Comment {
    pub(crate) threadId: u64,
    pub(crate) repliesCount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct Rating {
    pub(crate) average: f64,
    pub(crate) bayesian: f64,
    pub(crate) distribution: RatingDistribution,
}
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
#[derive(Debug, Clone)]
pub(crate) enum Saver {
    data,
    dataSaver,
}
