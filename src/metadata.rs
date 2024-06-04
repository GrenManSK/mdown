use chrono::Utc;
use serde::{ Serialize, Deserialize };

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
pub(crate) struct DAT {
    pub(crate) data: Vec<MangaMetadata>,
    pub(crate) version: String,
}

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
