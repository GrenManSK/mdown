use chrono::prelude::*;
use crosscurses::*;
use rand::{ distributions::Alphanumeric, Rng };
use remove_dir_all::remove_dir_all;
use serde_json::{ json, Value };
use sha2::{ Sha256, Digest };
use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fs::{ self, File, OpenOptions },
    io::{ Read, Write },
    process::exit,
    thread::sleep,
    time::{ Duration, Instant },
};
use uuid::Uuid;

use crate::{
    args,
    debug,
    download,
    error::{ MdownError, suspend_error },
    getter,
    IS_END,
    log,
    MAXPOINTS,
    metadata,
    resolute::{ self, resolve_move, CURRENT_PERCENT, CURRENT_SIZE, CURRENT_SIZE_MAX },
    string,
    version_manager::get_current_version,
};

pub(crate) fn setup_requirements(file_path: String) {
    debug!("start crosscurses");
    let _ = initscr();
    *resolute::INITSCR_INIT.lock() = true;
    curs_set(2);
    start_color();
    crosscurses::noecho();
    crosscurses::cbreak();
    let file_path_temp = file_path.clone();
    tokio::spawn(async move { print_version(&file_path).await });
    debug!("print_version instance started");
    tokio::spawn(async move { ctrl_handler(&file_path_temp).await });
    debug!("ctrl_handler instance started");
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
        let _ = fs::remove_file(&lock_path);
    }

    let mut current_id = String::new();

    loop {
        sleep(Duration::from_millis(100));

        if fs::metadata(&path).is_err() {
            let mut file = match fs::File::create(&path) {
                Ok(file) => file,
                Err(_err) => {
                    continue;
                }
            };

            let content = String::from("logs: {}");

            match file.write_all(content.as_bytes()) {
                Ok(()) => (),
                Err(_err) => (),
            };
        }
        if *resolute::ENDED.lock() {
            let _ = fs::remove_file(&lock_path);
            return;
        }

        while fs::metadata(&lock_path).is_ok() {
            sleep(Duration::from_millis(10));
        }
        let _ = File::create(&lock_path);
        let json = match resolute::get_dat_content(path.as_str()) {
            Ok(value) => value,
            Err(_err) => json!({}),
        };

        let mut data = match serde_json::from_value::<metadata::LogMetadata>(json) {
            Ok(obj) => obj,
            Err(_err) => {
                eprintln!("Couldn't parse log metadata");
                continue;
            }
        };

        let mut messages_lock = resolute::LOGS.lock();
        let mut handle_id_lock = resolute::HANDLE_ID_END.lock();

        let char = ["\\n", "\\r", "\\t", "\\\\", "\\'", "\\\"", "\\0"];

        let messages: Vec<metadata::Log> = messages_lock
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

        for message in messages.iter() {
            let handle_id = message.handle_id.to_string();
            let chap_num = message.name.to_string();
            if handle_id == String::new() {
                continue;
            }
            let mut inst: Vec<String> = Vec::new();
            let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
            if let Some(value) = data.logs.get_mut(&handle_id.to_string()) {
                if let Some(logs_inst) = value.logs.get_mut(&chap_num) {
                    inst.extend_from_slice(logs_inst);
                }
            }
            if let Some(value) = data.logs.get_mut(&handle_id.to_string()) {
                map = value.logs.clone();
            }
            let start_time = {
                if let Some(time) = data.logs.get(&handle_id.to_string()) {
                    time.time_start.clone()
                } else {
                    Utc::now().to_rfc3339()
                }
            };
            inst.push(format!("{}  {}", &message.time, &message.message));

            map.insert(chap_num.clone(), inst.clone());

            match handle_id.len() {
                10 => {
                    data.logs.insert(
                        handle_id.to_string(),
                        metadata::LogsMetadata::new("", map, "", "", "", &start_time, "web")
                    );
                }
                16 => {
                    let manga_name = resolute::MANGA_NAME.lock().clone();
                    // If downloading ended will not discard id
                    let manga_id = if *resolute::DOWNLOADING.lock() {
                        resolute::MANGA_ID.lock().clone()
                    } else {
                        current_id.clone()
                    };
                    let mwd = resolute::MWD.lock().clone();
                    data.logs.insert(
                        handle_id.to_string(),
                        metadata::LogsMetadata::new(
                            &manga_id,
                            map,
                            &mwd,
                            &manga_name,
                            "",
                            &start_time,
                            "downloader"
                        )
                    );
                    current_id = manga_id;
                }
                _ => {
                    data.logs.insert(
                        handle_id.to_string(),
                        metadata::LogsMetadata::new("", map, "", "", "", &start_time, "unknown")
                    );
                }
            }
        }
        for handle_id in handle_id_lock.iter() {
            if handle_id == &String::new().into_boxed_str() {
                continue;
            }
            let end_time = Utc::now().to_rfc3339();
            if let Some(handle) = data.logs.get_mut(&handle_id.to_string()) {
                handle.time_end = end_time;
            }
        }

        let mut file = match File::create(&path) {
            Ok(file) => file,
            Err(_err) => {
                continue;
            }
        };

        let json_string = match serde_json::to_string_pretty::<metadata::LogMetadata>(&data) {
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

        let _ = fs::remove_file(&lock_path);
    }
}

pub(crate) fn reset() -> Result<(), MdownError> {
    let confirmation = match input("Do you want to factory reset this app? (y/N) > ") {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };

    if confirmation.to_lowercase() != *"y" {
        return Ok(());
    }

    debug!("confirmation succeeded");
    let dat = match getter::get_dat_path() {
        Ok(dat) => dat,
        Err(err) => {
            return Err(err);
        }
    };
    let db = match getter::get_db_path() {
        Ok(dat) => dat,
        Err(err) => {
            return Err(err);
        }
    };
    let log = match getter::get_log_path() {
        Ok(dat) => dat,
        Err(err) => {
            return Err(err);
        }
    };

    match std::fs::remove_file(&dat) {
        Ok(_) => println!("dat.json was successfully removed"),
        Err(err) => {
            match err.raw_os_error() {
                Some(code) => {
                    if code != 2 {
                        push_suspended(err, "dat.json");
                    }
                }
                None => push_suspended(err, "dat.json"),
            }
        }
    }
    match std::fs::remove_file(&db) {
        Ok(_) => println!("resources.db was successfully removed"),
        Err(err) => {
            match err.raw_os_error() {
                Some(code) => {
                    if code != 2 {
                        push_suspended(err, "resources.db");
                    }
                }
                None => push_suspended(err, "resources.db"),
            }
        }
    }
    match std::fs::remove_file(&log) {
        Ok(_) => println!("log.json was successfully removed"),
        Err(err) => {
            match err.raw_os_error() {
                Some(code) => {
                    if code != 2 {
                        push_suspended(err, "log.json");
                    }
                }
                None => push_suspended(err, "log.json"),
            }
        }
    }

    debug!("operations were successfully executed");

    Ok(())
}

fn push_suspended(err: std::io::Error, name: &str) {
    suspend_error(MdownError::IoError(err, name.to_string(), 10400));
}

pub(crate) fn remove_cache() -> Result<(), MdownError> {
    if is_directory_empty(".cache\\") {
        match remove_dir_all(".cache") {
            Ok(()) => (),
            Err(err) => {
                suspend_error(MdownError::IoError(err, String::from(".cache\\"), 10401));
            }
        };
    }
    Ok(())
}

pub(crate) fn input(text: &str) -> Result<String, MdownError> {
    print!("{}", text);
    match std::io::stdout().flush() {
        Ok(()) => (),
        Err(err) => {
            return Err(MdownError::IoError(err, String::new(), 10402));
        }
    }

    let mut input = String::new();
    match std::io::stdin().read_line(&mut input) {
        Ok(_) => (),
        Err(err) => {
            return Err(MdownError::IoError(err, String::new(), 10403));
        }
    }
    Ok(input.trim().to_string())
}

pub(crate) fn setup_subscriber() -> Result<(), MdownError> {
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
            suspend_error(
                MdownError::CustomError(
                    String::from("Failed to set up tracing_subscriber (basically info)"),
                    String::from("Subscriber"),
                    10404
                )
            );
            Ok(())
        }
    }
}

pub(crate) fn create_cache_folder() -> Result<(), MdownError> {
    match fs::create_dir(".cache") {
        Ok(()) => Ok(()),
        Err(err) => {
            suspend_error(MdownError::IoError(err, String::from(".cache\\"), 10405));
            Ok(())
        }
    }
}

pub(crate) fn is_valid_uuid(s: &str) -> bool {
    Uuid::parse_str(s).is_ok()
}

pub(crate) fn clear_screen(from: u32) {
    // if crosscurses is not used DO NOT let this run
    if
        !*args::ARGS_WEB &&
        !*args::ARGS_GUI &&
        !*args::ARGS_CHECK &&
        !*args::ARGS_UPDATE &&
        !*args::ARGS_QUIET
    {
        for i in from..MAXPOINTS.max_y {
            string(i, 0, &" ".repeat(MAXPOINTS.max_x as usize));
        }
    }
}

pub(crate) fn process_filename(filename: &str) -> String {
    filename.replace(['<', '>', ':', '|', '?', '*', '/', '\\', '"'], "")
}

pub(crate) async fn wait_for_end(file_path: &str, images_length: usize) -> Result<(), MdownError> {
    let full_path = format!(".cache\\{}.lock", file_path);
    let mut full_size = 0.0;
    let start = Instant::now();
    while fs::metadata(&full_path).is_ok() {
        let mut size = 0.0;
        for i in 1..images_length + 1 {
            let image_name = format!(".cache\\{}_{}.lock", file_path, i);
            if fs::metadata(&image_name).is_ok() {
                let mut image_file = match File::open(&image_name) {
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
                if !image_content.is_empty() {
                    let image_content: f64 = match image_content.parse() {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(MdownError::ConversionError(err.to_string(), 10406));
                        }
                    };
                    size += image_content / 1024.0 / 1024.0;
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
                if !image_content.is_empty() {
                    let image_content: f64 = match image_content.parse() {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(MdownError::ConversionError(err.to_string(), 10407));
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
        let percent = if full_size == 0.0 { 0.0 } else { (100.0 / full_size) * size };
        *CURRENT_PERCENT.lock() = percent;
        *CURRENT_SIZE.lock() = size;
        *CURRENT_SIZE_MAX.lock() = full_size;
        string(
            4,
            MAXPOINTS.max_x - 60,
            &format!(
                " {:.2}% {:.2}mb/{:.2}mb [{:.2}mb remaining] [{:.2}s]",
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
    if
        !*args::ARGS_WEB &&
        !*args::ARGS_GUI &&
        !*args::ARGS_CHECK &&
        !*args::ARGS_UPDATE &&
        !*args::ARGS_SERVER
    {
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
pub(crate) fn sort(data: &Vec<metadata::ChapterResponse>) -> Vec<metadata::ChapterResponse> {
    let mut data_array = data.to_owned();

    if *args::ARGS_UNSORTED {
        return data.to_vec();
    }

    data_array.sort_unstable_by(|v, b| {
        match
            (
                match v.attributes.chapter.clone() {
                    Some(v_chapter) => v_chapter,
                    None => String::from("0"),
                }
            )
                .parse::<f32>()
                .ok()
                .map(|v_parsed| {
                    match
                        (
                            match b.attributes.chapter.clone() {
                                Some(b_chapter) => b_chapter,
                                None => String::from("0"),
                            }
                        )
                            .parse::<f32>()
                            .ok()
                            .map(|b_parsed| v_parsed.total_cmp(&b_parsed))
                    {
                        Some(value) => value,
                        None => Ordering::Equal,
                    }
                })
        {
            Some(value) => value,
            None => Ordering::Equal,
        }
    });

    data_array
}

pub(crate) fn get_json(manga_name_json: &str) -> Result<Value, MdownError> {
    match serde_json::from_str(manga_name_json) {
        Ok(value) => Ok(value),
        Err(err) => Err(MdownError::JsonError(err.to_string(), 10408)),
    }
}

pub(crate) async fn search() -> Result<String, MdownError> {
    let base_url = "https://api.mangadex.org";
    let title = &args::ARGS.lock().search.clone();

    let client = match download::get_client() {
        Ok(client) => client,
        Err(err) => {
            return Err(MdownError::NetworkError(err, 10409));
        }
    };

    let full_url = format!("{}/manga", base_url);

    debug!("sending request to: {}", full_url);

    let response = match
        client
            .get(&full_url)
            .query(&[("title", title)])
            .send().await
    {
        Ok(response) => response,
        Err(err) => {
            return Err(MdownError::NetworkError(err, 10410));
        }
    };

    if response.status().is_success() {
        let manga_data: serde_json::Value = match response.json().await {
            Ok(value) => value,
            Err(err) => {
                return Err(MdownError::JsonError(err.to_string(), 10411));
            }
        };

        let data = match manga_data.get("data") {
            Some(data) => data,
            None => {
                return Err(
                    MdownError::NotFoundError(String::from("data in manga_data in main.rs"), 10412)
                );
            }
        };
        let manga_array = match data.as_array() {
            Some(data) => data,
            None => {
                return Err(
                    MdownError::ConversionError(
                        String::from("manga_data to array in main.rs"),
                        10413
                    )
                );
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

        debug!("manga_ids: {:?}", manga_ids);

        return match manga_ids.first() {
            Some(id) => Ok(id.to_string()),
            None =>
                Err(
                    MdownError::NotFoundError(
                        String::from("manga_id in manga_ids in main.rs"),
                        10414
                    )
                ),
        };
    } else {
        Err(MdownError::StatusError(response.status(), 10415))
    }
}

pub(crate) fn resolve_start() -> Result<String, MdownError> {
    let file_path: String = format!(".cache\\mdown_{}.lock", get_current_version());
    if *args::ARGS_FORCE_DELETE {
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
    if fs::metadata(&file_path).is_ok() {
        eprintln!(
            "Lock file has been found;\nSee README.md;\nCannot run multiple instances of mdown"
        );
        exit(10499);
    }
    match File::create(&file_path) {
        Ok(_) => (),
        Err(e) => {
            panic!("Error creating the file: {}", e);
        }
    }

    Ok(file_path)
}

pub(crate) async fn ctrl_handler(file: &str) {
    if fs::metadata(".cache\\mdown_final_end.lock").is_ok() {
        match fs::remove_file(".cache\\mdown_final_end.lock") {
            Ok(()) => (),
            Err(err) => eprintln!("Error: removing file mdown_final_end.lock {}", err),
        };
    }
    loop {
        if fs::metadata(file).is_err() {
            break;
        }
        let key: Input = match stdscr().getch() {
            Some(ch) => ch,
            None => Input::Character('a'),
        };
        if key == crosscurses::Input::Character('\u{3}') {
            debug!("ctrl-c was received");
            *IS_END.lock() = true;
            if *args::ARGS_LOG {
                log!("CTRL+C received");
                log!("CTRL+C received", "", false);
            }
            break;
        }
    }
    if resolve_final_end() || *resolute::ENDED.lock() {
        exit(0);
    }
    clear_screen(0);
    string(0, 0, "CTRL_C: Cleaning up");
    sleep(Duration::from_secs(1));
    match fs::remove_file(file) {
        Ok(()) => (),
        Err(_err) => (),
    }

    delete_dir_if_unfinished(getter::get_folder_name());
    delete_dir();

    if is_directory_empty(".cache\\") {
        match remove_dir_all(".cache") {
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
            match remove_dir_all(".cache") {
                Ok(()) => (),
                Err(err) => eprintln!("Error: removing .cache, {}", err),
            };
        }
        return true;
    }
    false
}

pub(crate) fn delete_dir() {
    if let Ok(entries) = fs::read_dir(".cache") {
        for entry in entries.flatten() {
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

pub(crate) fn delete_dir_if_unfinished(path: &str) {
    match fs::read_dir(path) {
        Ok(entries) => {
            let mut should_delete = 0;

            for entry in entries.flatten() {
                let file_path = entry.path();
                let file_name = match file_path.file_name() {
                    Some(value) => value.to_str().unwrap_or_default(),
                    None => "",
                };

                if
                    !file_name.ends_with("_cover.png") &&
                    !file_name.ends_with("_description.txt") &&
                    !file_name.ends_with("_scanlation_groups.txt") &&
                    !file_name.ends_with("_statistics.md")
                {
                    debug!("file is not service file");
                    should_delete += 1;
                }
            }

            if should_delete == 0 {
                debug!("deleting manga folder because it didn't download anything");
                match remove_dir_all(path) {
                    Ok(()) => (),
                    Err(err) => eprintln!("Error: removing directory '{}' {}", path, err),
                };
            }
        }
        Err(e) => eprintln!("Error reading directory: {}", e),
    }
}

pub(crate) async fn print_version(file: &str) {
    if
        !*args::ARGS_WEB &&
        !*args::ARGS_GUI &&
        !*args::ARGS_CHECK &&
        !*args::ARGS_UPDATE &&
        !*args::ARGS_SERVER
    {
        return;
    }
    let version = get_current_version();
    for _ in 0..50 {
        string(MAXPOINTS.max_y - 1, 0, &format!("Current version: {}", version));
        if fs::metadata(file).is_err() {
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
            suspend_error(MdownError::RegexError(err, 10416));
            return None;
        }
    };
    re.captures(cap).and_then(|id| id.get(1))
}

pub(crate) fn resolve_end(
    file_path: &str,
    manga_name: &str,
    status_code: reqwest::StatusCode,
    err_code: u32
) -> Result<(), String> {
    match fs::remove_file(file_path) {
        Ok(()) => (),
        Err(err) => eprintln!("Error: removing file '{}' {}", file_path, err),
    }
    match
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(".cache\\mdown_final_end.lock")
    {
        Ok(_file) => (),
        Err(err) => {
            return Err(format!("Error: failed to open file {}", err));
        }
    }

    sleep(Duration::from_millis(110));
    let message = if status_code.is_client_error() {
        string(0, 0, "Id was not found, please recheck the id and try again");
        format!(
            "Ending session: {} has NOT been downloaded, because: {:?} ({})",
            manga_name,
            status_code.canonical_reason().unwrap_or("Didn't find error :/"),
            err_code
        )
    } else if status_code.is_server_error() {
        string(
            0,
            0,
            &format!(
                "Server error: {}: {:?} ({})",
                status_code,
                status_code.canonical_reason().unwrap_or("Didn't find error :/"),
                err_code
            )
        );
        format!("Ending session: {} has NOT been downloaded", manga_name)
    } else if manga_name.eq("!") {
        string(
            0,
            0,
            "Either --url or --search was not specified or website is not in pattern of UUID | https://mangadex.org/title/[UUID]/ or UUID is not valid"
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

fn calculate_sha256(file_path: &str) -> Result<String, MdownError> {
    let mut file = match File::open(file_path) {
        Ok(file) => file,
        Err(err) => {
            return Err(MdownError::IoError(err, file_path.to_string(), 10417));
        }
    };
    let mut hasher = Sha256::new();
    let mut buffer = [0; 4096];

    loop {
        let n = match file.read(&mut buffer) {
            Ok(n) => n,
            Err(err) => {
                return Err(MdownError::IoError(err, file_path.to_string(), 10418));
            }
        };
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let hash_result = hasher.finalize();
    Ok(format!("{:x}", hash_result))
}

fn get_backup_dat(backup_dir: &str) -> Result<(Vec<NaiveDate>, Vec<String>), MdownError> {
    let mut dats = Vec::new();
    let mut dats_filename = Vec::new();

    match fs::read_dir(backup_dir) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();

                    if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                        if file_name.starts_with("dat_") && file_name.ends_with(".json") {
                            let stripped_name = &file_name[4..file_name.len() - 5];
                            let date = match NaiveDate::parse_from_str(stripped_name, "%Y_%m_%d") {
                                Ok(date) => date,
                                Err(_err) => {
                                    continue;
                                }
                            };
                            dats.push(date);
                            dats_filename.push(file_name.to_string());
                        }
                    }
                }
            }
        }
        Err(err) => {
            return Err(MdownError::IoError(err, backup_dir.to_string(), 10419));
        }
    }

    dats.sort();
    dats_filename.sort();
    Ok((dats, dats_filename))
}

pub(crate) fn backup_choose() -> Result<(), MdownError> {
    let backup_dir = match getter::get_bac_path() {
        Ok(exe_path) => exe_path,
        Err(err) => {
            return Err(err);
        }
    };
    let dat_file = match getter::get_dat_path() {
        Ok(exe_path) => exe_path,
        Err(err) => {
            return Err(err);
        }
    };
    let exe_dir = match getter::get_exe_path() {
        Ok(exe_path) => exe_path,
        Err(err) => {
            return Err(err);
        }
    };

    let (dats, dats_filename) = match get_backup_dat(&backup_dir) {
        Ok((dats, dats_filename)) => (dats, dats_filename),
        Err(err) => {
            return Err(err);
        }
    };

    for (i, (value, filename)) in dats.iter().zip(dats_filename.iter()).enumerate() {
        let file_size = match std::fs::metadata(format!("{}\\{}", backup_dir, filename)) {
            Ok(metadata) => metadata.len(),
            Err(err) => {
                return Err(MdownError::IoError(err, filename.to_string(), 10420));
            }
        };
        let file_size_string = bytefmt::format(file_size);
        println!("{}) {} ({})", i + 1, value, file_size_string);
    }

    let vstup = match input("> ") {
        Ok(input) => {
            match input.trim().parse::<usize>() {
                Ok(index) => index,
                Err(_err) => 1,
            }
        }
        Err(err) => {
            return Err(err);
        }
    };
    if vstup > 0 && vstup <= dats.len() {
        let filename = &dats_filename[vstup - 1];
        let file_path = format!("{}\\{}", backup_dir, filename);
        let backup_file_path = format!("{}\\{}", exe_dir, "dat.json.tmp");

        let vstup = match
            input(&format!("Are you sure you want to choose \"{}\"? (y/N) > ", filename))
        {
            Ok(input) => input.to_ascii_lowercase(),
            Err(err) => {
                return Err(err);
            }
        };
        if vstup == "y" {
            match fs::copy(&dat_file, &backup_file_path) {
                Ok(_) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, dat_file, 10421));
                }
            }
            match fs::copy(&file_path, &dat_file) {
                Ok(_) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, dat_file, 10422));
                }
            }
            match fs::remove_file(&backup_file_path) {
                Ok(_) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, dat_file, 10423));
                }
            }
            println!("Backup successful");
        } else {
            println!("Backup canceled due user wishes");
        }
    }
    Ok(())
}

pub(crate) fn backup_handler(force: bool) -> Result<(), MdownError> {
    debug!("backup_handler");
    let backup_dir = match getter::get_bac_path() {
        Ok(exe_path) => exe_path,
        Err(err) => {
            return Err(err);
        }
    };
    match fs::create_dir_all(&backup_dir) {
        Ok(()) => (),
        Err(err) => {
            return Err(MdownError::IoError(err, backup_dir, 10424));
        }
    }

    let (dats, dats_filename) = match get_backup_dat(&backup_dir) {
        Ok((dats, dats_filename)) => (dats, dats_filename),
        Err(err) => {
            return Err(err);
        }
    };

    if dats.is_empty() {
        return Ok(());
    }

    let latest_date = match dats.last() {
        Some(date) => date,
        None => &chrono::Local::now().naive_local().date(),
    };

    let current_date = Local::now().naive_local().date();

    let day_before = match current_date.pred_opt() {
        Some(date) => date,
        None => *latest_date,
    };

    if latest_date < &day_before || dats.is_empty() || force {
        debug!("creating backup");
        let source_file = match getter::get_dat_path() {
            Ok(source) => source,
            Err(err) => {
                return Err(err);
            }
        };

        if !dats_filename.is_empty() {
            let current_file = match dats_filename.last() {
                Some(date) => date,
                None => "",
            };
            let source_sha = match calculate_sha256(&source_file) {
                Ok(hash) => hash,
                Err(err) => {
                    return Err(err);
                }
            };

            let latest_sha = match calculate_sha256(&format!("{}\\{}", backup_dir, current_file)) {
                Ok(hash) => hash,
                Err(err) => {
                    return Err(err);
                }
            };
            if source_sha == latest_sha && !force {
                debug!("No changes detected, not backing up");
                return Ok(());
            }
        }

        let date_name = chrono::Local::now().naive_local().date().format("%Y_%m_%d").to_string();

        let destination_file = format!("{}\\dat_{}.json", backup_dir, date_name);

        match fs::copy(&source_file, &destination_file) {
            Ok(_) => {
                debug!("Copied successfully");
            }
            Err(err) => {
                return Err(MdownError::IoError(err, source_file, 10425));
            }
        }
    }

    Ok(())
}

pub(crate) fn show_settings(settings: metadata::Settings) {
    println!("folder: {}", settings.folder);
    println!("stat: {}", settings.stat);
    println!("backup: {}", settings.backup);
}

pub(crate) fn is_directory_empty(path: &str) -> bool {
    if let Ok(entries) = std::fs::read_dir(path) {
        let mut count = 0;

        for entry in entries.flatten() {
            count += 1;
            if let Some(entry_name) = entry.file_name().to_str() {
                if entry_name.ends_with("mdown_final_end.lock") {
                    return true;
                }
            }
        }
        count <= 1
    } else {
        false
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FileName {
    pub(crate) manga_name: String,
    pub(crate) vol: String,
    pub(crate) chapter_num: String,
    pub(crate) title: String,
    pub(crate) folder: String,
}

impl FileName {
    pub(crate) fn get_folder_name(&self) -> String {
        if !self.title.is_empty() {
            process_filename(
                &format!(
                    "{} - {}Ch.{} - {}",
                    self.manga_name,
                    self.vol,
                    self.chapter_num,
                    self.title
                )
            )
        } else {
            process_filename(&format!("{} - {}Ch.{}", self.manga_name, self.vol, self.chapter_num))
        }
    }
    pub(crate) fn get_file_w_folder(&self) -> String {
        if *args::ARGS_UPDATE {
            format!("{}.cbz", process_filename(&self.get_folder_name()))
        } else {
            format!("{}\\{}.cbz", self.folder, process_filename(&self.get_folder_name()))
        }
    }
    pub(crate) fn get_file_w_folder_w_cwd(&self) -> String {
        format!(
            "{}{}\\{}.cbz",
            *args::ARGS_CWD,
            self.folder,
            process_filename(&self.get_folder_name())
        )
    }
    pub(crate) fn get_folder_w_end(&self) -> String {
        format!(".cache\\{}\\", self.get_folder_name())
    }
    pub(crate) fn get_folder(&self) -> String {
        format!(".cache\\{}", self.get_folder_name())
    }
    pub(crate) fn get_lock(&self) -> String {
        format!(".cache\\{}.lock", self.get_folder_name())
    }
}

pub(crate) fn skip_didnt_match<'a>(
    attr: &'a str,
    item: usize,
    moves: u32,
    hist: &'a mut Vec<String>
) -> u32 {
    let message = format!("({}) Skipping because supplied {} doesn't match", item as u32, attr);
    if
        *args::ARGS_WEB ||
        *args::ARGS_GUI ||
        *args::ARGS_CHECK ||
        *args::ARGS_UPDATE ||
        *args::ARGS_LOG
    {
        log!(&message);
    }
    hist.push(message);
    resolve_move(moves, hist, 3, 0)
}

pub(crate) fn skip_custom<'a>(
    attr: &'a str,
    item: usize,
    moves: u32,
    hist: &'a mut Vec<String>
) -> u32 {
    let message = format!("({}) Skipping because {}", item as u32, attr);
    if
        *args::ARGS_WEB ||
        *args::ARGS_GUI ||
        *args::ARGS_CHECK ||
        *args::ARGS_UPDATE ||
        *args::ARGS_LOG
    {
        log!(&message);
    }
    hist.push(message);
    resolve_move(moves, hist, 3, 0)
}

pub(crate) fn skip(
    attr: String,
    item: usize,
    moves: u32,
    hist: &mut Vec<String>,
    start: u32
) -> u32 {
    let al_dow = format!("({}) Skipping because file is already downloaded {}", item, attr);
    if
        *args::ARGS_WEB ||
        *args::ARGS_GUI ||
        *args::ARGS_CHECK ||
        *args::ARGS_UPDATE ||
        *args::ARGS_LOG
    {
        log!(&al_dow);
    }
    hist.push(al_dow);
    resolve_move(moves, hist, start, 0)
}
pub(crate) fn skip_offset(item: usize, moves: u32, hist: &mut Vec<String>) -> u32 {
    let al_dow = format!("({}) Skipping because of offset", item);
    if
        *args::ARGS_WEB ||
        *args::ARGS_GUI ||
        *args::ARGS_CHECK ||
        *args::ARGS_UPDATE ||
        *args::ARGS_LOG
    {
        log!(&al_dow);
    }
    hist.push(al_dow);
    resolve_move(moves, hist, 3, 0)
}

#[allow(dead_code)]
pub(crate) fn debug_print<T: std::fmt::Debug>(item: T, file: &str) -> Result<(), MdownError> {
    let mut file_inst = match
        std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(file)
    {
        Ok(file) => file,
        Err(err) => {
            return Err(MdownError::IoError(err, String::from(file), 10426));
        }
    };
    match write!(file_inst, "{:?}", item) {
        Ok(()) => (),
        Err(err) => {
            suspend_error(MdownError::IoError(err, String::from(file), 10427));
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
