use chrono::prelude::*;
use crosscurses::*;
use rand::{ Rng, distributions::Alphanumeric };
use regex::Regex;
use serde_json::{ Value, json };
use std::{
    cmp::Ordering,
    fs::{ self, File, OpenOptions },
    io::{ Read, Write },
    path::Path,
    process::exit,
    thread::sleep,
    time::{ Duration, Instant },
};
use uuid::Uuid;

use crate::{
    ARGS,
    download,
    error::{ mdown::Error, handle_error },
    getter,
    IS_END,
    log,
    MAXPOINTS,
    metadata,
    resolute::{ self, resolve_move, CURRENT_PERCENT, CURRENT_SIZE, CURRENT_SIZE_MAX },
    string,
};

pub(crate) fn setup_requirements(file_path_tm: String) {
    let _ = initscr();
    curs_set(2);
    start_color();
    let file_path_temp = file_path_tm.clone();
    tokio::spawn(async move { print_version(file_path_tm).await });
    tokio::spawn(async move { ctrl_handler(file_path_temp).await });
}

pub(crate) fn log_handler() {
    let path = match getter::get_log_path() {
        Ok(path) => path,
        Err(_err) => {
            return;
        }
    };
    let lock_path = match getter::get_log_lock_path() {
        Ok(path) => path,
        Err(_err) => {
            return;
        }
    };

    if fs::metadata(&lock_path).is_ok() {
        remove_log_lock_file();
    }

    loop {
        sleep(Duration::from_millis(100));

        let _ = if fs::metadata(&path).is_err() {
            let mut file = match fs::File::create(&path) {
                Ok(file) => file,
                Err(_err) => {
                    continue;
                }
            };

            let content = String::from("{}");

            match file.write_all(content.as_bytes()) {
                Ok(()) => (),
                Err(_err) => (),
            };
        };
        // prettier-ignore or #[rustfmt::skip]
        if match resolute::ENDED.lock() {
                Ok(value) => { *value }
                Err(_err) => {
                    sleep(Duration::from_millis(100));
                    false
                }
            }
        {
            remove_log_lock_file();
            return;
        }

        while fs::metadata(&lock_path).is_ok() {
            sleep(Duration::from_millis(10));
        }
        let _ = File::create(&lock_path);
        let mut json = match resolute::get_dat_content(path.as_str()) {
            Ok(value) => value,
            Err(_err) => json!({}),
        };

        let mut messages_lock = match resolute::LOGS.lock() {
            Ok(value) => value,
            Err(_err) => {
                continue;
            }
        };
        let mut handle_id_lock = match resolute::HANDLE_ID_END.lock() {
            Ok(value) => value,
            Err(_err) => {
                continue;
            }
        };

        let char = vec!["\\n", "\\r", "\\t", "\\\\", "\\'", "\\\"", "\\0"];

        let messages: Vec<metadata::LOG> = messages_lock
            .clone()
            .iter()
            .map(|message| {
                let mut message = message.clone();
                for c in char.iter() {
                    message.message = message.message.replace(c, "").to_string();
                }
                message
            })
            .collect();

        if let Some(data) = json.as_object_mut() {
            for message in messages.iter() {
                let handle_id = message.handle_id.to_string();
                let chap_num = message.name.to_string();
                if handle_id == String::new() {
                    continue;
                }
                let mut inst: Vec<Value> = Vec::new();
                let mut map: serde_json::Map<String, Value> = serde_json::Map::new();
                if
                    let Some(value) = data
                        .get_mut(&handle_id.to_string())
                        .and_then(|value| value.get_mut("logs"))
                        .and_then(|value| value.get_mut(chap_num.clone()))
                        .and_then(Value::as_array_mut)
                {
                    inst.extend_from_slice(value);
                }
                if
                    let Some(value) = data
                        .get_mut(&handle_id.to_string())
                        .and_then(|value| value.get_mut("logs"))
                        .and_then(Value::as_object_mut)
                {
                    map = value.clone();
                }
                let start_time = {
                    if
                        let Some(time) = data
                            .get(&handle_id.to_string())
                            .and_then(|value| value.get("time_start"))
                            .and_then(Value::as_str)
                    {
                        time.to_string()
                    } else {
                        Utc::now().to_rfc3339()
                    }
                };
                inst.push(
                    Value::String(format!("{}  {}", message.time.clone(), message.message.clone()))
                );

                map.insert(chap_num.clone(), serde_json::Value::Array(inst.clone()));

                match handle_id.len() {
                    10 => {
                        data.insert(
                            handle_id.to_string(),
                            json!({"logs":map, "type":"web", "time_start": start_time, "time_end": null})
                        );
                    }
                    16 => {
                        let manga_name = match resolute::MANGA_NAME.lock() {
                            Ok(value) => Value::String(value.clone()),
                            Err(_err) => Value::Null,
                        };
                        let manga_id = match resolute::MANGA_ID.lock() {
                            Ok(value) => Value::String(value.clone()),
                            Err(_err) => Value::Null,
                        };
                        let mwd = match resolute::MWD.lock() {
                            Ok(value) => Value::String(value.clone()),
                            Err(_err) => Value::Null,
                        };
                        data.insert(
                            handle_id.to_string(),
                            json!({"logs":map, "type":"downloader", "time_start": start_time, "time_end": null, "name": manga_name, "id": manga_id, "mwd": mwd})
                        );
                    }
                    _ => {
                        data.insert(
                            handle_id.to_string(),
                            json!({"logs":map, "type":"unknown", "time_start": start_time, "time_end": null})
                        );
                    }
                }
            }
            for handle_id in handle_id_lock.iter() {
                if handle_id == &String::new().into_boxed_str() {
                    continue;
                }
                let end_time = Utc::now().to_rfc3339();
                if
                    let Some(handle) = data
                        .get_mut(&handle_id.to_string())
                        .and_then(|value| value.get_mut("time_end"))
                {
                    *handle = Value::String(end_time);
                }
            }
        }
        let mut file = match File::create(&path) {
            Ok(file) => file,
            Err(_err) => {
                continue;
            }
        };

        let json_string = match serde_json::to_string_pretty(&json) {
            Ok(value) => value,
            Err(_err) => {
                continue;
            }
        };

        let _ = writeln!(file, "{}", json_string);
        *messages_lock = vec![];
        *handle_id_lock = vec![];
        drop(messages_lock);
        drop(handle_id_lock);

        remove_log_lock_file();
    }
}

pub(crate) fn remove_cache() -> Result<(), Error> {
    if is_directory_empty(".cache\\") {
        match fs::remove_dir_all(".cache") {
            Ok(()) => (),
            Err(err) => {
                (
                    match resolute::SUSPENDED.lock() {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(Error::PoisonError(err.to_string()));
                        }
                    }
                ).push(Error::IoError(err, Some(String::from(".cache\\"))));
            }
        };
    }
    Ok(())
}

pub(crate) fn setup_subscriber() -> Result<(), Error> {
    let subscriber = tracing_subscriber
        ::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .finish();
    match tracing::subscriber::set_global_default(subscriber) {
        Ok(()) => Ok(()),
        Err(err) => {
            eprintln!("Error: tracing_subscriber {:?}", err);
            (
                match resolute::SUSPENDED.lock() {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(Error::PoisonError(err.to_string()));
                    }
                }
            ).push(
                Error::CustomError(
                    String::from("Failed to set up tracing_subscriber (basically info)"),
                    String::from("SubscriberError")
                )
            );
            Ok(())
        }
    }
}

pub(crate) fn create_cache_folder() -> Result<(), Error> {
    match fs::create_dir(".cache") {
        Ok(()) => Ok(()),
        Err(err) => {
            (
                match resolute::SUSPENDED.lock() {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(Error::PoisonError(err.to_string()));
                    }
                }
            ).push(Error::IoError(err, Some(String::from(".cache\\"))));
            Ok(())
        }
    }
}

pub(crate) fn is_valid_uuid(s: &str) -> bool {
    match Uuid::parse_str(s) {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub(crate) fn clear_screen(from: u32) {
    if !ARGS.web && !ARGS.gui && !ARGS.check && !ARGS.update {
        for i in from..MAXPOINTS.max_y {
            string(i, 0, &" ".repeat(MAXPOINTS.max_x as usize));
        }
    }
}

pub(crate) fn process_filename(filename: &str) -> String {
    filename
        .replace('<', "")
        .replace('>', "")
        .replace(':', "")
        .replace('|', "")
        .replace('?', "")
        .replace('*', "")
        .replace('/', "")
        .replace('\\', "")
        .replace('"', "")
}

pub(crate) async fn wait_for_end(file_path: String, images_length: usize) -> Result<(), Error> {
    let full_path = format!(".cache\\{}.lock", file_path);
    let mut full_size = 0.0;
    let start = Instant::now();
    while fs::metadata(full_path.clone()).is_ok() {
        let mut size = 0.0;
        for i in 1..images_length + 1 {
            let image_name = format!(".cache\\{}_{}.lock", file_path, i);
            if fs::metadata(image_name.clone()).is_ok() {
                let mut image_file = match File::open(image_name.clone()) {
                    Ok(image) => image,
                    Err(_err) => {
                        continue;
                    }
                };
                let mut image_content = String::new();
                match image_file.read_to_string(&mut image_content) {
                    Ok(_size) => (),
                    Err(err) => eprintln!("Error: reading input {}", err),
                }
                if image_content != "" {
                    let image_content: f64 = match image_content.parse() {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(Error::ConversionError(err.to_string()));
                        }
                    };
                    size += image_content;
                }
            }
        }
        for i in 1..images_length + 1 {
            let image_name = format!(".cache\\{}_{}_final.lock", file_path, i);
            if fs::metadata(image_name.clone()).is_ok() {
                let mut image_file = match File::open(image_name.clone()) {
                    Ok(image) => image,
                    Err(_err) => {
                        continue;
                    }
                };
                let mut image_content = String::new();
                match image_file.read_to_string(&mut image_content) {
                    Ok(_size) => (),
                    Err(err) => eprintln!("Error: reading input {}", err),
                }
                if image_content != "" {
                    let image_content: f64 = match image_content.parse() {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(Error::ConversionError(err.to_string()));
                        }
                    };
                    full_size += image_content / 1024.0 / 1024.0;
                    match fs::remove_file(image_name.clone()) {
                        Ok(()) => (),
                        Err(err) => eprintln!("Error: removing _final.lock {}", err),
                    };
                }
            }
        }
        let percent;
        if full_size == 0.0 {
            percent = 0.0;
        } else {
            percent = (100.0 / full_size) * size;
        }
        *(match CURRENT_PERCENT.lock() {
            Ok(value) => value,
            Err(err) => {
                return Err(Error::PoisonError(err.to_string()));
            }
        }) = percent;
        *(match CURRENT_SIZE.lock() {
            Ok(value) => value,
            Err(err) => {
                return Err(Error::PoisonError(err.to_string()));
            }
        }) = size;
        *(match CURRENT_SIZE_MAX.lock() {
            Ok(value) => value,
            Err(err) => {
                return Err(Error::PoisonError(err.to_string()));
            }
        }) = full_size;
        string(
            4,
            MAXPOINTS.max_x - 60,
            &format!(
                "{:.2}% {:.2}mb/{:.2}mb [{:.2}mb remaining] [{:.2}s]",
                percent,
                size,
                full_size,
                (full_size - size).abs(),
                (Instant::now() - start).as_secs_f64().abs()
            )
        );
    }

    for i in 1..images_length + 1 {
        let image_name = format!(".cache\\{}_{}.lock", file_path, i);
        if fs::metadata(&image_name).is_ok() {
            match fs::remove_file(&image_name) {
                Ok(()) => (),
                Err(err) => eprintln!("Error: removing file '{}' {}", image_name, err),
            };
        }
    }
    Ok(())
}

pub(crate) fn progress_bar_preparation(start: u32, images_length: usize, line: u32) {
    if !ARGS.web && !ARGS.gui && !ARGS.check && !ARGS.update && !ARGS.server {
        string(line, 0, &format!("{}|", &"-".repeat((start as usize) - 1)));
        string(
            line,
            start + (images_length as u32),
            &format!(
                "|{}",
                &"-".repeat(
                    (MAXPOINTS.max_x as usize) - ((start + (images_length as u32) + 1) as usize)
                )
            )
        );
    }
}
pub(crate) fn sort(data: &Vec<Value>) -> Vec<Value> {
    let mut data_array = data.to_owned();

    if ARGS.unsorted {
        return data.to_vec();
    }

    data_array.sort_unstable_by(|v, b| {
        v.get("attributes")
            .and_then(|attr| attr.get("chapter"))
            .and_then(|chapter| chapter.as_str())
            .and_then(|chapter_str| chapter_str.parse::<f32>().ok())
            .map(|v_parsed| {
                b.get("attributes")
                    .and_then(|attr| attr.get("chapter"))
                    .and_then(|chapter| chapter.as_str())
                    .and_then(|chapter_str| chapter_str.parse::<f32>().ok())
                    .map(|b_parsed| v_parsed.total_cmp(&b_parsed))
                    .unwrap_or(Ordering::Equal)
            })
            .unwrap_or(Ordering::Equal)
    });

    data_array
}

pub(crate) fn remove_log_lock_file() {
    let lock_path = match getter::get_log_lock_path() {
        Ok(path) => path,
        Err(_err) => {
            return;
        }
    };
    let _ = fs::remove_file(lock_path);
}

pub(crate) fn get_json(manga_name_json: &str) -> Result<Value, Error> {
    match serde_json::from_str(&manga_name_json) {
        Ok(value) => Ok(value),
        Err(err) => Err(Error::JsonError(err.to_string())),
    }
}

pub(crate) async fn search() -> Result<String, Error> {
    let id;
    let base_url = "https://api.mangadex.org";
    let title = ARGS.search.clone();

    let client = match download::get_client() {
        Ok(client) => client,
        Err(err) => {
            return Err(Error::NetworkError(err));
        }
    };

    let response = match
        client
            .get(&format!("{}/manga", base_url))
            .query(&[("title", title)])
            .send().await
    {
        Ok(response) => response,
        Err(err) => {
            return Err(Error::NetworkError(err));
        }
    };

    if response.status().is_success() {
        let manga_data: serde_json::Value = match response.json().await {
            Ok(value) => value,
            Err(err) => {
                return Err(Error::JsonError(err.to_string()));
            }
        };

        let data = match manga_data.get("data") {
            Some(data) => data,
            None => {
                return Err(Error::NotFoundError(String::from("data in manga_data in main.rs")));
            }
        };
        let manga_array = match data.as_array() {
            Some(data) => data,
            None => {
                return Err(Error::ConversionError(String::from("manga_data to array in main.rs")));
            }
        };
        let manga_ids: Vec<&serde_json::Value> = manga_array
            .iter()
            .map(|manga| &manga["id"])
            .collect();
        let manga_ids: Vec<&str> = manga_ids
            .iter()
            .filter_map(|id| id.as_str())
            .collect();
        id = (
            match manga_ids.first() {
                Some(id) => id,
                None => {
                    return Err(
                        Error::NotFoundError(String::from("manga_id in manga_ids in main.rs"))
                    );
                }
            }
        ).to_string();
    } else {
        return Err(Error::StatusError(response.status()));
    }
    Ok(id)
}

pub(crate) fn resolve_start() -> Result<(String, String), Error> {
    let file_path: String = format!(".cache\\mdown_{}.lock", env!("CARGO_PKG_VERSION"));
    if ARGS.force_delete {
        match fs::remove_file(&file_path) {
            Ok(()) => println!("File has been deleted\nYou can now use it as normal"),
            Err(_err) => {
                println!("File had been already deleted");
                match remove_cache() {
                    Ok(()) => (),
                    Err(err) => eprintln!("Error: removing cache {}", err),
                }
                exit(0);
            }
        }
    }
    if fs::metadata(file_path.clone()).is_ok() {
        eprintln!(
            "Lock file has been found;\nSee README.md;\nCannot run multiple instances of mdown"
        );
        exit(100);
    }
    match File::create(file_path.clone()) {
        Ok(_) => (),
        Err(e) => {
            panic!("Error creating the file: {}", e);
        }
    }

    Ok((file_path.clone(), file_path))
}

pub(crate) fn delete_matching_directories(pattern: &Regex) -> Result<String, u32> {
    if Path::new(".cache").is_dir() {
        if let Ok(entries) = fs::read_dir(".cache") {
            let mut last_entry_path = String::new();
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if
                        pattern.is_match(
                            &(match path.clone().to_str() {
                                Some(value) => value.to_string().replace(".cache\\", ""),
                                None => String::new(),
                            })
                        ) &&
                        path.is_dir()
                    {
                        match fs::remove_dir_all(&path) {
                            Ok(()) => (),
                            Err(err) => {
                                eprintln!("Error: removing directory '{:?}' {}", &path, err);
                            }
                        }

                        last_entry_path = (
                            match path.to_str() {
                                Some(value) => value,
                                None => "__unknown",
                            }
                        )
                            .to_string()
                            .replace(".cache\\", "");
                    }
                }
            }
            return Ok(last_entry_path);
        }
    }
    Err(1)
}

pub(crate) async fn ctrl_handler(file: String) {
    if fs::metadata(".cache\\mdown_final_end.lock").is_ok() {
        match fs::remove_file(".cache\\mdown_final_end.lock") {
            Ok(()) => (),
            Err(err) => eprintln!("Error: removing file mdown_final_end.lock {}", err),
        };
    }
    loop {
        if fs::metadata(file.clone()).is_err() {
            break;
        }
        let key: Input = match stdscr().getch() {
            Some(ch) => ch,
            None => Input::Character('a'),
        };
        if key == Input::from(crosscurses::Input::Character('\u{3}')) {
            *(match IS_END.lock() {
                Ok(value) => value,
                Err(err) => {
                    handle_error(
                        &Error::PoisonError(err.to_string()),
                        String::from("ctrl_handler")
                    );
                    continue;
                }
            }) = true;
            if ARGS.log {
                log!("CTRL+C received");
                log!("CTRL+C received", "", false);
            }
            break;
        }
    }
    if
        resolve_final_end() ||
        (match resolute::ENDED.lock() {
            Ok(value) => *value,
            Err(err) => {
                handle_error(&Error::PoisonError(err.to_string()), String::from("ctrl_handler"));
                false
            }
        })
    {
        return;
    }
    clear_screen(0);
    string(0, 0, "CTRL_C: Cleaning up");
    sleep(Duration::from_secs(1));
    match fs::remove_file(&file) {
        Ok(()) => (),
        Err(_err) => (),
    }

    delete_dir();

    if is_directory_empty(".cache\\") {
        match fs::remove_dir_all(".cache") {
            Ok(()) => (),
            Err(err) => eprintln!("Error removing .cache, {}", err),
        };
    }
    exit(0);
}

pub(crate) fn resolve_final_end() -> bool {
    if fs::metadata(".cache\\mdown_final_end.lock").is_ok() {
        match fs::remove_file(".cache\\mdown_final_end.lock") {
            Ok(()) => (),
            Err(err) => eprintln!("Error: removing mdown_final_end.lock {}", err),
        }
        if is_directory_empty(".cache\\") {
            match fs::remove_dir_all(".cache") {
                Ok(()) => (),
                Err(err) => eprintln!("Error: removing .cache, {}", err),
            };
        }
        return true;
    }
    return false;
}

pub(crate) fn delete_dir() {
    let pattern = match Regex::new(r"(.*?)( - (Vol\.\d+ )?Ch\.\d+|$)") {
        Ok(value) => value,
        Err(err) => {
            (
                match resolute::SUSPENDED.lock() {
                    Ok(value) => value,
                    Err(_err) => {
                        return;
                    }
                }
            ).push(Error::RegexError(err));
            return;
        }
    };
    match delete_matching_directories(&pattern) {
        Ok(path) => {
            let pattern = r"(.+)(?: - Vol\.\d+)(?: Ch\.\d+)(?: - .+)";
            let re = match Regex::new(pattern) {
                Ok(value) => value,
                Err(err) => {
                    (
                        match resolute::SUSPENDED.lock() {
                            Ok(value) => value,
                            Err(_err) => {
                                return;
                            }
                        }
                    ).push(Error::RegexError(err));
                    return;
                }
            };
            if let Some(captures) = re.captures(&path) {
                if let Some(result) = captures.get(1) {
                    delete_dir_if_unfinished(result.as_str());
                }
            }
        }
        Err(_) => {
            "";
        }
    }

    if let Ok(entries) = fs::read_dir(".cache") {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();

                if path.is_file() && path.extension().map_or(false, |ext| ext == "lock") {
                    match fs::remove_file(&path) {
                        Ok(()) => (),
                        Err(err) => eprintln!("Error: removing file '{:?}' {}", path, err),
                    };
                }
            }
        }
    }
}

pub(crate) fn delete_dir_if_unfinished(path: &str) {
    match fs::read_dir(path) {
        Ok(entries) => {
            let mut should_delete = 0;

            for entry in entries {
                if let Ok(entry) = entry {
                    let file_path = entry.path();
                    let file_name = match file_path.file_name() {
                        Some(value) =>
                            match value.to_str() {
                                Some(value) => value,
                                None => "",
                            }
                        None => "",
                    };

                    if
                        !file_name.ends_with("_cover.png") &&
                        !file_name.ends_with("_description.txt") &&
                        !file_name.ends_with("_scanlation_groups.txt") &&
                        !file_name.ends_with("_statistics.md")
                    {
                        should_delete += 1;
                    }
                }
            }

            if should_delete == 0 {
                match fs::remove_dir_all(&path) {
                    Ok(()) => (),
                    Err(err) => eprintln!("Error: removing directory '{}' {}", path, err),
                };
            }
        }
        Err(e) => eprintln!("Error reading directory: {}", e),
    }
}

pub(crate) async fn print_version(file: String) {
    let version = env!("CARGO_PKG_VERSION");
    for _ in 0..50 {
        string(MAXPOINTS.max_y - 1, 0, &format!("Current version: {}", version));
        if fs::metadata(file.clone()).is_err() {
            break;
        }
        sleep(Duration::from_millis(100));
    }
    string(MAXPOINTS.max_y - 1, 0, &" ".repeat(MAXPOINTS.max_x as usize));
}

pub(crate) fn resolve_regex(cap: &str) -> Option<regex::Match> {
    let re = match regex::Regex::new(r"https://mangadex.org/title/([\w-]+)/?") {
        Ok(value) => value,
        Err(err) => {
            (
                match resolute::SUSPENDED.lock() {
                    Ok(value) => value,
                    Err(_err) => {
                        return None;
                    }
                }
            ).push(Error::RegexError(err));
            return None;
        }
    };
    re.captures(cap).and_then(move |id| id.get(1))
}

pub(crate) fn resolve_end(
    file_path: String,
    manga_name: String,
    status_code: reqwest::StatusCode
) -> Result<(), String> {
    match fs::remove_file(&file_path) {
        Ok(()) => (),
        Err(err) => eprintln!("Error: removing file '{}' {}", file_path, err),
    }
    match
        OpenOptions::new().read(true).write(true).create(true).open(".cache\\mdown_final_end.lock")
    {
        Ok(_file) => (),
        Err(err) => {
            return Err(format!("Error: failed to open file {}", err));
        }
    }

    sleep(Duration::from_millis(110));
    let message = if status_code.is_client_error() {
        string(0, 0, &format!("Id was not found, please recheck the id and try again"));
        format!("Ending session: {} has NOT been downloaded, because: {:?}", manga_name, match
            status_code.canonical_reason()
        {
            Some(status_code) => status_code,
            None => "Didn't find error :/",
        })
    } else if status_code.is_server_error() {
        string(
            0,
            0,
            &format!("Server error: {}: {:?}", status_code, match status_code.canonical_reason() {
                Some(status_code) => status_code,
                None => "Didn't find error :/",
            })
        );
        format!("Ending session: {} has NOT been downloaded", manga_name)
    } else if manga_name.eq("!") {
        string(
            0,
            0,
            &format!(
                "Either --url or --search was not specified or website is not in pattern of https://mangadex.org/title/[id]/"
            )
        );
        string(1, 0, "See readme.md for more information");
        string(2, 0, "Or use --help");
        format!("Ending session: {} has NOT been downloaded, because it was not found", manga_name)
    } else {
        format!("Ending session: {} has been downloaded", manga_name)
    };

    string(
        MAXPOINTS.max_y - 1,
        0,
        &(message.clone() + &" ".repeat((MAXPOINTS.max_x as usize) - message.len()))
    );
    Ok(())
}

pub(crate) fn is_directory_empty(path: &str) -> bool {
    if let Ok(entries) = std::fs::read_dir(path) {
        let mut count = 0;

        for entry in entries {
            if let Ok(entry) = entry {
                count += 1;
                if let Some(entry_name) = entry.file_name().to_str() {
                    if entry_name.ends_with("mdown_final_end.lock") {
                        return true;
                    }
                }
            }
        }
        count <= 1
    } else {
        false
    }
}

pub(crate) struct FileName {
    pub(crate) manga_name: String,
    pub(crate) vol: String,
    pub(crate) chapter_num: String,
    pub(crate) title: String,
    pub(crate) folder: String,
}

impl FileName {
    pub(crate) fn get_folder_name(&self) -> String {
        if self.title != "" {
            return process_filename(
                &format!(
                    "{} - {}Ch.{} - {}",
                    self.manga_name,
                    self.vol,
                    self.chapter_num,
                    self.title
                )
            );
        } else {
            return format!("{} - {}Ch.{}", self.manga_name, self.vol, self.chapter_num);
        }
    }
    pub(crate) fn get_file_w_folder(&self) -> String {
        format!("{}/{}.cbz", self.folder, format!("{}", process_filename(&self.get_folder_name())))
    }
    pub(crate) fn get_file_w_folder_w_cwd(&self) -> String {
        format!(
            "{}{}/{}.cbz",
            ARGS.cwd,
            self.folder,
            format!("{}", process_filename(&self.get_folder_name()))
        )
    }
    pub(crate) fn get_folder_w_end(&self) -> String {
        format!(".cache/{}/", self.get_folder_name())
    }
    pub(crate) fn get_folder(&self) -> String {
        format!(".cache/{}", self.get_folder_name())
    }
    pub(crate) fn get_lock(&self) -> String {
        format!(".cache\\{}.lock", self.get_folder_name())
    }
}

pub(crate) fn skip_didnt_match(
    attr: &str,
    item: usize,
    moves: u32,
    mut hist: Vec<String>
) -> (u32, Vec<String>) {
    let message = format!("({}) Skipping because supplied {} doesn't match", item as u32, attr);
    if ARGS.web || ARGS.gui || ARGS.check || ARGS.update || ARGS.log {
        log!(&message);
    }
    hist.push(message);
    resolve_move(moves, hist.clone(), 3, 0)
}

pub(crate) fn skip_custom(
    attr: &str,
    item: usize,
    moves: u32,
    mut hist: Vec<String>
) -> (u32, Vec<String>) {
    let message = format!("({}) Skipping because {}", item as u32, attr);
    if ARGS.web || ARGS.gui || ARGS.check || ARGS.update || ARGS.log {
        log!(&message);
    }
    hist.push(message);
    resolve_move(moves, hist.clone(), 3, 0)
}

pub(crate) fn skip(
    attr: String,
    item: usize,
    moves: u32,
    mut hist: Vec<String>
) -> (u32, Vec<String>) {
    let al_dow = format!("({}) Skipping because file is already downloaded {}", item, attr);
    if ARGS.web || ARGS.gui || ARGS.check || ARGS.update || ARGS.log {
        log!(&al_dow);
    }
    hist.push(al_dow);
    resolve_move(moves, hist.clone(), 3, 0)
}
pub(crate) fn skip_offset(item: usize, moves: u32, mut hist: Vec<String>) -> (u32, Vec<String>) {
    let al_dow = format!("({}) Skipping because of offset", item);
    if ARGS.web || ARGS.gui || ARGS.check || ARGS.update || ARGS.log {
        log!(&al_dow);
    }
    hist.push(al_dow);
    resolve_move(moves, hist.clone(), 3, 0)
}

pub(crate) fn debug_print<T: std::fmt::Debug>(item: T, file: &str) -> Result<(), Error> {
    let mut file_inst = match
        std::fs::OpenOptions::new().read(true).write(true).create(true).open(file)
    {
        Ok(file) => file,
        Err(err) => {
            return Err(Error::IoError(err, Some(String::from(file))));
        }
    };
    match write!(file_inst, "{:?}", item) {
        Ok(()) => (),
        Err(err) => {
            (
                match resolute::SUSPENDED.lock() {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(Error::PoisonError(err.to_string()));
                    }
                }
            ).push(Error::IoError(err, Some(String::from(file))));
        }
    }
    Ok(())
}

pub(crate) fn generate_random_id(length: usize) -> Box<str> {
    let rng = rand::thread_rng();
    let id: String = rng.sample_iter(&Alphanumeric).take(length).map(char::from).collect();
    id.into_boxed_str()
}

// Returns a regex match when given a string containing a valid Mangadex URL.
#[test]
fn test_resolve_regex_valid_mangadex_url() {
    let url = "https://mangadex.org/title/12345";
    let result = resolve_regex(url);
    assert!(result.is_some());
}

// Returns None when given a string that does not contain a valid Mangadex URL.
#[test]
fn test_resolve_regex_invalid_mangadex_url() {
    let url = "https://example.com";
    let result = resolve_regex(url);
    assert!(result.is_none());
}

// Returns None when given an empty string.
#[test]
fn test_resolve_regex_empty_string() {
    let url = "";
    let result = resolve_regex(url);
    assert!(result.is_none());
}

// Returns None when given a string that contains a URL that is not a Mangadex URL.
#[test]
fn test_resolve_regex_non_mangadex_url() {
    let url = "https://google.com";
    let result = resolve_regex(url);
    assert!(result.is_none());
}

// Returns None when given a string that contains a Mangadex URL with an invalid format.
#[test]
fn test_resolve_regex_invalid_mangadex_url_format() {
    let url = "https://mangadex.org/title/12345/extra";
    let result = resolve_regex(url);
    assert!(result.is_some());
}

// Returns a regex match when given a string containing a Mangadex URL with a valid format, but with additional query parameters.
#[test]
fn test_resolve_regex_mangadex_url_with_query_parameters() {
    let url = "https://mangadex.org/title/12345?param=value";
    let result = resolve_regex(url);
    assert!(result.is_some());
}

// Given a valid filename with no special characters, it should return the same filename
#[test]
fn should_return_same_filename() {
    let filename = "test.txt";
    let result = process_filename(filename);
    assert_eq!(result, "test.txt");
}

// Given a filename with only special characters, it should return an empty string
#[test]
fn should_return_empty_string() {
    let filename = "<>:|?*/\\\"";
    let result = process_filename(filename);
    assert_eq!(result, "");
}

// Sorts a vector of JSON objects by the 'chapter' attribute in ascending order
#[test]
fn sort_sorts_vector_by_chapter_attribute() {
    let data = vec![
        serde_json::json!({"attributes": {"chapter": "3"}}),
        serde_json::json!({"attributes": {"chapter": "1"}}),
        serde_json::json!({"attributes": {"chapter": "2"}})
    ];
    let expected = vec![
        serde_json::json!({"attributes": {"chapter": "1"}}),
        serde_json::json!({"attributes": {"chapter": "2"}}),
        serde_json::json!({"attributes": {"chapter": "3"}})
    ];

    let result = sort(&data);

    assert_eq!(result, expected);
}

// Handles input vector with non-JSON objects
#[test]
fn sort_handles_input_with_non_json_objects() {
    let data = vec![
        serde_json::json!({"attributes": {"chapter": "3"}}),
        serde_json::json!({"attributes": {"chapter": "1"}}),
        serde_json::json!({"attributes": {"chapter": "2"}}),
        "not a JSON object".into()
    ];
    let expected = vec![
        serde_json::json!({"attributes": {"chapter": "1"}}),
        serde_json::json!({"attributes": {"chapter": "2"}}),
        serde_json::json!({"attributes": {"chapter": "3"}}),
        "not a JSON object".into()
    ];

    let result = sort(&data);

    assert_eq!(result, expected);
}
