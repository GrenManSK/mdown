use crossterm::event::{ self, Event, KeyCode };
use parking_lot::Mutex;
use lazy_static::lazy_static;
use serde_json::{ Map, Value };
use std::{ collections::HashMap, fs::{ self, File, OpenOptions }, io::{ Read, Write }, sync::Arc };

use crate::{
    args::{ self, ARGS },
    debug,
    download,
    download_manga,
    error::MdownError,
    getter::{ self, get_folder_name, get_manga, get_manga_name, get_scanlation_group },
    handle_error,
    log,
    log_end,
    MAXPOINTS,
    metadata::{
        self,
        ChapterMetadata,
        Dat,
        Log,
        MangaDownloadLogs,
        MangaMetadata,
        MdownLogs,
        TagMetadata,
    },
    string,
    utils::{ self, clear_screen, input },
    zip_func,
};

lazy_static! {
    pub(crate) static ref SCANLATION_GROUPS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new()); // ID, name
    pub(crate) static ref WEB_DOWNLOADED: Mutex<Vec<String>> = Mutex::new(Vec::new()); // filenames
    pub(crate) static ref MANGA_NAME: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref MANGA_ID: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref CHAPTER_ID: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref LOGS: Mutex<Vec<Log>> = Mutex::new(Vec::new());
    pub(crate) static ref HANDLE_ID: Mutex<Box<str>> = Mutex::new(String::new().into_boxed_str()); // handle id
    pub(crate) static ref HANDLE_ID_END: Mutex<Vec<Box<str>>> = Mutex::new(Vec::new()); // handle id to end
    pub(crate) static ref CHAPTERS: Mutex<Vec<ChapterMetadata>> = Mutex::new(Vec::new()); // chapter metadata
    pub(crate) static ref CHAPTERS_TO_REMOVE: Mutex<Vec<ChapterMetadata>> = Mutex::new(Vec::new()); // chapters to remove from database
    pub(crate) static ref MWD: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref TO_DOWNLOAD: Mutex<Vec<String>> = Mutex::new(Vec::new()); // chapter number to download
    pub(crate) static ref TO_DOWNLOAD_DATE: Mutex<Vec<String>> = Mutex::new(Vec::new()); // chapter number to download because of date
    pub(crate) static ref CURRENT_CHAPTER: Mutex<String> = Mutex::new(String::new()); // filename.get_folder_name()
    pub(crate) static ref CURRENT_PAGE: Mutex<u64> = Mutex::new(0);
    pub(crate) static ref CURRENT_PAGE_MAX: Mutex<u64> = Mutex::new(0);
    pub(crate) static ref CURRENT_PERCENT: Mutex<f64> = Mutex::new(0.0);
    pub(crate) static ref CURRENT_SIZE: Mutex<f64> = Mutex::new(0.0);
    pub(crate) static ref CURRENT_SIZE_MAX: Mutex<f64> = Mutex::new(0.0);
    pub(crate) static ref CURRENT_CHAPTER_PARSED: Mutex<u64> = Mutex::new(0);
    pub(crate) static ref CURRENT_CHAPTER_PARSED_MAX: Mutex<u64> = Mutex::new(0);
    pub(crate) static ref DOWNLOADING: Mutex<bool> = Mutex::new(false);
    pub(crate) static ref COVER: Mutex<bool> = Mutex::new(false);
    pub(crate) static ref SUSPENDED: Mutex<Vec<MdownError>> = Mutex::new(Vec::new()); // Suspended errors
    pub(crate) static ref ENDED: Mutex<bool> = Mutex::new(false); // end variable for handlers
    pub(crate) static ref FINAL_END: Mutex<bool> = Mutex::new(false); // if true at the end it will use std::process::exit(0)
    pub(crate) static ref SAVER: Mutex<bool> = Mutex::new(ARGS.lock().saver);
    pub(crate) static ref DATE_FETCHED: Mutex<Vec<String>> = Mutex::new(Vec::new()); // date of fetching data in format %Y-%m-%d %H:%M:%S
    pub(crate) static ref LANGUAGES: Mutex<Vec<String>> = Mutex::new(Vec::new()); // vec of all available languages
    pub(crate) static ref LANGUAGE: Mutex<String> = Mutex::new(String::new()); // current language
    pub(crate) static ref CHAPTER_IDS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new()); // chapter number, id from mangadex database
    pub(crate) static ref CHAPTER_DATES: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new()); // chapter number, time from mangadex database
    pub(crate) static ref FIXED_DATES: Mutex<Vec<String>> = Mutex::new(Vec::new()); // vec of chapter number which have been fixed
    pub(crate) static ref GENRES: Mutex<Vec<TagMetadata>> = Mutex::new(Vec::new());
    pub(crate) static ref THEMES: Mutex<Vec<TagMetadata>> = Mutex::new(Vec::new());
    pub(crate) static ref MUSIC_STAGE: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref MUSIC_END: Mutex<bool> = Mutex::new(false);
}
pub(crate) fn args_delete() -> Result<(), MdownError> {
    let path = match getter::get_dat_path() {
        Ok(path) => path,
        Err(err) => {
            handle_error!(&err, String::from("program"));
            return Err(err);
        }
    };
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) => Err(MdownError::IoError(err, path)),
    }
}

pub(crate) async fn show_log() -> Result<(), MdownError> {
    let log_path = match getter::get_log_path() {
        Ok(path) => path,
        Err(err) => {
            return Err(err);
        }
    };
    match fs::metadata(&log_path) {
        Ok(_metadata) => (),
        Err(err) => {
            return Err(MdownError::IoError(err, log_path));
        }
    }
    let json = match get_dat_content(log_path.as_str()) {
        Ok(value) => value,
        Err(error) => {
            return Err(error);
        }
    };

    match serde_json::from_value::<MdownLogs>(json) {
        Ok(logs) => {
            println!(
                "N     ID            Name        In brackets is probable type (it may not be same real type)"
            );
            let data = logs.clone();

            let mut names: Vec<(String, String)> = Vec::new();
            for (name, log) in data.iter() {
                names.push((name.to_string(), log.name.clone()));
            }
            names.sort();
            for (times, (name, log)) in names.iter().enumerate() {
                let typ = match name.len() {
                    12 => "web",
                    16 => "downloader",
                    _ => "unknown",
                };
                println!("{}: {} {} ({})", times, name, log, typ);
            }

            let vstup = match input("> ") {
                Ok(vstup) => vstup,
                Err(err) => {
                    return Err(err);
                }
            };

            println!();

            let code = match vstup.parse::<usize>() {
                Ok(code) => code,
                Err(err) => {
                    return Err(MdownError::ConversionError(err.to_string()));
                }
            };
            if code >= data.len() {
                return Err(MdownError::ConversionError(String::from("code")));
            }

            let name = match names.get(code) {
                Some((name, _)) => name,
                None => {
                    return Err(MdownError::ConversionError(String::from("name")));
                }
            };

            let log = match logs.get(name) {
                Some(items) => items.clone(),
                None => MangaDownloadLogs::default(),
            };

            let name = log.name;
            let id = log.id;
            let mwd = log.mwd;
            let time_start = log.time_start;
            let time_end = log.time_end;
            let r#type = log.r#type;
            let logs = log.logs;

            println!("name: {}", name);
            println!("id: {}", id);
            println!("mwd: {}", mwd);
            println!("time_start: {}", time_start);
            println!("time_end:   {}", time_end);
            println!("type: {}", r#type);

            println!();

            let mut names: Vec<String> = Vec::new();
            for (name, _) in logs.iter() {
                let name = match name.as_str() {
                    "" => "General",
                    x => x,
                };
                names.push(name.to_string());
            }

            names.sort();

            println!("N   Name");

            for (times, name) in names.iter().enumerate() {
                println!("{}: {}", times, name);
            }

            let vstup = match input("> ") {
                Ok(vstup) => vstup,
                Err(err) => {
                    return Err(err);
                }
            };
            let code = match vstup.parse::<usize>() {
                Ok(code) => code,
                Err(err) => {
                    return Err(MdownError::ConversionError(err.to_string()));
                }
            };
            if code >= logs.len() {
                return Err(MdownError::ConversionError(String::from("code")));
            }

            let name = match names.get(code) {
                Some(name) =>
                    match name.as_str() {
                        "General" => "",
                        x => x,
                    }
                None => {
                    return Err(MdownError::ConversionError(String::from("name")));
                }
            };

            let items = match logs.get(name) {
                Some(items) => items.to_vec(),
                None => Vec::new(),
            };

            let mut lines = 0;
            let mut stdout = std::io::stdout().lock();

            for (times, name) in items.iter().enumerate() {
                if times >= lines + 100 {
                    match
                        write!(
                            stdout,
                            "Press Enter to print the next line, or space to print the next 100 lines.\r"
                        )
                    {
                        Ok(_) => {}
                        Err(err) => {
                            return Err(MdownError::IoError(err, String::from("stdout")));
                        }
                    }
                    match stdout.flush() {
                        Ok(_) => {}
                        Err(err) => {
                            return Err(MdownError::IoError(err, String::from("stdout")));
                        }
                    }

                    if let Ok(Event::Key(key_event)) = event::read() {
                        match key_event.code {
                            KeyCode::Enter => {
                                lines += 2;
                            }
                            KeyCode::Char(' ') => {
                                lines += 50;
                            }
                            _ => {}
                        }
                    }
                }
                println!("{}: {}", times, name);
            }
        }
        Err(err) => {
            return Err(MdownError::JsonError(err.to_string()));
        }
    }

    Ok(())
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
            return Err(MdownError::IoError(err, dat_path));
        }
    }
    let json = match get_dat_content(dat_path.as_str()) {
        Ok(value) => value,
        Err(error) => {
            return Err(error);
        }
    };

    match serde_json::from_value::<Dat>(json) {
        Ok(dat) => {
            let version = dat.version;
            println!("Version: {}", version);
            let data = dat.data;
            if data.is_empty() {
                println!("No manga found");
            }
            for item in data.iter() {
                let id = item.id.clone();

                let mut cont = false;
                let show;
                let show_all;

                match ARGS.lock().show {
                    Some(Some(ref filter)) if !filter.is_empty() => {
                        if &id != filter {
                            cont = true;
                        }
                        show = true;
                    }
                    Some(_) => {
                        show = true;
                    }
                    None => {
                        show = false;
                    }
                }

                match ARGS.lock().show_all {
                    Some(Some(ref filter)) if !filter.is_empty() => {
                        if &id != filter {
                            cont = true;
                        }
                        show_all = true;
                    }
                    Some(_) => {
                        cont = false;
                        show_all = true;
                    }
                    None => {
                        show_all = false;
                    }
                }

                if !show && !show_all {
                    return Ok(());
                }

                if cont {
                    continue;
                }
                println!();
                println!("------------------------------------");
                let manga_name = item.name.clone();
                let mwd = item.mwd.clone();
                let language = item.current_language.clone();
                let date = item.date.clone();
                let mut date_str = String::new();
                for i in date.iter() {
                    date_str += &format!("{}, ", i);
                }
                date_str = date_str.trim_end_matches(", ").to_string();
                let genres: Vec<String> = item.genre
                    .iter()
                    .map(|d| { d.name.clone() })
                    .collect();

                let mut genre_str = String::new();
                for i in genres.iter() {
                    genre_str += &format!("{}, ", i);
                }
                genre_str = genre_str.trim_end_matches(", ").to_string();

                let themes: Vec<String> = item.theme
                    .iter()
                    .map(|d| { d.name.clone() })
                    .collect();

                let mut theme_str = String::new();
                for i in themes.iter() {
                    theme_str += &format!("{}, ", i);
                }
                theme_str = theme_str.trim_end_matches(", ").to_string();
                let available_languages = item.available_languages.clone();

                let mut available_languages_str = String::new();
                for i in available_languages.iter() {
                    available_languages_str += &format!("{}, ", i);
                }
                available_languages_str = available_languages_str
                    .trim_end_matches(", ")
                    .to_string();
                let cover = fs::metadata(format!("{}\\_cover.png", mwd)).is_ok();
                let chapters: Vec<String> = item.chapters
                    .iter()
                    .map(|d| d.number.clone())
                    .collect();

                let mut chapter_str = String::new();

                for i in chapters.iter() {
                    chapter_str.push_str(&format!("{}, ", i.to_string().replace("\"", "")));
                }
                chapter_str = chapter_str.trim_end_matches(", ").to_string();

                println!("Manga name: {}", manga_name);
                println!("MWD: {}", mwd);
                println!("ID: {}", id);
                println!("Database fetched: {}", date_str);
                if !genres.is_empty() {
                    println!("Genres: {}", genre_str);
                }
                if !themes.is_empty() {
                    println!("Themes: {}", theme_str);
                }
                println!("Cover: {}", cover);
                println!("Language: {}", language);
                println!("Available language: {}", available_languages_str);
                println!("Chapters: {}", chapter_str);
                println!();

                if args::ARGS_SHOW_ALL.is_some() {
                    let mut chapters = vec![];
                    if let Ok(entries) = fs::read_dir(&mwd) {
                        for entry in entries.flatten() {
                            let file_name = entry.file_name();
                            if let Some(name) = file_name.to_str() {
                                if name.ends_with(".cbz") {
                                    chapters.push(name.to_string());
                                }
                            }
                        }
                    }
                    if !chapters.is_empty() {
                        for entry in chapters {
                            let path = format!("{}\\{}", mwd, entry);
                            let obj = match check_for_metadata(&path) {
                                Ok(metadata) => metadata,
                                Err(err) => {
                                    return Err(err);
                                }
                            };

                            let name = obj.name;

                            let pages = obj.pages;

                            let id = obj.id;

                            let title = obj.title;

                            let chapter = obj.chapter;

                            let volume = obj.volume;

                            println!("Name: {}", name);
                            if volume != "null" {
                                println!("Volume: {}", volume);
                            }
                            println!("Chapter: {}", chapter);
                            println!("Pages: {}", pages);
                            println!("ID: {}", id);
                            println!("Title: {}", title);
                            println!();
                        }
                    } else {
                        println!("No chapters found");
                    }
                }
            }
        }
        Err(err) => {
            return Err(MdownError::JsonError(err.to_string()));
        }
    }

    Ok(())
}

pub(crate) fn check_for_metadata_saver(file_path: &str) -> Result<bool, MdownError> {
    // Returns true if cbz file saver is different than the current one
    let obj = match check_for_metadata(file_path) {
        Ok(metadata) => metadata,
        Err(err) => {
            return Err(err);
        }
    };
    let saver = obj.saver;
    if *SAVER.lock() != saver {
        return Ok(true);
    }
    Ok(false)
}

pub(crate) fn check_for_metadata(
    file_path: &str
) -> Result<metadata::ChapterMetadataIn, MdownError> {
    let metadata_file_name = "_metadata";

    zip_func::extract_file_from_zip(file_path, metadata_file_name)
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
            return Err(MdownError::IoError(err, dat_path));
        }
    }
    let mut json = match get_dat_content(dat_path.as_str()) {
        Ok(value) => value,
        Err(error) => {
            return Err(error);
        }
    };

    json = match serde_json::from_value::<Dat>(json) {
        Ok(mut dat) => {
            let data = &mut dat.data;
            let mut iter: i32 = -1;
            let mut to_remove = vec![];
            for item in data.iter_mut() {
                *MUSIC_STAGE.lock() = String::from("init");
                iter += 1;
                let manga_name = item.name.clone();
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
                        return Err(MdownError::IoError(err, String::new()));
                    }
                };
                let mwd: String = item.mwd.clone();

                *LANGUAGE.lock() = item.current_language.clone();
                if std::env::set_current_dir(&mwd).is_err() {
                    println!("{} not found; deleting from database", &manga_name);
                    to_remove.push(iter);
                    continue;
                }

                match std::fs::rename(format!("{}\\.cache", past_mwd), format!("{}\\.cache", mwd)) {
                    Ok(()) => (),
                    Err(err) => {
                        eprintln!("Error: moving MWD from {} to {} {}", past_mwd, mwd, err);
                    }
                }
                let id = item.id.clone();
                let cover_file = format!("{}\\_cover.png", mwd);
                let mut cover = fs::metadata(cover_file).is_ok();
                if let Ok(manga_name_json) = getter::get_manga_json(&id).await {
                    match utils::get_json(&manga_name_json) {
                        Ok(obj) => {
                            let empty = Value::String(String::new());
                            let cover_data: &str = match
                                obj
                                    .get("data")
                                    .and_then(|name_data| name_data.get("relationships"))
                                    .and_then(Value::as_array)
                                    .map(|data| {
                                        let mut cover_data = "";
                                        for el in data {
                                            if
                                                (match el.get("type") {
                                                    Some(cover_dat) => cover_dat,
                                                    None => &empty,
                                                }) == "cover_art"
                                            {
                                                cover_data = el
                                                    .get("attributes")
                                                    .and_then(|dat| dat.get("fileName"))
                                                    .and_then(Value::as_str)
                                                    .unwrap_or_default();
                                            }
                                        }
                                        cover_data
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
                                            String::from(
                                                "Didn't find attributes property (title_data)"
                                            )
                                        )
                                    );
                                }
                            };
                            let chapters_temp = item.chapters.clone();
                            let mut chapter_da = CHAPTER_DATES.lock();
                            let mut chapter_id = CHAPTER_IDS.lock();
                            for i in chapters_temp.iter() {
                                let number = i.number.clone();
                                let date = i.updated_at.clone();
                                let id = i.id.clone();
                                chapter_da.insert(number.clone(), date);
                                chapter_id.insert(number, id);
                            }
                            drop(chapter_da);
                            drop(chapter_id);

                            if *args::ARGS_UPDATE && !cover {
                                let m_name = get_manga_name(title_data);
                                let folder = get_folder_name(&m_name);
                                *COVER.lock() = match
                                    download::download_cover(
                                        Arc::from("https://uploads.mangadex.org/"),
                                        Arc::from(id.as_str()),
                                        Arc::from(cover_data),
                                        Arc::from(folder)
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
                            *MANGA_NAME.lock() = get_manga_name(title_data);
                            match
                                resolve_manga(&id, get_manga_name(title_data).as_str(), false).await
                            {
                                Ok(()) => (),
                                Err(err) => {
                                    handle_error!(&err, String::from("manga"));
                                }
                            }
                        }
                        Err(err) => {
                            return Err(err);
                        }
                    };
                }
                if *args::ARGS_UPDATE {
                    item.cover = if !cover { *COVER.lock() } else { true };
                }
                let mut chapters_temp = item.chapters.clone();
                let chapters_remove = CHAPTERS_TO_REMOVE.lock();
                for i in chapters_remove.iter() {
                    chapters_temp.retain(|value| {
                        let number = value.number.clone();
                        let date = value.updated_at.clone();
                        let id = value.id.clone();
                        ChapterMetadata::new(&number, &date, &id) != *i
                    });
                }
                drop(chapters_remove);
                let mut chapters = Vec::new();
                for i in chapters_temp.iter() {
                    let number = i.number.clone();
                    let date = i.updated_at.clone();
                    let id = i.id.clone();
                    chapters.push(ChapterMetadata::new(&number, &date, &id));
                }

                for i in CHAPTERS.lock().iter() {
                    if !chapters.contains(i) {
                        chapters.push(i.clone());
                    }
                }
                item.chapters = chapters;

                if item.chapters.is_empty() && !cover {
                    println!("{} not found; deleting from database", &manga_name);
                    to_remove.push(iter);
                    continue;
                }

                if *args::ARGS_CHECK {
                    println!("Checked {} ({})", &manga_name, item.id);
                    let to_dow;
                    if !TO_DOWNLOAD.lock().is_empty() || !TO_DOWNLOAD_DATE.lock().is_empty() {
                        to_dow = true;
                        println!("Chapters available");
                        for chapter in TO_DOWNLOAD.lock().iter() {
                            println!(" {}", chapter);
                        }
                        for chapter in TO_DOWNLOAD_DATE.lock().iter() {
                            println!(" {} (OUTDATED CHAPTER)", chapter);
                        }
                    } else if !FIXED_DATES.lock().is_empty() {
                        to_dow = false;
                        println!("Chapters ERROR");
                        for date in FIXED_DATES.lock().iter() {
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
                CHAPTERS.lock().clear();
                TO_DOWNLOAD.lock().clear();
                TO_DOWNLOAD_DATE.lock().clear();
                FIXED_DATES.lock().clear();
            }
            *MUSIC_STAGE.lock() = String::from("end");
            *MUSIC_END.lock() = true;
            for &index in to_remove.iter().rev() {
                data.remove(index as usize);
            }
            match serde_json::to_value(dat) {
                Ok(value) => value,
                Err(err) => {
                    return Err(MdownError::JsonError(err.to_string()));
                }
            }
        }
        Err(err) => {
            return Err(MdownError::JsonError(err.to_string()));
        }
    };

    let mut file = match File::create(&dat_path) {
        Ok(path) => path,
        Err(err) => {
            return Err(MdownError::IoError(err, dat_path));
        }
    };

    let json_string = match serde_json::to_string_pretty(&json) {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::JsonError(err.to_string()));
        }
    };

    if let Err(err) = writeln!(file, "{}", json_string) {
        return Err(MdownError::IoError(err, dat_path));
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
                return Err(MdownError::IoError(err, dat_path));
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

    json = match serde_json::from_value::<Dat>(json.clone()) {
        Ok(mut dat) => {
            let data = &mut dat.data;

            let manga_names: Vec<String> = data
                .iter()
                .map(|item| item.name.clone())
                .collect();
            if data.is_empty() || !manga_names.contains(&MANGA_NAME.lock().clone()) {
                let mwd = format!("{}", MWD.lock());
                let cover = COVER.lock();
                let mut chapters = Vec::new();
                let chapters_data: Vec<ChapterMetadata> = CHAPTERS.lock().clone();
                for i in chapters_data.iter() {
                    chapters.push(match serde_json::to_value(i) {
                        Ok(v) => v,
                        Err(err) => {
                            return Err(MdownError::JsonError(err.to_string()));
                        }
                    });
                }
                let mut genres = Vec::new();
                let genres_data = GENRES.lock().clone();
                for i in genres_data.iter() {
                    genres.push(match serde_json::to_value(i) {
                        Ok(v) => v,
                        Err(err) => {
                            return Err(MdownError::JsonError(err.to_string()));
                        }
                    });
                }
                let mut themes = Vec::new();
                let themes_data = THEMES.lock().clone();
                for i in themes_data.iter() {
                    themes.push(match serde_json::to_value(i) {
                        Ok(v) => v,
                        Err(err) => {
                            return Err(MdownError::JsonError(err.to_string()));
                        }
                    });
                }
                let manga_data = MangaMetadata::new(
                    &MANGA_NAME.lock().clone(),
                    &MANGA_ID.lock().clone(),
                    chapters_data,
                    &mwd,
                    *cover,
                    DATE_FETCHED.lock().clone(),
                    LANGUAGES.lock().clone(),
                    &LANGUAGE.lock().clone(),
                    themes_data,
                    genres_data
                );

                data.push(manga_data);
            } else {
                for chap_data in data.iter_mut() {
                    let name = &chap_data.name;
                    if name == MANGA_NAME.lock().as_str() {
                        let existing_chapters = &mut chap_data.chapters;

                        let mut existing_chapters_temp = Vec::new();

                        for i in existing_chapters.iter_mut() {
                            let number = &i.number;
                            existing_chapters_temp.push(number);
                        }

                        let mut new_chapters: Vec<_> = CHAPTERS.lock()
                            .iter()
                            .filter(|&chapter| {
                                let number = chapter.number.clone();
                                !existing_chapters_temp.contains(&&number)
                            })
                            .cloned()
                            .collect();

                        new_chapters.sort_by(|a, b| {
                            let a_num = match a.number.parse::<u32>() {
                                Ok(value) => value,
                                Err(_err) => 0,
                            };
                            let b_num = match b.number.parse::<u32>() {
                                Ok(value) => value,
                                Err(_err) => 0,
                            };
                            a_num.cmp(&b_num)
                        });

                        for i in new_chapters.iter() {
                            existing_chapters.push(i.clone());
                        }

                        break;
                    }
                }
            }
            match serde_json::to_value(dat) {
                Ok(json) => json,
                Err(err) => {
                    return Err(MdownError::JsonError(err.to_string()));
                }
            }
        }
        Err(err) => {
            return Err(MdownError::JsonError(err.to_string()));
        }
    };

    let mut file = match File::create(&dat_path) {
        Ok(file) => file,
        Err(err) => {
            return Err(MdownError::IoError(err, dat_path));
        }
    };

    let json_string = match serde_json::to_string_pretty(&json) {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::JsonError(err.to_string()));
        }
    };

    if let Err(err) = writeln!(file, "{}", json_string) {
        return Err(MdownError::JsonError(err.to_string()));
    }
    Ok(())
}

pub(crate) fn get_dat_content(dat_path: &str) -> Result<Value, MdownError> {
    let file = File::open(dat_path);
    let mut file = match file {
        Ok(file) => file,
        Err(err) => {
            return Err(MdownError::IoError(err, dat_path.to_string()));
        }
    };
    let mut contents = String::new();
    if let Err(err) = file.read_to_string(&mut contents) {
        return Err(MdownError::IoError(err, dat_path.to_string()));
    }
    utils::get_json(&contents)
}

pub(crate) async fn resolve(obj: Map<String, Value>, id: &str) -> Result<String, MdownError> {
    let handle_id = utils::generate_random_id(16);
    *HANDLE_ID.lock() = handle_id.clone();
    let title_data = match obj.get("data").and_then(|name_data| name_data.get("attributes")) {
        Some(value) => value,
        None => {
            return Err(MdownError::NotFoundError(String::from("resolve")));
        }
    };

    let manga_name = if ARGS.lock().title == "*" {
        debug!("manga name using functions");
        get_manga_name(title_data)
    } else {
        debug!("manga name is user defined");
        ARGS.lock().title.to_string()
    };
    *MANGA_NAME.lock() = manga_name.clone();
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
    let current_lang = LANGUAGE.lock().to_string();
    if
        current_lang != orig_lang &&
        !final_lang.contains(&current_lang.as_str()) &&
        current_lang != "*"
    {
        debug!("defined language not found in manga information");
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
        string(1, 0, &format!("Language is not available\nSelected language: {}", LANGUAGE.lock()));
        string(3, 0, &format!("Original language: {}", orig_lang));
        string(4, 0, &format!("Available languages: {}", langs));
        string(5, 0, &format!("Choose from these    {}", "^".repeat(lang_range)));
        return Ok(manga_name);
    }
    drop(current_lang);
    *DOWNLOADING.lock() = true;

    let was_rewritten = fs::metadata(folder).is_ok();
    match fs::create_dir(folder) {
        Ok(()) => (),
        Err(err) => {
            if err.raw_os_error().unwrap_or_default() != 183 {
                eprintln!("Error: creating directory {} {}", &folder, err);
            }
        }
    }
    debug!("created directory {}", folder);
    *MWD.lock() = match std::fs::canonicalize(folder) {
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
            return Err(MdownError::IoError(err, folder.to_string()));
        }
    };
    let desc = title_data
        .get("description")
        .and_then(|description| description.get("en"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut desc_file = match
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("{}\\_description.txt", folder))
    {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::IoError(err, format!("{}\\_description.txt", folder)));
        }
    };
    match write!(desc_file, "{}", desc) {
        Ok(()) => (),
        Err(err) => eprintln!("Error: writing in description file {}", err),
    }

    debug!("created description file");

    let empty_vec = vec![];

    let tags_attributes = match title_data.get("tags").and_then(Value::as_array) {
        Some(value) => value,
        None => &empty_vec,
    };

    let mut theme: Vec<TagMetadata> = vec![];
    let mut genre: Vec<TagMetadata> = vec![];

    for tag in tags_attributes.iter() {
        let id = tag.get("id").and_then(Value::as_str).unwrap_or_default();
        let attr = tag.get("attributes");
        if let Some(attr) = attr {
            let typ = attr.get("group").and_then(Value::as_str).unwrap_or_default();
            let name = attr
                .get("name")
                .and_then(|value| value.get("en"))
                .and_then(Value::as_str)
                .unwrap_or_default();
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

    *GENRES.lock() = genre;
    *THEMES.lock() = theme;

    let folder = get_folder_name(&manga_name);
    let cover = obj
        .get("data")
        .and_then(|name_data| name_data.get("relationships"))
        .and_then(Value::as_array)
        .map(|data| {
            let mut cover = "";
            for el in data {
                if el.get("type").and_then(Value::as_str).unwrap_or_default() == "cover_art" {
                    cover = el
                        .get("attributes")
                        .and_then(|dat| dat.get("fileName"))
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                }
            }
            cover
        })
        .unwrap_or_default();
    if !cover.is_empty() {
        debug!("starting downloading cover");
        *COVER.lock() = match
            download::download_cover(
                Arc::from("https://uploads.mangadex.org/"),
                Arc::from(id),
                Arc::from(cover),
                Arc::from(folder)
            ).await
        {
            Ok(()) => true,
            Err(err) => {
                eprintln!("Error: failed to download cover {}", err);
                false
            }
        };
        debug!("cover downloaded successfully");
    }

    if ARGS.lock().stat {
        debug!("starting downloading stat");
        match download::download_stat(id, folder, &manga_name).await {
            Ok(()) => (),
            Err(err) => {
                handle_error!(&err, String::from("statistics"));
            }
        }
        debug!("stat downloaded successfully");
    }

    *LANGUAGES.lock() = {
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

    match resolve_manga(id, &manga_name, was_rewritten).await {
        Ok(()) => (),
        Err(err) => {
            handle_error!(&err, String::from("program"));
        }
    }
    log_end(handle_id);
    if
        *args::ARGS_WEB ||
        *args::ARGS_GUI ||
        *args::ARGS_CHECK ||
        *args::ARGS_UPDATE ||
        *args::ARGS_LOG
    {
        log!("Downloaded manga");
    }
    *DOWNLOADING.lock() = false;
    *MUSIC_STAGE.lock() = String::from("end");
    CHAPTERS.lock().clear();
    MANGA_ID.lock().clear();
    CURRENT_CHAPTER.lock().clear();
    *CURRENT_PAGE.lock() = 0;
    *CURRENT_PAGE_MAX.lock() = 0;
    *CURRENT_PERCENT.lock() = 0.0;
    *CURRENT_SIZE.lock() = 0.0;
    *CURRENT_SIZE_MAX.lock() = 0.0;
    *CURRENT_CHAPTER_PARSED.lock() = 0;
    *CURRENT_CHAPTER_PARSED_MAX.lock() = 0;
    debug!("global variables reset");
    Ok(manga_name)
}

pub(crate) async fn resolve_group(
    array_item: &metadata::ChapterResponse
) -> Result<(String, String), MdownError> {
    let scanlation_group = array_item.relationships.clone();
    let scanlation_group_id = match get_scanlation_group(&scanlation_group) {
        Some(value) => value,
        None => {
            SUSPENDED.lock().push(MdownError::NotFoundError(String::from("resolve_group")));
            return Ok((String::from("null"), String::from("null")));
        }
    };
    if scanlation_group_id.is_empty() {
        return Ok((String::from("null"), String::from("null")));
    }

    let (name, website) = match resolve_group_metadata(&scanlation_group_id).await {
        Ok((name, website)) => (name, website),
        Err(err) => {
            return Err(err);
        }
    };
    if name != "Unknown" && !SCANLATION_GROUPS.lock().contains_key(&scanlation_group_id) {
        SCANLATION_GROUPS.lock().insert(scanlation_group_id, name.clone());
    }
    Ok((name, website))
}

pub(crate) fn get_scanlation_group_to_file(
    manga_name: &str,
    name: &str,
    website: &str
) -> Result<(), MdownError> {
    if name == "null" {
        return Ok(());
    }
    let file_name = format!("{}\\_scanlation_groups.txt", get_folder_name(manga_name));

    let mut file_inst = match OpenOptions::new().create(true).append(true).open(&file_name) {
        Ok(file_inst) => file_inst,
        Err(err) => {
            return Err(MdownError::IoError(err, file_name));
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
                            MdownError::NotFoundError("data in resolve_group_metadata".to_string())
                        );
                    }
                };
                let attr = match data.get("attributes") {
                    Some(value) => value,
                    None => {
                        return Err(
                            MdownError::NotFoundError(
                                "attributes in resolve_group_metadata".to_string()
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
                let website = attr
                    .get("website")
                    .and_then(Value::as_str)
                    .unwrap_or("None")
                    .to_owned();
                return Ok((name, website));
            }
            _ => {
                return Ok((String::from("Unknown"), String::new()));
            }
        }
    }
    Err(MdownError::NetworkError(response.error_for_status().unwrap_err()))
}

async fn resolve_manga(id: &str, manga_name: &str, was_rewritten: bool) -> Result<(), MdownError> {
    let going_offset: u32 = match ARGS.lock().database_offset.as_str().parse() {
        Ok(offset) => offset,
        Err(err) => {
            return Err(MdownError::ConversionError(err.to_string()));
        }
    };
    let arg_force = ARGS.lock().force;
    let downloaded: &mut Vec<String> = &mut vec![];
    *MANGA_ID.lock() = id.to_owned();
    match get_manga(id, going_offset).await {
        Ok((json, _offset)) => {
            clear_screen(1);
            let downloaded_temp = match download_manga(json, manga_name, arg_force).await {
                Ok(value) => value,
                Err(err) => {
                    return Err(err);
                }
            };
            for i in &downloaded_temp {
                downloaded.push(i.clone());
            }
            clear_screen(1);
        }
        Err(err) => eprintln!("Error: {}", err),
    }
    if !*args::ARGS_WEB && !*args::ARGS_GUI && !*args::ARGS_CHECK && !*args::ARGS_UPDATE {
        if !downloaded.is_empty() {
            string(1, 0, "Downloaded files:");
            for i in 0..downloaded.len() {
                resolve_move(i as u32, downloaded, 2, 1);
            }
        } else if !was_rewritten {
            match fs::remove_dir_all(get_folder_name(manga_name)) {
                Ok(()) => (),
                Err(err) => eprintln!("Error: remove directory {}", err),
            };
        }
    }
    Ok(())
}

pub(crate) fn resolve_move(mut moves: u32, hist: &mut Vec<String>, start: u32, end: u32) -> u32 {
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
                start + i,
                0,
                &format!("{}{}", message, " ".repeat((MAXPOINTS.max_x as usize) - message.len()))
            );
        } else {
            string(start + i, 0, &message.to_string());
        }
    }
    moves
}

pub(crate) fn title(mut title: String) -> String {
    if title.chars().last().unwrap_or('0') == '.' {
        title = title[..title.len() - 1].to_string();
    }
    title
}

pub(crate) fn resolve_skip(arg: &str, with: &str) -> bool {
    if arg == "*" || arg == with {
        return false;
    }
    true
}
