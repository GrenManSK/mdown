use crosscurses::*;
use std::{
    fs::{ self, OpenOptions },
    time::{ Instant, Duration },
    fs::File,
    io::Read,
    process::exit,
    thread::sleep,
    path::Path,
};
use serde_json::Value;
use regex::Regex;

use crate::{ string, ARGS, resolute::resolve_move, MAXPOINTS, IS_END };

pub(crate) fn clear_screen(from: i32) {
    for i in from..MAXPOINTS.max_y {
        string(i, 0, &" ".repeat(MAXPOINTS.max_x as usize));
    }
}

pub(crate) fn process_filename(filename: String) -> String {
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

pub(crate) async fn wait_for_end(file_path: String, images_length: usize) {
    let full_path = format!("{}.lock", file_path);
    let mut full_size = 0.0;
    let start = Instant::now();
    while fs::metadata(full_path.clone()).is_ok() {
        let mut size = 0.0;
        for i in 1..images_length + 1 {
            let image_name = format!("{}_{}.lock", file_path, i);
            if fs::metadata(image_name.clone()).is_ok() {
                let mut image_file = unsafe { File::open(image_name.clone()).unwrap_unchecked() };
                let mut image_content = String::new();
                let _ = image_file.read_to_string(&mut image_content);
                if image_content != "" {
                    let image_content: f64 = image_content.parse().unwrap();
                    size += image_content;
                }
            }
        }
        for i in 1..images_length + 1 {
            let image_name = format!("{}_{}_final.lock", file_path, i);
            if fs::metadata(image_name.clone()).is_ok() {
                let mut image_file = unsafe { File::open(image_name.clone()).unwrap_unchecked() };
                let mut image_content = String::new();
                let _ = image_file.read_to_string(&mut image_content);
                if image_content != "" {
                    let image_content: f64 = image_content.parse().unwrap();
                    full_size += image_content / 1024.0 / 1024.0;
                    let _ = fs::remove_file(image_name.clone());
                }
            }
        }
        let percent;
        if full_size == 0.0 {
            percent = 0.0;
        } else {
            percent = (100.0 / full_size) * size;
        }
        string(
            6,
            MAXPOINTS.max_x - 60,
            &format!(
                "{:.2}% {:.2}mb/{:.2}mb [{:.2}mb remaining] [{:.2}s]",
                percent,
                size,
                full_size,
                full_size - size,
                (Instant::now() - start).as_secs_f64()
            )
        );
    }
    let _ = fs::remove_file(full_path.clone());

    for i in 1..images_length + 1 {
        let image_name = format!("{}_{}.lock", file_path, i);
        if fs::metadata(image_name.clone()).is_ok() {
            let _ = fs::remove_file(image_name);
        }
    }
}

pub(crate) fn progress_bar_preparation(start: i32, images_length: usize, line: i32) {
    string(line, 0, &format!("{}|", &"-".repeat((start as usize) - 1)));
    string(
        line,
        start + (images_length as i32),
        &format!(
            "|{}",
            &"-".repeat(
                (MAXPOINTS.max_x as usize) - ((start + (images_length as i32) + 1) as usize)
            )
        )
    );
}

pub(crate) fn sort(data: &Vec<Value>) -> Vec<Value> {
    let mut data_array = data.to_owned();
    data_array.sort_unstable_by(|v, b| {
        return v
            .get("attributes")
            .unwrap()
            .get("chapter")
            .unwrap()
            .as_str()
            .unwrap()
            .parse::<f32>()
            .unwrap()
            .total_cmp(
                &b
                    .get("attributes")
                    .unwrap()
                    .get("chapter")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .parse::<f32>()
                    .unwrap()
            );
    });
    return data_array;
}

pub(crate) fn resolve_start() -> (String, String, String) {
    let file_path: String = format!("mdown_{}.lock", env!("CARGO_PKG_VERSION"));
    if ARGS.force_delete {
        let _ = fs::remove_file(file_path);
        exit(0);
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

    let _ = initscr();
    curs_set(2);
    start_color();
    (file_path.clone(), file_path.clone(), file_path)
}

pub(crate) fn delete_matching_directories(pattern: &Regex) -> Result<String, u32> {
    if Path::new(".").is_dir() {
        if let Ok(entries) = fs::read_dir(".") {
            let mut last_entry_path = String::new();
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if pattern.is_match(path.to_str().unwrap_or("")) && path.is_dir() {
                        let _ = fs::remove_dir_all(&path);
                        last_entry_path = path.to_str().unwrap_or("").to_string();
                    }
                }
            }
            return Ok(last_entry_path);
        }
    }
    Err(1)
}

pub(crate) async fn ctrl_handler(file: String) {
    if fs::metadata("mdown_final_end.lock").is_ok() {
        let _ = fs::remove_file("mdown_final_end.lock");
    }
    loop {
        if fs::metadata(file.clone()).is_err() {
            break;
        }
        let key = stdscr().getch().unwrap();
        if key == Input::from(crosscurses::Input::Character('\u{3}')) {
            let mut is_end = IS_END.lock().unwrap();
            *is_end = true;
            break;
        }
    }
    if fs::metadata("mdown_final_end.lock").is_ok() {
        let _ = fs::remove_file("mdown_final_end.lock");
        return;
    }
    clear_screen(0);
    string(0, 0, "CTRL_C: Cleaning up");
    sleep(Duration::from_secs(1));
    let _ = fs::remove_file(file);

    let pattern = Regex::new(r"Vol\.\d+ Ch\.\d+").expect("Invalid regex pattern");
    match delete_matching_directories(&pattern) {
        Ok(path) => {
            let pattern = r#"\.\\(.*?)( - (Vol\.\d+ )?Ch\.\d+|$)"#;
            let re = Regex::new(pattern).expect("Invalid regex pattern");
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

    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();

                if path.is_file() && path.extension().map_or(false, |ext| ext == "lock") {
                    if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                        if file_name.ends_with("Cargo.lock") {
                            continue;
                        }
                    }

                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    exit(0);
}

pub(crate) fn delete_dir_if_unfinished(path: &str) {
    match fs::read_dir(path) {
        Ok(entries) => {
            let mut should_delete = 0;

            for entry in entries {
                if let Ok(entry) = entry {
                    let file_path = entry.path();
                    let file_name = file_path.file_name().unwrap().to_str().unwrap();

                    if
                        file_name.ends_with("_cover.png") ||
                        file_name.ends_with("_description.txt") ||
                        file_name.ends_with("_scanlation_groups.txt")
                    {
                    } else {
                        should_delete += 1;
                    }
                }
            }

            if should_delete == 0 {
                let _ = fs::remove_dir_all(path);
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
    let re = regex::Regex
        ::new(r"https://mangadex.org/title/([\w-]+)/?")
        .expect("Failed to compile regex pattern");
    re.captures(cap).and_then(move |id| id.get(1))
}

pub(crate) fn resolve_end(file_path: String, manga_name: String, status_code: reqwest::StatusCode) {
    let _ = fs::remove_file(file_path);
    OpenOptions::new().read(true).write(true).create(true).open("mdown_final_end.lock").unwrap();

    sleep(Duration::from_millis(110));
    let message = if status_code.is_client_error() {
        string(0, 0, &format!("Id was not found, please recheck the id and try again"));
        format!(
            "Ending session: {} has NOT been downloaded, because: {:?}",
            manga_name,
            status_code.canonical_reason().unwrap()
        )
    } else if status_code.is_server_error() {
        string(
            0,
            0,
            &format!("Server error: {}: {:?}", status_code, status_code.canonical_reason().unwrap())
        );
        format!("Ending session: {} has NOT been downloaded", manga_name)
    } else if manga_name.eq("!") {
        string(
            0,
            0,
            &format!(
                "Either --url was not specified or website is not in pattern of https://mangadex.org/title/id/"
            )
        );
        format!("Ending session: {} has NOT been downloaded, because it was not found", manga_name)
    } else {
        format!("Ending session: {} has been downloaded", manga_name)
    };

    string(
        MAXPOINTS.max_y - 1,
        0,
        &(message.clone() + &" ".repeat((MAXPOINTS.max_x as usize) - message.len()))
    );
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
                format!(
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
        format!("{}\\{}.cbz", self.folder, format!("{}", process_filename(self.get_folder_name())))
    }
    pub(crate) fn get_folder_w_end(&self) -> String {
        format!("{}\\", self.get_folder_name())
    }
    pub(crate) fn get_lock(&self) -> String {
        format!("{}.lock", self.get_folder_name())
    }
}

pub(crate) fn skip_didnt_match(
    attr: &str,
    item: usize,
    moves: i32,
    mut hist: Vec<String>
) -> (i32, Vec<String>) {
    hist.push(format!("({}) Skipping because supplied {} doesn't match", item as i32, attr));
    resolve_move(moves, hist.clone(), 3, 0)
}

pub(crate) fn skip(
    attr: String,
    item: usize,
    moves: i32,
    mut hist: Vec<String>
) -> (i32, Vec<String>) {
    let al_dow = format!("({}) Skipping because file is already downloaded {}", item, attr);
    hist.push(al_dow);
    resolve_move(moves, hist.clone(), 3, 0)
}
pub(crate) fn skip_offset(item: usize, moves: i32, mut hist: Vec<String>) -> (i32, Vec<String>) {
    let al_dow = format!("({}) Skipping because of offset", item);
    hist.push(al_dow);
    resolve_move(moves, hist.clone(), 3, 0)
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

// Lock file already exists
#[test]
fn test_lock_file_already_exists() {
    let file_path = format!("mdown_{}.lock", env!("CARGO_PKG_VERSION"));
    File::create(&file_path).unwrap();
    let result = std::panic::catch_unwind(|| {
        resolve_start();
    });
    assert!(result.is_err());
    assert!(fs::metadata(file_path.clone()).is_ok());
    fs::remove_file(file_path).unwrap();
}
