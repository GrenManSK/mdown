use crosscurses::*;
use std::{ fs, time::{ Instant, Duration }, fs::File, io::Read, process::exit, thread::sleep };
use serde_json::Value;

use crate::{ string, ARGS, resolute::resolve_move };

pub(crate) fn clear_screen(from: i32) {
    for i in from..stdscr().get_max_y() {
        string(i, 0, &" ".repeat(stdscr().get_max_x() as usize));
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
            stdscr().get_max_x() - 60,
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
                (stdscr().get_max_x() as usize) - ((start + (images_length as i32) + 1) as usize)
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

pub(crate) fn start() -> (String, String) {
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
    (file_path.clone(), file_path)
}

pub(crate) async fn print_version(file: String) {
    let version = env!("CARGO_PKG_VERSION");
    for _ in 0..50 {
        string(stdscr().get_max_y() - 1, 0, &format!("Current version: {}", version));
        if !fs::metadata(file.clone()).is_ok() {
            break;
        }
        sleep(Duration::from_millis(100));
    }
    string(stdscr().get_max_y() - 1, 0, &" ".repeat(stdscr().get_max_x() as usize));
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
