use lazy_static::lazy_static;
use serde_json::{ json, Map, Value };
use std::{
    collections::HashMap,
    fs::{ self, File, OpenOptions },
    io::{ Read, Write },
    sync::{ Arc, Mutex },
};

use crate::{
    ARGS,
    download,
    download_manga,
    error::{ MdownError, handle_error },
    getter::{ self, get_folder_name, get_manga, get_manga_name, get_scanlation_group },
    log,
    log_end,
    metadata::{ ChapterMetadata, TagMetadata, LOG },
    MAXPOINTS,
    string,
    utils::{ self, clear_screen },
    zip_func,
};

lazy_static! {
    pub(crate) static ref SCANLATION_GROUPS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    pub(crate) static ref WEB_DOWNLOADED: Mutex<Vec<String>> = Mutex::new(Vec::new()); // filenames
    pub(crate) static ref MANGA_NAME: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref MANGA_ID: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref LOGS: Mutex<Vec<LOG>> = Mutex::new(Vec::new());
    pub(crate) static ref HANDLE_ID: Mutex<Box<str>> = Mutex::new(String::new().into_boxed_str());
    pub(crate) static ref HANDLE_ID_END: Mutex<Vec<Box<str>>> = Mutex::new(Vec::new());
    pub(crate) static ref CHAPTERS: Mutex<Vec<ChapterMetadata>> = Mutex::new(Vec::new());
    pub(crate) static ref CHAPTERS_TO_REMOVE: Mutex<Vec<ChapterMetadata>> = Mutex::new(Vec::new());
    pub(crate) static ref MWD: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref TO_DOWNLOAD: Mutex<Vec<String>> = Mutex::new(Vec::new());
    pub(crate) static ref TO_DOWNLOAD_DATE: Mutex<Vec<String>> = Mutex::new(Vec::new());
    pub(crate) static ref CURRENT_CHAPTER: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref CURRENT_PAGE: Mutex<u64> = Mutex::new(0);
    pub(crate) static ref CURRENT_PAGE_MAX: Mutex<u64> = Mutex::new(0);
    pub(crate) static ref CURRENT_PERCENT: Mutex<f64> = Mutex::new(0.0);
    pub(crate) static ref CURRENT_SIZE: Mutex<f64> = Mutex::new(0.0);
    pub(crate) static ref CURRENT_SIZE_MAX: Mutex<f64> = Mutex::new(0.0);
    pub(crate) static ref CURRENT_CHAPTER_PARSED: Mutex<u64> = Mutex::new(0);
    pub(crate) static ref CURRENT_CHAPTER_PARSED_MAX: Mutex<u64> = Mutex::new(0);
    pub(crate) static ref DOWNLOADING: Mutex<bool> = Mutex::new(false);
    pub(crate) static ref COVER: Mutex<bool> = Mutex::new(false);
    pub(crate) static ref SUSPENDED: Mutex<Vec<MdownError>> = Mutex::new(Vec::new());
    pub(crate) static ref ENDED: Mutex<bool> = Mutex::new(false);
    pub(crate) static ref FINAL_END: Mutex<bool> = Mutex::new(false);
    pub(crate) static ref SAVER: Mutex<bool> = Mutex::new(ARGS.saver);
    pub(crate) static ref DATE_FETCHED: Mutex<Vec<String>> = Mutex::new(Vec::new());
    pub(crate) static ref LANGUAGES: Mutex<Vec<String>> = Mutex::new(Vec::new());
    pub(crate) static ref LANGUAGE: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref CHAPTER_DATES: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    pub(crate) static ref FIXED_DATES: Mutex<Vec<String>> = Mutex::new(Vec::new());
    pub(crate) static ref GENRES: Mutex<Vec<TagMetadata>> = Mutex::new(Vec::new());
    pub(crate) static ref THEMES: Mutex<Vec<TagMetadata>> = Mutex::new(Vec::new());
}

pub(crate) fn args_delete() -> Result<(), MdownError> {
    let path = match getter::get_dat_path() {
        Ok(path) => path,
        Err(err) => {
            handle_error(&err, String::from("program"));
            return Err(err);
        }
    };
    match fs::remove_file(path.clone()) {
        Ok(()) => Ok(()),
        Err(err) => Err(MdownError::IoError(err, Some(path))),
    }
}

pub(crate) async fn show() -> Result<(), MdownError> {
    let dat_path = match getter::get_dat_path() {
        Ok(path) => path,
        Err(err) => {
            return Err(err);
        }
    };
    match fs::metadata(&dat_path) {
        Ok(_metadata) => (),
        Err(err) => {
            return Err(MdownError::IoError(err, Some(dat_path)));
        }
    }
    let mut json = match get_dat_content(dat_path.as_str()) {
        Ok(value) => value,
        Err(error) => {
            return Err(error);
        }
    };
    let version = match json.get("version").and_then(Value::as_str) {
        Some(value) => value.to_string(),
        None => String::from("unknown"),
    };
    println!("Version: {}", version);

    if let Some(data) = json.get_mut("data").and_then(Value::as_array_mut) {
        if data.len() == 0 {
            println!("No manga found");
        }
        for item in data.iter_mut() {
            println!("");
            println!("------------------------------------");
            let manga_name = (
                match item.get("name").and_then(Value::as_str) {
                    Some(m) => m,
                    None => "No Name; invalid name",
                }
            ).to_string();
            let mwd = match item.get("mwd").and_then(Value::as_str) {
                Some(val) => val,
                None => "Didn't find MWD property",
            };

            let id = match item.get("id").and_then(Value::as_str) {
                Some(id) => id,
                None => "Didn't find ID property",
            };

            let language = match item.get("current_language").and_then(Value::as_str) {
                Some(id) => id,
                None => "Didn't find ID property",
            };
            let date: Vec<String> = match item.get("date").and_then(Value::as_array) {
                Some(date) => {
                    date.iter()
                        .filter_map(|d| d.as_str().map(|s| s.to_string()))
                        .collect()
                }
                None => vec![String::from("Didn't find date property")],
            };

            let mut date_str = String::new();
            for i in date.iter() {
                date_str += &format!("{}, ", i.to_string());
            }
            date_str = date_str.trim_end_matches(", ").to_string();
            let genres: Vec<String> = match item.get("genre").and_then(Value::as_array) {
                Some(genre) => {
                    genre
                        .iter()
                        .filter_map(|d| {
                            match d.get("name").and_then(Value::as_str) {
                                Some(name) => Some(name.to_string()),
                                None => None,
                            }
                        })
                        .collect()
                }
                None => vec![String::from("Didn't find genre property")],
            };

            let mut genre_str = String::new();
            for i in genres.iter() {
                genre_str += &format!("{}, ", i.to_string());
            }
            genre_str = genre_str.trim_end_matches(", ").to_string();
            let themes: Vec<String> = match item.get("theme").and_then(Value::as_array) {
                Some(theme) => {
                    theme
                        .iter()
                        .filter_map(|d| {
                            match d.get("name").and_then(Value::as_str) {
                                Some(name) => Some(name.to_string()),
                                None => None,
                            }
                        })
                        .collect()
                }
                None => vec![String::from("Didn't find genre property")],
            };

            let mut theme_str = String::new();
            for i in themes.iter() {
                theme_str += &format!("{}, ", i.to_string());
            }
            theme_str = theme_str.trim_end_matches(", ").to_string();
            let available_languages: Vec<String> = match
                item.get("available_languages").and_then(Value::as_array)
            {
                Some(theme) => {
                    theme
                        .iter()
                        .filter_map(|d| d.as_str().map(|s| s.to_string()))
                        .collect()
                }
                None => vec![String::from("Didn't find genre property")],
            };

            let mut available_languages_str = String::new();
            for i in available_languages.iter() {
                available_languages_str += &format!("{}, ", i.to_string());
            }
            available_languages_str = available_languages_str.trim_end_matches(", ").to_string();
            let cover = fs::metadata(format!("{}\\_cover.png", mwd)).is_ok();
            let chapters: Vec<String> = match item.get("chapters").and_then(Value::as_array) {
                Some(chapters) => {
                    chapters
                        .iter()
                        .filter_map(|d|
                            (
                                match d.get("number") {
                                    Some(number) => number.as_str(),
                                    None => Some(""),
                                }
                            ).map(|s| s.to_string())
                        )
                        .collect()
                }
                None => vec![format!("Didn't find chapters in {} in dat.json", manga_name)],
            };

            let mut chapter_str = String::new();

            for i in chapters.iter() {
                chapter_str.push_str(&format!("{}, ", i.to_string().replace("\"", "")));
            }
            chapter_str = chapter_str.trim_end_matches(", ").to_string();

            println!("Manga name: {}", manga_name);
            println!("MWD: {}", mwd);
            println!("ID: {}", id);
            println!("Database fetched: {}", date_str);
            if genres.len() > 0 {
                println!("Genres: {}", genre_str);
            }
            if themes.len() > 0 {
                println!("Themes: {}", theme_str);
            }
            println!("Cover: {}", cover);
            println!("Language: {}", language);
            println!("Available language: {}", available_languages_str);
            println!("Chapters: {}", chapter_str);
            println!("");

            if ARGS.show_all {
                let mut chapters = vec![];
                if let Ok(entries) = fs::read_dir(mwd) {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let file_name = entry.file_name();
                            if let Some(name) = file_name.to_str() {
                                if name.ends_with(".cbz") {
                                    chapters.push(name.to_string());
                                }
                            }
                        }
                    }
                }
                if chapters.len() != 0 {
                    for entry in chapters {
                        let path = format!("{}\\{}", mwd, entry);
                        let obj = match check_for_metadata(&path) {
                            Ok(metadata) => metadata,
                            Err(err) => {
                                return Err(err);
                            }
                        };

                        let name = match obj.get("name").and_then(Value::as_str) {
                            Some(name) => name.to_string(),
                            None => String::from("No Name; invalid name"),
                        };

                        let pages = match obj.get("pages").and_then(Value::as_str) {
                            Some(pages) => pages.to_string(),
                            None => String::from("Invalid pages"),
                        };

                        let id = match obj.get("id").and_then(Value::as_str) {
                            Some(id) => id.to_string(),
                            None => String::from("Invalid id"),
                        };
                        let title = match obj.get("title").and_then(Value::as_str) {
                            Some(title) => title.to_string(),
                            None => String::from("Invalid title"),
                        };

                        let chapter = match obj.get("chapter").and_then(Value::as_str) {
                            Some(chapter) => chapter.to_string(),
                            None => String::from("Invalid chapter"),
                        };
                        let volume = match obj.get("volume").and_then(Value::as_str) {
                            Some(volume) => volume.to_string(),
                            None => String::from("Invalid volume"),
                        };

                        println!("Name: {}", name);
                        if volume != "null" {
                            println!("Volume: {}", volume);
                        }
                        println!("Chapter: {}", chapter);
                        println!("Pages: {}", pages);
                        println!("ID: {}", id);
                        println!("Title: {}", title);
                        println!("");
                    }
                } else {
                    println!("No chapters found");
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn check_for_metadata_saver(file_path: &str) -> Result<bool, MdownError> {
    let obj = match check_for_metadata(file_path) {
        Ok(metadata) => metadata,
        Err(err) => {
            return Err(err);
        }
    };
    let saver = match obj.get("saver").and_then(Value::as_str) {
        Some(value) =>
            match value {
                "true" => true,
                "false" => false,
                _ => {
                    return Ok(false);
                }
            }
        None => {
            return Ok(false);
        }
    };
    if
        (match SAVER.lock() {
            Ok(value) => { *value != saver }
            Err(err) => {
                return Err(MdownError::PoisonError(err.to_string()));
            }
        }) &&
        true
    {
        return Ok(true);
    }
    Ok(false)
}

pub(crate) fn check_for_metadata(file_path: &str) -> Result<Map<String, Value>, MdownError> {
    let metadata_file_name = "_metadata";

    match zip_func::extract_metadata_from_zip(file_path, metadata_file_name) {
        Ok(metadata_content) => {
            let json_value = match utils::get_json(&metadata_content) {
                Ok(value) => value,
                Err(err) => {
                    return Err(err);
                }
            };
            match json_value {
                Value::Object(obj) => {
                    return Ok(obj);
                }
                _ => {
                    return Err(MdownError::NotFoundError(String::from("")));
                }
            }
        }
        Err(_err) => {
            return Err(MdownError::NotFoundError(String::from("")));
        }
    }
}

pub(crate) async fn resolve_check() -> Result<(), MdownError> {
    let dat_path = match getter::get_dat_path() {
        Ok(path) => path,
        Err(err) => {
            return Err(err);
        }
    };
    match fs::metadata(&dat_path) {
        Ok(_metadata) => (),
        Err(err) => {
            return Err(MdownError::IoError(err, Some(dat_path.clone())));
        }
    }
    let mut json = match get_dat_content(dat_path.as_str()) {
        Ok(value) => value,
        Err(error) => {
            return Err(error);
        }
    };
    if let Some(data) = json.get_mut("data").and_then(Value::as_array_mut) {
        let mut iter: i32 = -1;
        let mut to_remove = vec![];
        for item in data.iter_mut() {
            iter += 1;
            let manga_name = (
                match item.get("name").and_then(Value::as_str) {
                    Some(m) => m,
                    None => "No Name; invalid name",
                }
            ).to_string();
            println!("Checking {}\r", manga_name);
            let past_mwd = match std::env::current_dir() {
                Ok(m) =>
                    (
                        match m.to_str() {
                            Some(s) => s,
                            None => {
                                return Err(
                                    MdownError::ConversionError(
                                        String::from("cwd conversion to string slice failed")
                                    )
                                );
                            }
                        }
                    ).to_string(),
                Err(err) => {
                    return Err(MdownError::IoError(err, None));
                }
            };
            let mwd = match item.get("mwd").and_then(Value::as_str) {
                Some(val) => val,
                None => {
                    return Err(MdownError::NotFoundError(String::from("Didn't find ID property")));
                }
            };

            *(match LANGUAGE.lock() {
                Ok(value) => value,
                Err(err) => {
                    return Err(MdownError::PoisonError(err.to_string()));
                }
            }) = match item.get("current_language").and_then(Value::as_str) {
                Some(val) => val.to_string(),
                None => {
                    return Err(MdownError::NotFoundError(String::from("Didn't find ID property")));
                }
            };
            if std::env::set_current_dir(mwd).is_err() {
                println!("{} not found; deleting from database", &manga_name);
                to_remove.push(iter);
                continue;
            }

            match std::fs::rename(format!("{past_mwd}\\.cache"), format!("{mwd}\\.cache")) {
                Ok(()) => (),
                Err(err) => {
                    eprintln!("Error: moving MWD from {} to {} {}", past_mwd, mwd, err);
                }
            }
            let id = match item.get("id").and_then(Value::as_str) {
                Some(id) => id,
                None => {
                    return Err(MdownError::NotFoundError(String::from("Didn't find ID property")));
                }
            };
            let cover_file = format!("{}\\_cover.png", mwd);
            let mut cover = fs::metadata(cover_file).is_ok();
            match getter::get_manga_json(id).await {
                Ok(manga_name_json) => {
                    let json_value = match utils::get_json(&manga_name_json) {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(err);
                        }
                    };
                    if let Value::Object(obj) = json_value {
                        let empty = Value::String(String::new());
                        let cover_data: &str = match
                            obj
                                .get("data")
                                .and_then(|name_data| name_data.get("relationships"))
                                .and_then(Value::as_array)
                                .and_then(|data| {
                                    let mut cover_data = "";
                                    for el in data {
                                        if
                                            (match el.get("type") {
                                                Some(cover_dat) => cover_dat,
                                                None => &empty,
                                            }) == "cover_art"
                                        {
                                            cover_data = match
                                                el
                                                    .get("attributes")
                                                    .and_then(|dat| dat.get("fileName"))
                                                    .and_then(Value::as_str)
                                            {
                                                Some(name) => name,
                                                None => "",
                                            };
                                        }
                                    }
                                    Option::Some(cover_data)
                                })
                        {
                            Some(name) => name,
                            None => {
                                return Err(
                                    MdownError::NotFoundError(
                                        String::from("Didn't find ID property")
                                    )
                                );
                            }
                        };

                        let title_data = match
                            obj.get("data").and_then(|name_data| name_data.get("attributes"))
                        {
                            Some(name_data) => name_data,
                            None => {
                                return Err(
                                    MdownError::NotFoundError(
                                        String::from("Didn't find attributes property (title_data)")
                                    )
                                );
                            }
                        };

                        if
                            let Some(chapters_temp) = item
                                .clone()
                                .get("chapters")
                                .and_then(Value::as_array)
                        {
                            let mut chapter_da = match CHAPTER_DATES.lock() {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(MdownError::PoisonError(err.to_string()));
                                }
                            };
                            for i in chapters_temp.iter() {
                                let number = (
                                    match i.get("number").and_then(Value::as_str) {
                                        Some(value) => value,
                                        None => "0",
                                    }
                                ).to_string();
                                let date = (
                                    match i.get("updatedAt").and_then(Value::as_str) {
                                        Some(value) => value,
                                        None => "0",
                                    }
                                ).to_string();
                                chapter_da.insert(number, date);
                            }
                            drop(chapter_da);
                        }

                        if ARGS.update && !cover {
                            let folder = get_folder_name(&get_manga_name(title_data));
                            *(match COVER.lock() {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(MdownError::PoisonError(err.to_string()));
                                }
                            }) = match
                                download::download_cover(
                                    Arc::from("https://uploads.mangadex.org/"),
                                    Arc::from(id),
                                    Arc::from(cover_data),
                                    Arc::from(folder.clone())
                                ).await
                            {
                                Ok(()) => {
                                    cover = true;
                                    true
                                }
                                Err(err) => {
                                    eprintln!("Error: failed to download cover {}", err);
                                    false
                                }
                            };
                        }
                        *(match MANGA_NAME.lock() {
                            Ok(value) => value,
                            Err(err) => {
                                return Err(MdownError::PoisonError(err.to_string()));
                            }
                        }) = get_manga_name(title_data);
                        match resolve_manga(&id, get_manga_name(title_data).as_str(), false).await {
                            Ok(()) => (),
                            Err(err) => {
                                handle_error(&err, String::from("manga"));
                            }
                        }
                    } else {
                        return Err(MdownError::JsonError(String::from("Failed to parse")));
                    }
                }
                Err(_) => (),
            }
            if ARGS.update {
                item["cover"] = match COVER.lock() {
                    Ok(value) => {
                        if !cover { Value::Bool(*value) } else { Value::Bool(true) }
                    }
                    Err(err) => {
                        return Err(MdownError::PoisonError(err.to_string()));
                    }
                };
            }
            if
                let Some(chapters_temp) = item
                    .clone()
                    .get_mut("chapters")
                    .and_then(Value::as_array_mut)
            {
                let chapters_remove = match CHAPTERS_TO_REMOVE.lock() {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(MdownError::PoisonError(err.to_string()));
                    }
                };
                for i in chapters_remove.iter() {
                    chapters_temp.retain(|value| {
                        let number = getter::get_attr_as_str(value, "number");
                        let date = getter::get_attr_as_str(value, "updatedAt");
                        let id = getter::get_attr_as_str(value, "id");
                        ChapterMetadata::new(number, date, id).value() != i.value()
                    });
                }
                drop(chapters_remove);
                let mut chapters = Vec::new();
                for i in chapters_temp.iter() {
                    let number = getter::get_attr_as_str(i, "number");
                    let date = getter::get_attr_as_str(i, "updatedAt");
                    let id = getter::get_attr_as_str(i, "id");
                    chapters.push(ChapterMetadata::new(number, date, id).value());
                }
                let chapters_lock = match CHAPTERS.lock() {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(
                            MdownError::PoisonError(format!("Failed to lock CHAPTERS: {}", err))
                        );
                    }
                };

                for i in chapters_lock.iter() {
                    if !chapters.contains(&i.value()) {
                        chapters.push(i.value());
                    }
                }
                item["chapters"] = serde_json::Value::Array(chapters);
            }
            if
                (
                    match item["chapters"].as_array() {
                        Some(chapters) => chapters,
                        None => {
                            return Err(MdownError::NotFoundError(String::from("")));
                        }
                    }
                ).len() == 0 &&
                !cover
            {
                println!("{} not found; deleting from database", &manga_name);
                to_remove.push(iter);
                continue;
            }

            if ARGS.check {
                println!("Checked {} ({})", &manga_name, item["id"]);
                let to_dow;
                if
                    !(
                        match TO_DOWNLOAD.lock() {
                            Ok(value) => value,
                            Err(err) => {
                                return Err(MdownError::PoisonError(err.to_string()));
                            }
                        }
                    ).is_empty() ||
                    !(
                        match TO_DOWNLOAD_DATE.lock() {
                            Ok(value) => value,
                            Err(err) => {
                                return Err(MdownError::PoisonError(err.to_string()));
                            }
                        }
                    ).is_empty()
                {
                    to_dow = true;
                    println!("Chapters available");
                    for chapter in (
                        match TO_DOWNLOAD.lock() {
                            Ok(value) => value,
                            Err(err) => {
                                return Err(MdownError::PoisonError(err.to_string()));
                            }
                        }
                    ).iter() {
                        println!(" {}", chapter);
                    }
                    for chapter in (
                        match TO_DOWNLOAD_DATE.lock() {
                            Ok(value) => value,
                            Err(err) => {
                                return Err(MdownError::PoisonError(err.to_string()));
                            }
                        }
                    ).iter() {
                        println!(" {} (OUTDATED CHAPTER)", chapter);
                    }
                } else if
                    (match FIXED_DATES.lock() {
                        Ok(value) => !value.is_empty(),
                        Err(err) => {
                            return Err(MdownError::PoisonError(err.to_string()));
                        }
                    }) &&
                    true
                {
                    to_dow = false;
                    println!("Chapters ERROR");
                    for date in (
                        match FIXED_DATES.lock() {
                            Ok(value) => value,
                            Err(err) => {
                                return Err(MdownError::PoisonError(err.to_string()));
                            }
                        }
                    ).iter() {
                        println!(" {} (CORRUPT DATE) (FIXED)", date);
                    }
                } else {
                    to_dow = false;
                }
                if !cover {
                    println!("Cover is not downloaded");
                }
                if !to_dow && cover {
                    println!("Up to-date");
                }
            }
            (
                match CHAPTERS.lock() {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(MdownError::PoisonError(err.to_string()));
                    }
                }
            ).clear();
        }
        for &index in to_remove.iter().rev() {
            data.remove(index as usize);
        }
        let mut file = match File::create(&dat_path) {
            Ok(path) => path,
            Err(err) => {
                return Err(MdownError::IoError(err, Some(dat_path)));
            }
        };

        let json_string = match serde_json::to_string_pretty(&json) {
            Ok(value) => value,
            Err(err) => {
                return Err(MdownError::JsonError(String::from(err.to_string())));
            }
        };

        if let Err(err) = writeln!(file, "{}", json_string) {
            return Err(MdownError::IoError(err, Some(dat_path)));
        }
    }
    Ok(())
}

pub(crate) fn resolve_dat() -> Result<(), MdownError> {
    let dat_path = match getter::get_dat_path() {
        Ok(path) => path,
        Err(err) => {
            return Err(err);
        }
    };
    if fs::metadata(&dat_path).is_err() {
        let mut file = match fs::File::create(&dat_path) {
            Ok(file) => file,
            Err(err) => {
                return Err(MdownError::IoError(err, Some(dat_path)));
            }
        };

        let content = format!(
            "{{\n  \"data\": [],\n  \"version\": \"{}\"\n}}",
            env!("CARGO_PKG_VERSION")
        );

        match file.write_all(content.as_bytes()) {
            Ok(()) => (),
            Err(_err) => (),
        };
    }
    let mut json = match get_dat_content(dat_path.as_str()) {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };
    if let Some(data) = json.get_mut("data").and_then(Value::as_array_mut) {
        let manga_names: Vec<&str> = data
            .iter()
            .filter_map(|item| item.get("name").and_then(Value::as_str))
            .collect();

        if
            manga_names.contains(
                &(
                    match MANGA_NAME.lock() {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(MdownError::PoisonError(err.to_string()));
                        }
                    }
                ).as_str()
            )
        {
            for item in data.iter_mut() {
                if let Some(name) = item.get("name").and_then(Value::as_str) {
                    if
                        name ==
                        (
                            match MANGA_NAME.lock() {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(MdownError::PoisonError(err.to_string()));
                                }
                            }
                        ).as_str()
                    {
                        let existing_chapters = match
                            item.get_mut("chapters").and_then(Value::as_array_mut)
                        {
                            Some(value) => value,
                            None => {
                                return Err(
                                    MdownError::NotFoundError(
                                        String::from("mut chapters in dat.json")
                                    )
                                );
                            }
                        };

                        let mut existing_chapters_temp = Vec::new();

                        for i in existing_chapters.iter() {
                            let number = getter::get_attr_as_same(i, "number");
                            existing_chapters_temp.push(number);
                        }

                        let mut new_chapters: Vec<_> = (
                            match CHAPTERS.lock() {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(MdownError::PoisonError(err.to_string()));
                                }
                            }
                        )
                            .iter()
                            .cloned()
                            .filter(|chapter| {
                                let number = json!(chapter.number);
                                !existing_chapters_temp.contains(&&number)
                            })
                            .collect();

                        new_chapters.sort_by(|a, b| {
                            let a_num = match json!(a.number).as_str() {
                                Some(value) =>
                                    match value.parse::<u32>() {
                                        Ok(value) => value,
                                        Err(_err) => 0,
                                    }
                                None => { 0 }
                            };
                            let b_num = match json!(b.number).as_str() {
                                Some(value) =>
                                    match value.parse::<u32>() {
                                        Ok(value) => value,
                                        Err(_err) => 0,
                                    }
                                None => { 0 }
                            };
                            a_num.cmp(&b_num)
                        });

                        for i in new_chapters.iter() {
                            existing_chapters.push(i.value());
                        }

                        break;
                    }
                }
            }
        } else {
            let mwd = format!("{}", match MWD.lock() {
                Ok(value) => value,
                Err(err) => {
                    return Err(MdownError::PoisonError(err.to_string()));
                }
            });
            let cover = match COVER.lock() {
                Ok(value) => *value,
                Err(err) => {
                    return Err(MdownError::PoisonError(err.to_string()));
                }
            };
            let mut chapters = Vec::new();
            let chapters_data = (
                match CHAPTERS.lock() {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(MdownError::PoisonError(err.to_string()));
                    }
                }
            ).clone();
            for i in chapters_data.iter() {
                chapters.push(i.json());
            }
            let mut genres = Vec::new();
            let genres_data = (
                match GENRES.lock() {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(MdownError::PoisonError(err.to_string()));
                    }
                }
            ).clone();
            for i in genres_data.iter() {
                genres.push(i.json());
            }
            let mut themes = Vec::new();
            let themes_data = (
                match THEMES.lock() {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(MdownError::PoisonError(err.to_string()));
                    }
                }
            ).clone();
            for i in themes_data.iter() {
                themes.push(i.json());
            }
            let manga_data =
                json!({
                    "name": match MANGA_NAME.lock(){
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(
                                            MdownError::PoisonError(err.to_string())
                                        );
                                    }
                                }.clone(),
                    "id": match MANGA_ID.lock(){
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(
                                            MdownError::PoisonError(err.to_string())
                                        );
                                    }
                                }.to_string(),
                    "chapters": chapters.clone(),
                    "mwd": mwd,
                    "cover": cover,
                    "date":  match DATE_FETCHED.lock(){
                        Ok(value) => value,
                        Err(err) => {
                            return Err(
                                MdownError::PoisonError(err.to_string())
                            );
                        }
                    }.clone(),
                    "available_languages":  match LANGUAGES.lock(){
                        Ok(value) => value,
                        Err(err) => {
                            return Err(
                                MdownError::PoisonError(err.to_string())
                            );
                        }
                    }.clone(),
                    "current_language":  match LANGUAGE.lock(){
                        Ok(value) => value,
                        Err(err) => {
                            return Err(
                                MdownError::PoisonError(err.to_string())
                            );
                        }
                    }.clone(),
                    "theme":  themes.clone(),
                    "genre":  genres.clone(),
                    });

            data.push(manga_data.clone());
        }

        let mut file = match File::create(&dat_path) {
            Ok(file) => file,
            Err(err) => {
                return Err(MdownError::IoError(err, Some(dat_path)));
            }
        };

        let json_string = match serde_json::to_string_pretty(&json) {
            Ok(value) => value,
            Err(err) => {
                return Err(MdownError::JsonError(String::from(err.to_string())));
            }
        };

        if let Err(err) = writeln!(file, "{}", json_string) {
            return Err(MdownError::JsonError(String::from(err.to_string())));
        }
    }

    Ok(())
}

pub(crate) fn get_dat_content(dat_path: &str) -> Result<Value, MdownError> {
    let file = File::open(&dat_path);
    let mut file = match file {
        Ok(file) => file,
        Err(err) => {
            return Err(MdownError::IoError(err, Some(dat_path.to_string())));
        }
    };
    let mut contents = String::new();
    if let Err(err) = file.read_to_string(&mut contents) {
        return Err(MdownError::IoError(err, Some(dat_path.to_string())));
    }
    utils::get_json(&contents)
}

pub(crate) async fn resolve(obj: Map<String, Value>, id: &str) -> Result<String, MdownError> {
    let handle_id = utils::generate_random_id(16);
    *(match HANDLE_ID.lock() {
        Ok(id) => id,
        Err(err) => {
            return Err(MdownError::PoisonError(err.to_string()));
        }
    }) = handle_id.clone();
    let title_data = match obj.get("data").and_then(|name_data| name_data.get("attributes")) {
        Some(value) => value,
        None => {
            return Err(MdownError::NotFoundError(String::from("resolve")));
        }
    };

    let manga_name = if ARGS.title == "*" {
        get_manga_name(title_data)
    } else {
        ARGS.title.to_string()
    };
    *(match MANGA_NAME.lock() {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::PoisonError(err.to_string()));
        }
    }) = manga_name.clone();
    let folder = get_folder_name(&manga_name);

    let orig_lang = match title_data.get("originalLanguage").and_then(Value::as_str) {
        Some(value) => value,
        None => {
            return Err(MdownError::NotFoundError(String::from("Didn't find originalLanguage")));
        }
    };
    let languages = match title_data.get("availableTranslatedLanguages").and_then(Value::as_array) {
        Some(value) => value,
        None => {
            return Err(
                MdownError::NotFoundError(String::from("Didn't find availableTranslatedLanguages"))
            );
        }
    };
    let mut final_lang = vec![];
    for lang in languages {
        final_lang.push(match lang.as_str() {
            Some(value) => value,
            None => {
                return Err(
                    MdownError::ConversionError(
                        String::from("final_lang could not convert to string slice ?")
                    )
                );
            }
        });
    }
    let current_lang = (
        match LANGUAGE.lock() {
            Ok(value) => value,
            Err(err) => {
                return Err(MdownError::PoisonError(err.to_string()));
            }
        }
    ).to_string();
    if
        current_lang != orig_lang &&
        !final_lang.contains(&current_lang.as_str()) &&
        current_lang != "*"
    {
        let mut final_lang = vec![];
        for lang in languages {
            final_lang.push(match lang.as_str() {
                Some(value) => value,
                None => {
                    return Err(
                        MdownError::ConversionError(
                            String::from("final_lang could not convert to string slice ?")
                        )
                    );
                }
            });
        }
        let mut langs = String::new();
        let mut lang_range: usize = 0;
        for lang in languages {
            langs.push_str(&format!("{} ", lang.to_string().replace("\"", "")));
            lang_range += 1 + lang.to_string().replace("\"", "").len();
        }
        lang_range -= 1;
        string(
            1,
            0,
            &format!("Language is not available\nSelected language: {}", match LANGUAGE.lock() {
                Ok(value) => value,
                Err(err) => {
                    return Err(MdownError::PoisonError(err.to_string()));
                }
            })
        );
        string(3, 0, &format!("Original language: {}", orig_lang));
        string(4, 0, &format!("Available languages: {}", langs));
        string(5, 0, &format!("Choose from these    {}", "^".repeat(lang_range)));
        return Ok(manga_name);
    }
    drop(current_lang);
    *(match DOWNLOADING.lock() {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::PoisonError(err.to_string()));
        }
    }) = true;

    let was_rewritten = fs::metadata(folder.clone()).is_ok();
    match fs::create_dir(&folder) {
        Ok(()) => (),
        Err(err) => {
            if
                (match err.raw_os_error() {
                    Some(value) => value,
                    None => 0,
                }) != 183
            {
                eprintln!("Error: creating directory {} {}", &folder, err);
            }
        }
    }
    *(match MWD.lock() {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::PoisonError(err.to_string()));
        }
    }) = match std::fs::canonicalize(&folder) {
        Ok(value) =>
            match value.to_str() {
                Some(value) => value.to_string(),
                None => {
                    return Err(
                        MdownError::ConversionError(
                            String::from("final_lang could not convert to string slice ?")
                        )
                    );
                }
            }
        Err(err) => {
            return Err(MdownError::IoError(err, Some(folder)));
        }
    };
    let desc = match
        title_data
            .get("description")
            .and_then(|description| description.get("en"))
            .and_then(Value::as_str)
    {
        Some(value) => value,
        None => "",
    };
    let mut desc_file = match
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("{}\\_description.txt", folder))
    {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::IoError(err, Some(format!("{}\\_description.txt", folder))));
        }
    };
    match write!(desc_file, "{}", desc) {
        Ok(()) => (),
        Err(err) => eprintln!("Error: writing in description file {}", err),
    }

    let empty_vec = vec![];

    let tags_attributes = match title_data.get("tags").and_then(Value::as_array) {
        Some(value) => value,
        None => &empty_vec,
    };

    let mut theme: Vec<TagMetadata> = vec![];
    let mut genre: Vec<TagMetadata> = vec![];

    for tag in tags_attributes.iter() {
        let id = match tag.get("id").and_then(Value::as_str) {
            Some(value) => value,
            None => "",
        };
        let attr = tag.get("attributes");
        if let Some(attr) = attr {
            let typ = match attr.get("group").and_then(Value::as_str) {
                Some(value) => value,
                None => "",
            };
            let name = match
                attr
                    .get("name")
                    .and_then(|value| value.get("en"))
                    .and_then(Value::as_str)
            {
                Some(value) => value,
                None => "",
            };
            if !name.is_empty() {
                match typ {
                    "theme" => {
                        theme.push(TagMetadata::new(name, id));
                    }
                    "genre" => {
                        genre.push(TagMetadata::new(name, id));
                    }
                    _ => (),
                }
            }
        }
    }

    *(match GENRES.lock() {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::PoisonError(err.to_string()));
        }
    }) = genre;
    *(match THEMES.lock() {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::PoisonError(err.to_string()));
        }
    }) = theme;

    let folder = get_folder_name(&manga_name);
    let cover = match
        obj
            .get("data")
            .and_then(|name_data| name_data.get("relationships"))
            .and_then(Value::as_array)
            .and_then(|data| {
                let mut cover = "";
                for el in data {
                    if
                        (match el.get("type").and_then(Value::as_str) {
                            Some(el) => el,
                            None => "",
                        }) == "cover_art"
                    {
                        cover = match
                            el
                                .get("attributes")
                                .and_then(|dat| dat.get("fileName"))
                                .and_then(Value::as_str)
                        {
                            Some(cover) => cover,
                            None => "",
                        };
                    }
                }
                Option::Some(cover)
            })
    {
        Some(cover) => cover,
        None => "",
    };
    if cover != "" {
        *(match COVER.lock() {
            Ok(value) => value,
            Err(err) => {
                return Err(MdownError::PoisonError(err.to_string()));
            }
        }) = match
            download::download_cover(
                Arc::from("https://uploads.mangadex.org/"),
                Arc::from(id),
                Arc::from(cover),
                Arc::from(folder.clone())
            ).await
        {
            Ok(()) => true,
            Err(err) => {
                eprintln!("Error: failed to download cover {}", err);
                false
            }
        };
    }

    if ARGS.stat {
        match download::download_stat(&id, &folder, &manga_name).await {
            Ok(()) => (),
            Err(err) => {
                crate::error::handle_error(&err, String::from("statistics"));
            }
        };
    }

    *(match LANGUAGES.lock() {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::PoisonError(err.to_string()));
        }
    }) = {
        let langs = match title_data.get("availableTranslatedLanguages").and_then(Value::as_array) {
            Some(value) => value,
            None => {
                return Err(MdownError::NotFoundError(String::from("resolve")));
            }
        };
        let mut langs_final: Vec<String> = Vec::new();
        for lang in langs.iter() {
            langs_final.push(lang.to_string().replace("\"", ""));
        }
        langs_final
    };

    match resolve_manga(&id, &manga_name, was_rewritten).await {
        Ok(()) => (),
        Err(err) => {
            handle_error(&err, String::from("program"));
        }
    }
    log_end(handle_id);
    if ARGS.web || ARGS.gui || ARGS.check || ARGS.update || ARGS.log {
        log!("Downloaded manga");
    }
    *(match DOWNLOADING.lock() {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::PoisonError(err.to_string()));
        }
    }) = false;
    Ok(manga_name)
}

pub(crate) async fn resolve_group(array_item: &Value) -> Result<(String, String), MdownError> {
    let scanlation_group = match array_item.get("relationships").and_then(Value::as_array) {
        Some(group) => group,
        None => {
            return Ok((String::from("null"), String::from("null")));
        }
    };
    let scanlation_group_id = match get_scanlation_group(scanlation_group) {
        Some(value) => value,
        None => {
            (
                match SUSPENDED.lock() {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(MdownError::PoisonError(err.to_string()));
                    }
                }
            ).push(MdownError::NotFoundError(String::from("resolve_group")));
            return Ok((String::from("null"), String::from("null")));
        }
    };
    if scanlation_group_id.is_empty() {
        return Ok((String::from("null"), String::from("null")));
    }

    let (name, website) = match resolve_group_metadata(scanlation_group_id).await {
        Ok((name, website)) => (name, website),
        Err(err) => {
            return Err(err);
        }
    };
    if
        name != "Unknown" &&
        !(
            match SCANLATION_GROUPS.lock() {
                Ok(value) => value,
                Err(err) => {
                    return Err(MdownError::PoisonError(err.to_string()));
                }
            }
        ).contains_key(scanlation_group_id)
    {
        (
            match SCANLATION_GROUPS.lock() {
                Ok(value) => value,
                Err(err) => {
                    return Err(MdownError::PoisonError(err.to_string()));
                }
            }
        ).insert(String::from(scanlation_group_id), name.clone());
    }
    Ok((name, website))
}

pub(crate) fn get_scanlation_group_to_file(
    manga_name: &str,
    name: String,
    website: String
) -> Result<(), MdownError> {
    if name == String::from("null") {
        return Ok(());
    }
    let file_name = format!("{}\\_scanlation_groups.txt", get_folder_name(manga_name));

    let mut file_inst = match OpenOptions::new().create(true).append(true).open(&file_name) {
        Ok(file_inst) => file_inst,
        Err(err) => {
            return Err(MdownError::IoError(err, Some(file_name)));
        }
    };

    match file_inst.write_all(format!("{} - {}\n", name, website).as_bytes()) {
        Ok(()) => (),
        Err(err) => eprintln!("Error: writing to {}: {}", name, err),
    }
    Ok(())
}

pub(crate) async fn resolve_group_metadata(id: &str) -> Result<(String, String), MdownError> {
    let base_url = "https://api.mangadex.org/group/";
    let full_url = format!("{}\\{}", base_url, id);

    let response = match download::get_response_client(&full_url).await {
        Ok(res) => res,
        Err(err) => {
            return Err(err);
        }
    };
    if response.status().is_success() {
        let json = match response.text().await {
            Ok(json) => json,
            Err(err) => {
                return Err(MdownError::JsonError(err.to_string()));
            }
        };
        let json_value = match utils::get_json(&json) {
            Ok(value) => value,
            Err(err) => {
                return Err(err);
            }
        };
        match json_value {
            Value::Object(obj) => {
                let data = match obj.get("data") {
                    Some(value) => value,
                    None => {
                        return Err(
                            MdownError::NotFoundError(format!("data in resolve_group_metadata"))
                        );
                    }
                };
                let attr = match data.get("attributes") {
                    Some(value) => value,
                    None => {
                        return Err(
                            MdownError::NotFoundError(
                                format!("attributes in resolve_group_metadata")
                            )
                        );
                    }
                };
                let name = match attr.get("name").and_then(Value::as_str) {
                    Some(name) => name.to_string(),
                    None => {
                        return Ok((String::from("Unknown"), String::new()));
                    }
                };
                let website = (
                    match attr.get("website").and_then(Value::as_str) {
                        Some(value) => value,
                        None => "None",
                    }
                ).to_owned();
                return Ok((name, website));
            }
            _ => {
                return Ok((String::from("Unknown"), String::new()));
            }
        }
    }
    return Err(MdownError::NetworkError(response.error_for_status().unwrap_err()));
}

async fn resolve_manga(id: &str, manga_name: &str, was_rewritten: bool) -> Result<(), MdownError> {
    let going_offset: u32 = match ARGS.database_offset.as_str().parse() {
        Ok(offset) => offset,
        Err(err) => {
            return Err(MdownError::ConversionError(err.to_string()));
        }
    };
    let arg_force = ARGS.force;
    let mut downloaded: Vec<String> = vec![];
    *(match MANGA_ID.lock() {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::PoisonError(err.to_string()));
        }
    }) = id.to_owned();
    match get_manga(id, going_offset).await {
        Ok((json, _offset)) => {
            let downloaded_temp = match download_manga(json, manga_name, arg_force).await {
                Ok(value) => value,
                Err(err) => {
                    return Err(err);
                }
            };
            for i in 0..downloaded_temp.len() {
                downloaded.push(downloaded_temp[i].clone());
            }
            clear_screen(1);
        }
        Err(err) => eprintln!("Error: {}", err),
    }
    if !ARGS.web && !ARGS.gui && !ARGS.check && !ARGS.update {
        if downloaded.len() != 0 {
            string(1, 0, "Downloaded files:");
            for i in 0..downloaded.len() {
                (_, downloaded) = resolve_move(i as u32, downloaded.clone(), 2, 1);
            }
        } else {
            if !was_rewritten {
                match fs::remove_dir_all(get_folder_name(manga_name)) {
                    Ok(()) => (),
                    Err(err) => eprintln!("Error: remove directory {}", err),
                };
            }
        }
    }
    Ok(())
}

pub(crate) fn resolve_move(
    mut moves: u32,
    mut hist: Vec<String>,
    start: u32,
    end: u32
) -> (u32, Vec<String>) {
    if moves + start >= MAXPOINTS.max_y - end {
        hist.remove(0);
    } else {
        moves += 1;
    }
    for i in 0..moves {
        if (i as usize) == hist.len() {
            break;
        }
        let message = &hist[i as usize];
        let length = message.len();
        if length < (MAXPOINTS.max_x as usize) {
            string(
                (start + i) as u32,
                0,
                &format!("{}{}", message, " ".repeat((MAXPOINTS.max_x as usize) - message.len()))
            );
        } else {
            string(start + i, 0, &format!("{}", message));
        }
    }
    (moves, hist)
}

pub(crate) fn title(mut title: &str) -> &str {
    if
        (match title.chars().last() {
            Some(value) => value,
            None => '0',
        }) == '.'
    {
        title = &title[..title.len() - 1];
    }
    title
}

pub(crate) fn resolve_skip(arg: String, with: &str) -> bool {
    if arg == "*" || arg == with {
        return false;
    }
    true
}
