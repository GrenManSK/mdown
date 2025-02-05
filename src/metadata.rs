use chrono::Utc;
use serde::{ Deserialize, Serialize };
use std::collections::BTreeMap;

use crate::resolute;

/// Represents the settings for the application, such as folder paths, status flags, and optional features.
///
/// This struct holds configuration options for the application, including the folder path for saving files,
/// whether to display status updates, backup preferences, and an optional music-related setting (enabled through
/// the "music" feature flag).
///
/// # Fields
/// - `folder`: A `String` representing the folder path where files are saved.
/// - `stat`: A `bool` indicating whether to display status updates. Defaults to `false` if not set.
/// - `backup`: A `bool` indicating whether to enable backup functionality. Defaults to `false` if not set.
/// - `music`: An optional setting that is only included when the "music" feature is enabled. It holds an `Option<String>`
///   which may represent a music-related configuration or path.
///
/// # Notes
/// - The `music` field is only available if the `music` feature is enabled during compilation.
/// - Don't forget to update `utils::show_settings` to reflect any changes made to the settings.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Settings {
    /// The folder path for saving files.
    pub(crate) folder: String,

    /// Whether to save status updates.
    pub(crate) stat: bool,

    /// Whether to enable backup functionality.
    pub(crate) backup: bool,

    /// An optional music setting, available only when the "music" feature is enabled.
    #[cfg(feature = "music")]
    pub(crate) music: Option<Option<String>>,
}

/// Contains metadata for a specific manga chapter.
///
/// This struct holds essential information about a manga chapter, including the
/// last updated timestamp, chapter number, and a unique identifier.
///
/// # Fields
/// - `updated_at`: A `String` representing the timestamp when the chapter was last updated.
/// - `number`: A `String` representing the chapter's number or identifier, typically used for ordering chapters.
/// - `id`: A `String` representing a unique identifier for the chapter.
///
/// # Notes
/// This struct is typically used to manage and track chapter metadata within a manga's lifecycle.
/// It provides information that can be used for sorting, updating, or identifying specific chapters.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterMetadata {
    /// The timestamp when the chapter was last updated in mangadex api.
    pub(crate) updated_at: String,

    /// The chapter's number or identifier.
    pub(crate) number: String,

    /// A unique identifier for the chapter.
    pub(crate) id: String,
}

#[cfg(feature = "gui")]
impl ChapterMetadata {
    /// Parses the chapter number into a vector of integers.
    ///
    /// This method splits the chapter number (e.g., "1.2.3") by periods and tries to parse each segment
    /// into an integer. It returns a `Vec<i32>` containing the parsed integers. If parsing fails for any segment,
    /// it will be skipped.
    ///
    /// # Returns
    ///
    /// A `Vec<i32>` representing the parsed chapter number components.
    pub(crate) fn parse_number(&self) -> Vec<i32> {
        self.number
            .split('.')
            .filter_map(|part| part.parse().ok())
            .collect()
    }

    /// Retrieves the next chapter from a list of chapters.
    ///
    /// This method finds the index of the current chapter in the provided list and attempts to return the next chapter.
    /// If there is no next chapter (i.e., this is the last chapter), it returns `None`.
    ///
    /// # Parameters
    ///
    /// - `chapters: &[ChapterMetadata]`: A slice of `ChapterMetadata` representing the list of chapters.
    ///
    /// # Returns
    ///
    /// An `Option<&ChapterMetadata>`, where `Some(chapter)` is the next chapter, or `None` if this is the last chapter.
    pub(crate) fn get_next_chapter<'a>(
        &self,
        chapters: &'a [ChapterMetadata]
    ) -> Option<&'a ChapterMetadata> {
        // Find the index of the current chapter
        let current_index = chapters.iter().position(|x| x.id == self.id)?;

        // Check for the next chapter
        if current_index + 1 < chapters.len() {
            Some(&chapters[current_index + 1])
        } else {
            None
        }
    }

    /// Retrieves the previous chapter from a list of chapters.
    ///
    /// This method finds the index of the current chapter in the provided list and attempts to return the previous chapter.
    /// If there is no previous chapter (i.e., this is the first chapter), it returns `None`.
    ///
    /// # Parameters
    ///
    /// - `chapters: &[ChapterMetadata]`: A slice of `ChapterMetadata` representing the list of chapters.
    ///
    /// # Returns
    ///
    /// An `Option<&ChapterMetadata>`, where `Some(chapter)` is the previous chapter, or `None` if this is the first chapter.
    pub(crate) fn get_previous_chapter<'a>(
        &self,
        chapters: &'a [ChapterMetadata]
    ) -> Option<&'a ChapterMetadata> {
        // Find the index of the current chapter
        let current_index = chapters.iter().position(|x| x.id == self.id)?;

        // Check for the previous chapter
        if current_index > 0 {
            Some(&chapters[current_index - 1])
        } else {
            None
        }
    }
}

impl ChapterMetadata {
    /// Creates a new instance of `ChapterMetadata`.
    ///
    /// This method is used to instantiate a `ChapterMetadata` struct with the specified chapter number,
    /// updated timestamp, and unique chapter ID.
    ///
    /// # Parameters
    ///
    /// - `number: &str`: The chapter number, typically a string such as "1.2.3".
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
///
/// This struct holds detailed metadata about a manga chapter, including the chapter's name, ID,
/// associated manga ID, and scanlation information. It is used for generating chapter information,
/// particularly when working with manga downloads and related data.
///
/// # Fields
/// - `name`: A `String` representing the name of the chapter.
/// - `id`: A `String` representing the unique identifier for the chapter.
/// - `manga_id`: A `String` representing the unique identifier for the associated manga.
/// - `saver`: A `bool` indicating whether the chapter is marked for saving. Defaults to `false`.
/// - `title`: A `String` representing the title of the chapter.
/// - `pages`: A `String` representing the number of pages in the chapter.
/// - `chapter`: A `String` representing the chapter number or identifier.
/// - `volume`: A `String` representing the volume number of the chapter.
/// - `scanlation`: A `ScanlationMetadata` struct containing metadata related to the scanlation group or process.
///
/// # Notes
/// This struct is particularly useful for managing chapter-related metadata during manga downloads and for generating
/// chapter information to be displayed or saved.
/// The `scanlation` field provides additional details about the scanlation group or source, and is crucial for tracking
/// the source of the chapter.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub(crate) struct ChapterMetadataIn {
    /// The name of the chapter.
    pub(crate) name: String,

    /// A unique identifier for the chapter.
    pub(crate) id: String,

    /// A unique identifier for the associated manga.
    pub(crate) manga_id: String,

    /// Indicates whether the chapter is marked for saving.
    pub(crate) saver: bool,

    /// The title of the chapter.
    pub(crate) title: String,

    /// The number of pages in the chapter.
    pub(crate) pages: String,

    /// The chapter number or identifier.
    pub(crate) chapter: String,

    /// The volume number of the chapter.
    pub(crate) volume: String,

    /// Metadata about the scanlation group or process.
    pub(crate) scanlation: ScanlationMetadata,
}

/// Contains metadata about the scanlation group.
///
/// This struct holds information about the scanlation group responsible for the translation and
/// publication of a manga chapter. It includes the name of the group and the group's website for
/// easy reference.
///
/// # Fields
/// - `name`: A `String` representing the name of the scanlation group.
/// - `website`: A `String` representing the website of the scanlation group, if available.
///
/// # Notes
/// The `ScanlationMetadata` struct is used to track the source of the translation and publication
/// for a manga chapter. This information is helpful for acknowledging scanlation groups and providing
/// links to their websites for further reference.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub(crate) struct ScanlationMetadata {
    /// The name of the scanlation group.
    pub(crate) name: String,

    /// The website of the scanlation group.
    pub(crate) website: String,
}

/// Contains tag metadata, typically for genres or themes.
///
/// This struct holds information about a tag, which is commonly used to categorize or label manga
/// based on genres, themes, or other characteristics. Tags are often used for filtering or sorting
/// manga in a collection.
///
/// # Fields
/// - `name`: A `String` representing the name of the tag (e.g., "Action", "Romance").
/// - `id`: A `String` representing the unique identifier for the tag.
///
/// # Notes
/// Tags are useful for organizing manga based on specific genres or themes, and this struct is often used
/// in systems that categorize manga for easier discovery or filtering.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct TagMetadata {
    /// The name of the tag.
    pub(crate) name: String,

    /// A unique identifier for the tag.
    pub(crate) id: String,
}

impl TagMetadata {
    /// Creates a new instance of `TagMetadata`.
    ///
    /// This method is used to instantiate a `TagMetadata` struct with the specified tag name and ID.
    ///
    /// # Parameters
    ///
    /// - `name: &str`: The name of the tag, typically a genre or theme (e.g., "Action").
    /// - `id: &str`: The unique identifier for the tag.
    ///
    /// # Returns
    ///
    /// A new `TagMetadata` instance with the provided values.
    #[inline]
    pub(crate) fn new(name: &str, id: &str) -> TagMetadata {
        TagMetadata {
            name: name.to_owned(),
            id: id.to_owned(),
        }
    }
}

/// Represents a log file for the application.
///
/// This struct holds metadata for a log file generated by the application. It includes details like
/// the log file ID, the logs themselves (organized in a `BTreeMap`), and timestamps for when the
/// log file started and ended. It also contains information about the log type and the name of the log.
///
/// # Fields
/// - `id`: A `String` representing the unique identifier for the log file.
/// - `logs`: A `BTreeMap<String, Vec<String>>` where each entry corresponds to a log category (key) and
///   a list of log messages (value).
/// - `mwd`: A `String` representing the working directory or path associated with the log.
/// - `name`: A `String` representing the name of the log file or session.
/// - `time_end`: A `String` representing the timestamp when the logging session ended.
/// - `time_start`: A `String` representing the timestamp when the logging session started.
/// - `r#type`: A `String` representing the type or category of the log (e.g., "web", "downloader").
///
/// # Notes
/// This struct is used to store and manage log metadata, and is typically used for troubleshooting
/// and tracking the progress or issues during the application's execution.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct LogsMetadata {
    /// The unique identifier for the log file.
    pub(crate) id: String,

    /// A map of logs, categorized by type, with each category containing a list of log messages.
    pub(crate) logs: BTreeMap<String, Vec<String>>,

    /// The working directory or path associated with the log file.
    pub(crate) mwd: String,

    /// The name of the log file or session.
    pub(crate) name: String,

    /// The timestamp when the logging session ended.
    pub(crate) time_end: String,

    /// The timestamp when the logging session started.
    pub(crate) time_start: String,

    /// The type or category of the log (e.g., "error", "info", "debug").
    pub(crate) r#type: String,
}

impl LogsMetadata {
    /// Creates a new instance of `LogsMetadata`.
    ///
    /// This method is used to instantiate a `LogsMetadata` struct with the specified values for the log file's
    /// metadata, including ID, logs, timestamps, and type.
    ///
    /// # Parameters
    ///
    /// - `id: &str`: The unique identifier for the log file.
    /// - `logs: BTreeMap<String, Vec<String>>`: A map of log categories (key) and their corresponding log messages (value).
    /// - `mwd: &str`: The working directory or path associated with the log.
    /// - `name: &str`: The name of the log file or session.
    /// - `time_end: &str`: The timestamp when the logging session ended.
    /// - `time_start: &str`: The timestamp when the logging session started.
    /// - `r#type: &str`: The type of the log (e.g., "error", "info", "debug").
    ///
    /// # Returns
    ///
    /// A new `LogsMetadata` instance with the provided values.
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
///
/// This struct contains a collection of `LogsMetadata`, categorized by their unique identifiers.
/// It is used for organizing multiple log files, each containing detailed metadata and logs for the application.
///
/// # Fields
/// - `logs`: A `BTreeMap<String, LogsMetadata>` where each key is a unique log ID, and the value is the corresponding `LogsMetadata`
///   instance that holds detailed log information for that specific log session.
///
/// # Notes
/// The `LogMetadata` struct is useful when managing a collection of logs for the application, allowing for easy
/// access and organization of log files. The `serde(default)` attribute ensures that the `logs` field defaults
/// to an empty map if no value is provided, preventing the struct from being in an incomplete state.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct LogMetadata {
    /// A map of log identifiers to corresponding log metadata.
    #[serde(default)]
    pub(crate) logs: BTreeMap<String, LogsMetadata>,
}

/// Represents a log entry for the application.
///
/// This struct holds a single log entry containing details such as a unique handle ID, the log message,
/// a timestamp for when the log was created, and the associated name (e.g., the current chapter or process).
/// Log entries are useful for tracking application events, errors, or other significant occurrences.
///
/// # Fields
/// - `handle_id`: A `String` representing a unique handle ID for the log entry (e.g., related to the current process or thread).
/// - `message`: A `String` containing the log message, typically describing the event or error being logged.
/// - `time`: A `String` containing the timestamp (in RFC 3339 format) when the log entry was created.
/// - `name`: A `String` representing the name associated with the log entry, such as the current chapter or task name.
///
/// # Notes
/// This struct is used to create log entries that can be stored, displayed, or processed for tracking application activities.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct Log {
    /// The unique handle ID for the log entry.
    pub(crate) handle_id: String,

    /// The message describing the log entry.
    pub(crate) message: String,

    /// The timestamp when the log entry was created.
    pub(crate) time: String,

    /// The name associated with the log entry, such as the current chapter.
    pub(crate) name: String,
}

impl Log {
    /// Creates a new log entry with a message and the current time.
    ///
    /// This method generates a new `Log` instance using the provided message, with the current time and
    /// the current chapter name. The handle ID is fetched from a shared resource (if available).
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
    /// This method generates a new `Log` instance using the provided message and custom name, with the current time.
    /// The handle ID is fetched from a shared resource (if available).
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
    /// This method generates a new `Log` instance using the provided message and custom handle ID, with the current time.
    /// The current chapter name is fetched from a shared resource.
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
///
/// This struct holds a collection of `DBItem` objects, representing individual items in the database.
/// It provides a way to store, manage, and manipulate a list of items.
///
/// # Fields
/// - `files`: A `Vec<DBItem>` containing the database items. Each `DBItem` holds information about an individual
///   item in the database, and the list allows for easy manipulation of the data stored within.
///
/// # Notes
/// This struct serves as a container for the items in the database. It can be used to query, update, and
/// manage the collection of items efficiently.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct DB {
    /// A vector containing the database items.
    pub(crate) files: Vec<DBItem>,
}

/// Represents an item in the database.
///
/// This struct holds information about a single item within the database, including its type,
/// associated URL, name, database-specific name, DMCA status, and any dependencies it may have.
/// It provides a way to organize and access metadata related to individual database items.
///
/// # Fields
/// - `r#type`: A `String` representing the type of the item (e.g., manga, chapter, etc.).
/// - `url`: A `String` containing the URL associated with the item, typically pointing to its location or source.
/// - `name`: A `String` representing the display name of the item.
/// - `db_name`: A `String` used as the name for the item in the database, often serving as a unique identifier.
/// - `dmca`: A `String` representing the DMCA status or related information for the item.
/// - `dependencies`: A `Vec<String>` containing the dependencies of the item, where each dependency is represented by its URL or identifier.
///
/// # Notes
/// This struct is useful for representing an individual entry in the database, allowing for the storage and
/// retrieval of various attributes associated with items. It can be used for managing relationships
/// between items, checking DMCA status, and keeping track of dependencies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct DBItem {
    /// The type of the item (e.g., manga, chapter).
    pub(crate) r#type: String,

    /// The URL associated with the item.
    pub(crate) url: String,

    /// The display name of the item.
    pub(crate) name: String,

    /// The database name of the item, often used as a unique identifier.
    pub(crate) db_name: String,

    /// The DMCA status or related information of the item.
    pub(crate) dmca: String,

    /// A list of dependencies for the item, represented as URLs or identifiers.
    pub(crate) dependencies: Vec<String>,
}

/// Contains data about manga, including metadata and version.
///
/// This struct is used to hold a collection of manga-related metadata along with versioning information.
/// It allows for storing multiple `MangaMetadata` entries and provides version control to track the state of the data.
///
/// # Fields
/// - `data`: A `Vec<MangaMetadata>` containing the metadata for multiple manga entries. Each `MangaMetadata` holds details about a specific manga, such as its title, author, and other relevant information.
/// - `version`: A `String` representing the version of the data, which is useful for tracking updates, migrations, or changes to the manga data over time.
///
/// # Notes
/// This struct helps manage collections of manga metadata, making it easy to update, query, and maintain the metadata.
/// The `version` field ensures that any changes to the data structure can be tracked, allowing for proper handling of data updates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct Dat {
    /// A list of manga metadata entries.
    pub(crate) data: Vec<MangaMetadata>,

    /// The version of the manga data.
    pub(crate) version: String,
}

/// Contains metadata for manga, including chapters and tags.
///
/// This struct holds detailed information about a specific manga, such as its name, unique ID, chapters, tags, and additional metadata
/// like available languages, theme, genre, and links. It is designed to store and manage various attributes related to a manga entry.
///
/// # Fields
/// - `name`: A `String` representing the title or name of the manga.
/// - `id`: A `String` that uniquely identifies the manga in the database or system.
/// - `chapters`: A `Vec<ChapterMetadata>` containing the metadata for all the chapters of the manga.
/// - `mwd`: A `String` used for some internal or external identification of the manga.
/// - `cover`: A `bool` indicating whether the manga has a cover image.
/// - `date`: A `Vec<String>` containing various dates associated with the manga (e.g., release dates, last updated, etc.).
/// - `available_languages`: A `Vec<String>` listing all languages the manga is available in.
/// - `current_language`: A `String` representing the language currently being used or preferred for the manga.
/// - `theme`: A `Vec<TagMetadata>` representing the themes of the manga (e.g., drama, comedy, etc.).
/// - `genre`: A `Vec<TagMetadata>` representing the genres of the manga (e.g., action, romance, etc.).
/// - `links`: A `LinksMetadata` struct that contains various URLs or external links related to the manga.
///
/// # Notes
/// This struct is essential for representing all metadata related to a specific manga, including its chapters, themes, genres,
/// languages, and external links. It provides a comprehensive way to manage the manga's information and makes it easy to query or update data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct MangaMetadata {
    /// The title or name of the manga.
    pub(crate) name: String,

    /// The unique identifier for the manga.
    pub(crate) id: String,

    /// A list of chapters associated with the manga.
    pub(crate) chapters: Vec<ChapterMetadata>,

    /// An identifier used for the manga (e.g., some external or internal identifier).
    pub(crate) mwd: String,

    /// A flag indicating whether the manga has a cover image.
    pub(crate) cover: bool,

    /// A list of dates associated with the manga.
    pub(crate) date: Vec<String>,

    /// A list of available languages for the manga.
    pub(crate) available_languages: Vec<String>,

    /// The current language being used for the manga.
    pub(crate) current_language: String,

    /// A list of tags representing the themes of the manga.
    pub(crate) theme: Vec<TagMetadata>,

    /// A list of tags representing the genres of the manga.
    pub(crate) genre: Vec<TagMetadata>,

    /// Links and external resources related to the manga.
    #[serde(default)]
    pub(crate) links: LinksMetadata,
}

/// Contains metadata for links.
///
/// This struct holds various external URLs or links related to a manga, such as links to its official pages,
/// scanlation sources, or platforms like MyAnimeList (MAL), AniList (AL), and others. It provides a convenient way to
/// manage and store multiple URLs associated with a manga.
///
/// # Fields
/// - `al`: An optional `String` containing the AniList (AL) URL for the manga.
/// - `mal`: An optional `String` containing the MyAnimeList (MAL) URL for the manga.
/// - `amz`: An optional `String` containing the Amazon (AMZ) URL for the manga.
/// - `ebj`: An optional `String` containing the ebookJapan (EBJ) URL for the manga.
/// - `cdj`: An optional `String` containing the CDJapan (CDJ) URL for the manga.
/// - `raw`: An optional `String` containing the raw source URL for the manga.
/// - `engtl`: An optional `String` containing the English-translated version's URL for the manga.
/// - `mu`: An optional `String` containing the MangaUpdates (MU) URL for the manga.
/// - `nu`: An optional `String` containing the NovelUpdates (NU) URL for the manga.
///
/// # Notes
/// This struct is used to store various external links related to the manga. The links can represent official pages,
/// fan-translated versions, or other related platforms. The optional nature of each field means that some links may not
/// be available for all mangas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub(crate) struct LinksMetadata {
    /// The AniList (AL) URL for the manga.
    pub(crate) al: Option<String>,

    /// The MyAnimeList (MAL) URL for the manga.
    pub(crate) mal: Option<String>,

    /// The Amazon (AMZ) URL for the manga.
    pub(crate) amz: Option<String>,

    /// The ebookJapan (EBJ) URL for the manga.
    pub(crate) ebj: Option<String>,

    /// The CDJapan (CDJ) URL for the manga.
    pub(crate) cdj: Option<String>,

    /// The raw source URL for the manga.
    pub(crate) raw: Option<String>,

    /// The English-translated version's URL for the manga.
    pub(crate) engtl: Option<String>,

    /// The MangaUpdates (MU) URL for the manga.
    pub(crate) mu: Option<String>,

    /// The NovelUpdates (NU) URL for the manga.
    pub(crate) nu: Option<String>,
}

/// Defines the maximum coordinates for points.
///
/// This struct is used to represent the maximum values for the x and y coordinates. It is useful for defining boundaries
/// or limits within window.
///
/// # Fields
/// - `max_x`: A `u32` representing the maximum value for the x-coordinate.
/// - `max_y`: A `u32` representing the maximum value for the y-coordinate.
///
/// # Notes
/// The `MaxPoints` struct allows you to define upper bounds for coordinates in window.
#[derive(Debug, Clone)]
pub(crate) struct MaxPoints {
    /// The maximum value for the x-coordinate.
    pub(crate) max_x: u32,

    /// The maximum value for the y-coordinate.
    pub(crate) max_y: u32,
}

/// Represents the response from a manga API request.
///
/// This struct holds the response data from an API call to a manga service. It includes the result of the request,
/// the response type, metadata for pagination, and the data itself, which typically contains information about manga chapters.
///
/// # Fields
/// - `result`: A `String` indicating the result of the request (e.g., "success", "error").
/// - `response`: A `String` indicating the response type (e.g., "ok", "error").
/// - `data`: A `Vec<ChapterResponse>` containing the chapters returned by the API.
/// - `limit`: A `u64` specifying the maximum number of results per request (pagination limit).
/// - `offset`: A `u64` representing the starting point for the results (pagination offset).
/// - `total`: A `u64` indicating the total number of available results for the query.
///
/// # Notes
/// The `MangaResponse` struct is used to represent the JSON response from a manga API. It supports pagination, with fields
/// like `limit`, `offset`, and `total` allowing clients to handle multiple pages of results efficiently. The `data` field
/// typically contains the detailed information about manga chapters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct MangaResponse {
    pub(crate) result: String,
    pub(crate) response: String,

    /// The chapters returned by the API.
    pub(crate) data: Vec<ChapterResponse>,

    /// The maximum number of results per request (pagination limit).
    pub(crate) limit: u64,

    /// The starting point for the results (pagination offset).
    pub(crate) offset: u64,

    /// The total number of available results for the query.
    pub(crate) total: u64,
}

/// Contains information about a specific chapter in the API response.
///
/// This struct represents the data related to a specific manga chapter returned by the API. It includes the chapter's
/// unique ID, type, attributes, and relationships with other resources like manga or scanlation groups.
///
/// # Fields
/// - `id`: A `String` representing the unique identifier for the chapter.
/// - `type`: A `String` indicating the type of the chapter (e.g., "chapter", "volume").
/// - `attributes`: A `ChapterAttrResponse` containing the attributes related to the chapter (e.g., title, language, date of release).
/// - `relationships`: A `Vec<ChapterRelResponse>` representing relationships with other entities like manga, scanlation groups, or other chapters.
///
/// # Notes
/// The `ChapterResponse` struct is used to store detailed information about a manga chapter as returned by the API. The
/// `attributes` field contains metadata for the chapter itself, while the `relationships` field details how this chapter
/// is related to other entities, providing a broader context for the chapter’s place in the manga ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterResponse {
    /// The unique identifier for the chapter.
    pub(crate) id: String,
    pub(crate) r#type: String,

    /// The attributes related to the chapter (e.g., title, language, release date).
    pub(crate) attributes: ChapterAttrResponse,

    /// The relationships with other entities like manga or scanlation groups.
    pub(crate) relationships: Vec<ChapterRelResponse>,
}

/// Contains relationship information for chapters.
///
/// This struct represents a relationship between a chapter and another entity, such as the manga it belongs to,
/// the scanlation group, or other related chapters. It includes the unique identifier and type of the related entity.
///
/// # Fields
/// - `id`: A `String` representing the unique identifier of the related entity (e.g., manga series ID, scanlation group ID).
/// - `type`: A `String` indicating the type of relationship (e.g., "manga", "scanlation_group").
///
/// # Notes
/// The `ChapterRelResponse` struct is used to represent relationships between a chapter and other entities in the
/// manga ecosystem. This can include the manga series to which the chapter belongs, scanlation groups involved in its
/// production, or any other relevant connections.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterRelResponse {
    /// The unique identifier of the related entity (e.g., manga series, scanlation group).
    pub(crate) id: String,

    /// The type of relationship (e.g., "manga", "scanlation_group").
    pub(crate) r#type: String,
}

/// Contains attributes for chapters in the API response.
///
/// This struct represents various attributes related to a specific chapter of a manga, including metadata such as
/// chapter number, title, language, timestamps for creation and updates, and the number of pages. It is typically
/// used to store the detailed information for each chapter returned in an API response.
///
/// # Fields
/// - `volume`: An optional `String` representing the volume of the chapter, if applicable.
/// - `chapter`: An optional `String` indicating the chapter number.
/// - `title`: An optional `String` that contains the title of the chapter.
/// - `translatedLanguage`: An optional `String` representing the language in which the chapter was translated.
/// - `externalUrl`: An optional `String` providing a link to an external page related to the chapter (e.g., a reader page).
/// - `publishAt`: A `String` representing the timestamp when the chapter was published.
/// - `readableAt`: A `String` representing the timestamp when the chapter became readable (e.g., unlocked or available).
/// - `createdAt`: A `String` representing the timestamp when the chapter was created in the database or system.
/// - `updatedAt`: A `String` representing the timestamp when the chapter was last updated.
/// - `pages`: A `u64` representing the number of pages in the chapter.
/// - `version`: A `u64` representing the version of the chapter (for tracking updates or revisions).
///
/// # Notes
/// The `ChapterAttrResponse` struct is used to store detailed metadata for a manga chapter, as provided in the API response.
/// It includes both optional and required fields that describe the chapter's content, timestamps, and additional resources.
/// This data is essential for displaying chapter details and managing chapters in a manga system or application.
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterAttrResponse {
    /// The volume of the chapter, if applicable.
    pub(crate) volume: Option<String>,

    /// The chapter number.
    pub(crate) chapter: Option<String>,

    /// The title of the chapter.
    pub(crate) title: Option<String>,

    /// The language in which the chapter was translated.
    pub(crate) translatedLanguage: Option<String>,

    /// A URL to an external page related to the chapter (e.g., a reader link).
    pub(crate) externalUrl: Option<String>,

    /// The timestamp when the chapter was published.
    pub(crate) publishAt: String,

    /// The timestamp when the chapter became readable.
    pub(crate) readableAt: String,

    /// The timestamp when the chapter was created.
    pub(crate) createdAt: String,

    /// The timestamp when the chapter was last updated.
    pub(crate) updatedAt: String,

    /// The number of pages in the chapter.
    pub(crate) pages: u64,

    /// The version of the chapter (for tracking updates).
    pub(crate) version: u64,
}

/// Contains data about a chapter, including image URLs.
///
/// This struct represents the data related to a specific manga chapter, primarily focusing on the chapter's image URLs.
/// It includes the result status of the API request, the base URL for the images, and the specific images related to the chapter.
///
/// # Fields
/// - `result`: A `String` indicating the result of the API request (e.g., "success", "error").
/// - `baseUrl`: A `String` representing the base URL where the chapter's images are hosted.
/// - `chapter`: A `ChapterDataImages` instance containing the list of images related to the chapter.
///
/// # Notes
/// The `ChapterData` struct is used to represent the data returned from an API for a specific chapter, including the
/// image URLs needed to display the chapter’s content. It is particularly useful for manga readers or applications that
/// need to load chapter images dynamically from the internet.
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterData {
    /// The result of the API request (e.g., "success", "error").
    pub(crate) result: String,

    /// The base URL where the chapter's images are hosted.
    pub(crate) baseUrl: String,

    /// The images for the chapter, represented by a `ChapterDataImages` instance.
    pub(crate) chapter: ChapterDataImages,
}

/// Contains image data for a chapter.
///
/// This struct holds the image data for a specific chapter, including the image URLs and, optionally, data saver URLs.
/// It is used to manage and retrieve the images for a chapter in a manga, including both full-resolution and optimized versions for data saving.
///
/// # Fields
/// - `hash`: A `String` representing a unique hash associated with the chapter's images. This can be used to verify or cache the image data.
/// - `data`: A `Vec<String>` containing the URLs for the full-resolution images of the chapter.
/// - `dataSaver`: An optional `Vec<String>` containing the URLs for the data saver (lower-resolution) versions of the images, if available.
///
/// # Notes
/// The `ChapterDataImages` struct is used to represent the image data associated with a chapter. The `data` field contains
/// the primary image URLs, while the `dataSaver` field can be used to provide lower-resolution versions of the images, useful
/// for saving bandwidth or improving performance on slower connections.
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ChapterDataImages {
    /// The hash representing a unique identifier for the chapter's images.
    pub(crate) hash: String,

    /// A vector of strings representing the URLs for the full-resolution images of the chapter.
    pub(crate) data: Vec<String>,

    /// An optional vector of strings representing the URLs for lower-resolution images (data saver versions).
    pub(crate) dataSaver: Option<Vec<String>>,
}

/// Contains statistics for a manga.
///
/// This struct holds various statistics related to a manga, such as the number of comments, the rating, and the number of followers.
/// It is useful for displaying analytics or engagement data related to a specific manga title.
///
/// # Fields
/// - `comments`: An optional `Comment` instance that holds details about the comments for the manga. It could be `None` if there are no comments or the comments are unavailable.
/// - `rating`: A `Rating` instance representing the rating of the manga. This field holds information such as the average rating and the number of ratings given.
/// - `follows`: A `u64` representing the number of users who are following the manga.
///
/// # Notes
/// The `Statistics` struct is used to aggregate various engagement and popularity metrics for a manga. It includes user feedback
/// through comments, a rating system, and a follower count to track the manga's reception among readers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct Statistics {
    /// An optional comment instance that holds details about the manga's comments.
    pub(crate) comments: Option<Comment>,

    /// The rating of the manga, represented by a `Rating` instance.
    pub(crate) rating: Rating,

    /// The number of users following the manga.
    pub(crate) follows: u64,
}

/// Contains comment statistics.
///
/// This struct holds data related to the comments on a manga, including the comment thread ID and the number of replies.
/// It is useful for tracking the engagement in comment threads for a particular manga.
///
/// # Fields
/// - `threadId`: A `u64` representing the unique ID of the comment thread for the manga.
/// - `repliesCount`: A `u64` indicating the number of replies to the comment thread.
///
/// # Notes
/// The `Comment` struct is designed to provide insight into the activity within a manga's comment section. It tracks the
/// thread's identifier and how many replies it has received, which can be useful for engagement analysis and user interaction
/// tracking within an application.
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct Comment {
    /// The unique ID of the comment thread for the manga.
    pub(crate) threadId: u64,

    /// The number of replies to the comment thread.
    pub(crate) repliesCount: u64,
}

/// Contains rating information for a manga.
///
/// This struct holds the rating information for a manga, including the average rating, the Bayesian rating, and the distribution
/// of ratings. It provides a comprehensive overview of how the manga is rated by users.
///
/// # Fields
/// - `average`: A `f64` representing the average rating of the manga. This is calculated from all user ratings and reflects
///   the general opinion of the manga.
/// - `bayesian`: A `f64` representing the Bayesian rating of the manga. This rating method accounts for the number of ratings
///   and adjusts the score based on the volume of user feedback, helping to prevent skewed ratings from a small number of users.
/// - `distribution`: A `RatingDistribution` instance that contains a breakdown of how many ratings the manga has received in each
///   rating tier (e.g., how many 1-star, 2-star, etc., ratings the manga has).
///
/// # Notes
/// The `Rating` struct aggregates various aspects of user feedback. The average rating reflects the overall opinion of the manga,
/// while the Bayesian rating helps provide a more reliable score by factoring in the number of ratings. The distribution field allows
/// for a deeper understanding of how ratings are spread across different levels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct Rating {
    /// The average rating of the manga.
    pub(crate) average: f64,

    /// The Bayesian rating of the manga.
    pub(crate) bayesian: f64,

    /// The distribution of ratings across different rating levels.
    pub(crate) distribution: RatingDistribution,
}

/// Contains the distribution of ratings.
///
/// This struct holds data about how many users rated the manga in each rating tier, from 1 to 10 stars.
/// It provides a detailed breakdown of the rating scores, offering insight into how users have rated the manga across different levels.
///
/// # Fields
/// - `one`: A `u64` representing the number of 1-star ratings the manga has received.
/// - `two`: A `u64` representing the number of 2-star ratings the manga has received.
/// - `three`: A `u64` representing the number of 3-star ratings the manga has received.
/// - `four`: A `u64` representing the number of 4-star ratings the manga has received.
/// - `five`: A `u64` representing the number of 5-star ratings the manga has received.
/// - `six`: A `u64` representing the number of 6-star ratings the manga has received.
/// - `seven`: A `u64` representing the number of 7-star ratings the manga has received.
/// - `eight`: A `u64` representing the number of 8-star ratings the manga has received.
/// - `nine`: A `u64` representing the number of 9-star ratings the manga has received.
/// - `ten`: A `u64` representing the number of 10-star ratings the manga has received.
///
/// # Notes
/// The `RatingDistribution` struct allows for a detailed breakdown of the ratings a manga receives. Each field represents how many
/// users gave a specific rating (from 1 to 10). This can be used to understand the distribution of opinions among users and track
/// how ratings are spread out across different levels of satisfaction with the manga.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct RatingDistribution {
    /// The number of 1-star ratings.
    #[serde(rename = "1")]
    pub(crate) one: u64,

    /// The number of 2-star ratings.
    #[serde(rename = "2")]
    pub(crate) two: u64,

    /// The number of 3-star ratings.
    #[serde(rename = "3")]
    pub(crate) three: u64,

    /// The number of 4-star ratings.
    #[serde(rename = "4")]
    pub(crate) four: u64,

    /// The number of 5-star ratings.
    #[serde(rename = "5")]
    pub(crate) five: u64,

    /// The number of 6-star ratings.
    #[serde(rename = "6")]
    pub(crate) six: u64,

    /// The number of 7-star ratings.
    #[serde(rename = "7")]
    pub(crate) seven: u64,

    /// The number of 8-star ratings.
    #[serde(rename = "8")]
    pub(crate) eight: u64,

    /// The number of 9-star ratings.
    #[serde(rename = "9")]
    pub(crate) nine: u64,

    /// The number of 10-star ratings.
    #[serde(rename = "10")]
    pub(crate) ten: u64,
}

/// Enum to specify the type of data saving.
///
/// This enum is used to differentiate between two types of data saving methods in the application.
/// It helps determine how and where data is saved, depending on the context in which it is used.
///
/// # Variants
/// - `data`: Represents the standard data saving method.
/// - `dataSaver`: Represents an alternative data saving method that might involve data compression or other optimizations.
///
/// # Usage
/// This enum can be used to specify which type of saving strategy to apply when saving data in the application. For example,
/// it can be passed as an argument to functions or methods that handle data storage, allowing different data-saving behaviors
/// based on the chosen variant.
#[allow(non_camel_case_types)]
pub(crate) enum Saver {
    /// Represents the standard data saving method.
    data,

    /// Represents an alternative data saving method, such as using a data saver technique.
    dataSaver,
}

/// Enum representing the stages of music playback.
///
/// This enum is used to track and represent different stages in the music playback process.
/// It helps in managing the state transitions of music, from initialization to playback completion.
///
/// # Variants
/// - `None`: Represents the absence of music playback or an uninitialized state.
/// - `Init`: Indicates that the music playback is in the initialization phase, typically when the system is preparing to play music.
/// - `Start`: Denotes that the music playback has started and is currently playing.
/// - `End`: Indicates that the music playback has finished or ended.
///
/// # Usage
/// The `MusicStage` enum is used to handle various stages of music playback, typically within applications that involve audio
/// processing or media playback. By transitioning through the different stages, the system can appropriately react to the status
/// of music playback and trigger corresponding actions, such as loading, starting, or stopping the music.
#[cfg(feature = "music")]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MusicStage {
    /// No music playback or uninitialized state.
    None,

    /// Music playback is in the initialization phase.
    Init,

    /// Music playback has started and is ongoing.
    Start,

    /// Music playback has ended.
    End,
}
