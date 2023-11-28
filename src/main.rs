use clap::Parser;
use serde_json::Value;
use std::{
    time::{ Duration, Instant },
    fs::{ File, OpenOptions },
    fs,
    process::exit,
    thread::sleep,
    io::{ Read, Write },
};
use crosscurses::*;
use lazy_static::lazy_static;

mod zip_func;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    url: String,
    #[arg(short, long, default_value_t = format!("en").to_string())]
    lang: String,
    #[arg(short, long, default_value_t = format!("0").to_string())]
    offset: String,
    #[arg(short, long, default_value_t = format!("0").to_string())]
    database_offset: String,
    #[arg(short, long, default_value_t = format!("*").to_string())]
    title: String,
    #[arg(short, long, default_value_t = format!(".").to_string())]
    folder: String,
    #[arg(short, long, default_value_t = format!("*").to_string())]
    volume: String,
    #[arg(short, long, default_value_t = format!("*").to_string())]
    chapter: String,
    #[arg(short, long, default_value_t = format!("40").to_string())]
    max_consecutive: String,
    #[arg(long)]
    force: bool,
    #[arg(short, long)]
    saver: bool,
    #[arg(short, long)]
    force_delete: bool,
}

fn string(y: i32, x: i32, value: &str) {
    stdscr().mvaddnstr(y, x, value, stdscr().get_max_x() - x);
    stdscr().refresh();
}

async fn print_version(file: String) {
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

lazy_static! {
    static ref ARGS: Args = Args::parse();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = &ARGS.url;
    let file_path: String = format!("mdown_{}.lock", env!("CARGO_PKG_VERSION"));
    let file_path_tm = file_path.to_string();
    if ARGS.force_delete {
        let _ = fs::remove_file(file_path);
        exit(0);
    }
    if fs::metadata(file_path_tm.clone()).is_ok() {
        eprintln!(
            "Lock file has been found;\nSee README.md;\nCannot run multiple instances of mdown"
        );
        exit(100);
    }
    let _ = match File::create(file_path.clone()) {
        Ok(file) => file,
        Err(e) => {
            panic!("Error creating the file: {}", e);
        }
    };
    let re = regex::Regex::new(r"/title/([\w-]+)/").unwrap();

    let _ = initscr();
    curs_set(2);
    start_color();
    tokio::spawn(async move { print_version(file_path_tm).await });
    let mut manga_name: String = "!".to_string();

    if let Some(id) = re.captures(&input).and_then(|id| id.get(1)) {
        string(0, 0, &format!("Extracted ID: {}", id.as_str()));
        let id = id.as_str();
        let manga_name_json = match get_manga_json(id).await {
            Ok(name) => name,
            Err(_) => exit(1),
        };
        match serde_json::from_str(&manga_name_json) {
            Ok(json_value) =>
                match json_value {
                    Value::Object(obj) => {
                        let title_data = obj
                            .get("data")
                            .and_then(|name_data| name_data.get("attributes"))
                            .unwrap_or_else(|| {
                                eprintln!("attributes or title doesn't exist");
                                exit(1);
                            });
                        let manga_name_tmp;
                        if ARGS.title == "*" {
                            manga_name_tmp = get_manga_name(title_data);
                        } else {
                            manga_name_tmp = &ARGS.title;
                        }
                        manga_name = manga_name_tmp.to_owned();
                        let folder = get_folder_name(&manga_name);

                        let _ = fs::create_dir(&folder);
                        let desc = title_data
                            .get("description")
                            .and_then(|description| description.get("en"))
                            .and_then(Value::as_str)
                            .unwrap();
                        let mut desc_file = OpenOptions::new()
                            .read(true)
                            .write(true)
                            .create(true)
                            .open(
                                format!(
                                    "{}\\{}_description.txt",
                                    folder,
                                    get_manga_name(title_data)
                                )
                            )
                            .unwrap();
                        let _ = write!(desc_file, "{}", desc);

                        resolve_manga(id, manga_name_tmp).await;
                    }
                    _ => todo!(),
                }
            Err(err) => eprintln!("Error parsing JSON: {}", err),
        };
    }
    let _ = fs::remove_file(file_path);

    sleep(Duration::from_millis(200));

    let message: String;
    if manga_name == "!" {
        message =
            format!("Ending session: {} has NOT been downloaded, because it was not found", manga_name);
    } else {
        message = format!("Ending session: {} has been downloaded", manga_name);
    }
    string(
        stdscr().get_max_y() - 1,
        0,
        &format!("{}{}", message, " ".repeat((stdscr().get_max_x() as usize) - message.len()))
    );
    stdscr().getch();

    Ok(())
}

fn get_folder_name(manga_name: &String) -> String {
    if ARGS.folder == "name" {
        return manga_name.to_owned();
    } else {
        return ARGS.folder.as_str().to_string();
    }
}

fn get_manga_name(title_data: &Value) -> &str {
    title_data
        .get("title")
        .and_then(|attr_data| attr_data.get("en"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| {
            let get = title_data
                .get("altTitles")
                .and_then(|val| val.as_array())
                .unwrap();
            let mut return_title = "*";
            for title_object in get {
                if let Some(lang_object) = title_object.as_object() {
                    for (lang, title) in lang_object.iter() {
                        if lang == "en" {
                            return_title = title.as_str().unwrap();
                            break;
                        }
                    }
                }
                break;
            }
            if return_title == "*" {
                return_title = title_data.get("ja-ro").and_then(Value::as_str).unwrap();
            }
            return_title
        })
}

async fn resolve_manga(id: &str, manga_name: &str) {
    let arg_database_offset: i32 = ARGS.database_offset.as_str().parse().unwrap();
    let mut arg_force = ARGS.force as bool;
    let going_offset = arg_database_offset;
    let end = 2;
    let mut downloaded: Vec<String> = vec![];
    for _ in 0..end {
        match get_manga(id, going_offset).await {
            Ok((json, _offset)) => {
                let downloaded_temp = download_manga(json, manga_name, arg_force).await;
                for i in 0..downloaded_temp.len() {
                    downloaded.push(downloaded_temp[i].clone());
                }
                clear_screen(1);
            }
            Err(err) => eprintln!("Error: {}", err),
        }
        arg_force = false;
    }
    if downloaded.len() != 0 {
        string(1, 0, "Downloaded files:");
        for i in 0..downloaded.len() {
            (_, downloaded) = resolve_move(i as i32, downloaded.clone(), 2, 1);
        }
    }
}

async fn get_manga_json(id: &str) -> Result<String, reqwest::Error> {
    let base_url = "https://api.mangadex.org/manga/";
    let full_url = format!("{}{}", base_url, id);

    let client = reqwest::Client
        ::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/116.0"
        )
        .build()?;

    let response = client.get(&full_url).send().await?;

    if response.status().is_success() {
        let json = response.text().await?;

        Ok(json)
    } else {
        eprintln!(
            "Error: {}",
            format!("Failed to fetch data from the API. Status code: {:?}", response.status())
        );
        exit(1);
    }
}

fn sort(data: &Vec<Value>) -> Vec<Value> {
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

async fn download_manga(manga_json: String, manga_name: &str, arg_force: bool) -> Vec<String> {
    let folder;
    if ARGS.folder == "name" {
        folder = manga_name.to_owned();
    } else {
        folder = ARGS.folder.as_str().to_string();
    }
    let arg_volume = match ARGS.volume.as_str() {
        "" => "*",
        x => x,
    };
    let arg_chapter = match ARGS.chapter.as_str() {
        "" => "*",
        x => x,
    };
    let mut downloaded: Vec<String> = vec![];
    let arg_offset: i32 = ARGS.offset.as_str().parse().unwrap();
    let language = &ARGS.lang;
    match serde_json::from_str(&manga_json) {
        Ok(json_value) =>
            match json_value {
                Value::Object(obj) => {
                    let data_array = obj
                        .get("data")
                        .and_then(Value::as_array)
                        .unwrap_or_else(|| {
                            eprintln!("data doesn't exist");
                            exit(1);
                        });
                    let data_array = sort(data_array);
                    let mut times = 0;
                    let mut moves = 0;
                    let mut hist: Vec<String> = vec![];
                    for item in 0..data_array.len() {
                        let array_item = data_array.get(item).unwrap_or_else(|| {
                            eprintln!("{} doesn't exist", item);
                            exit(1);
                        });
                        let field_value = array_item.get("id").unwrap_or_else(|| {
                            eprintln!("id doesn't exist");
                            exit(1);
                        });
                        let value = &field_value.to_string();
                        let id = value.trim_matches('"');
                        string(2, 0, &format!(" ({}) Found chapter with id: {}", item as i32, id));
                        let chapter_attr = array_item.get("attributes").unwrap_or_else(|| {
                            eprintln!("attributes doesn't exist");
                            exit(1);
                        });
                        let lang = chapter_attr
                            .get("translatedLanguage")
                            .and_then(Value::as_str)
                            .unwrap_or_else(|| {
                                eprintln!("translatedLanguage doesn't exist");
                                exit(1);
                            });
                        let pages = chapter_attr
                            .get("pages")
                            .and_then(Value::as_u64)
                            .unwrap_or_else(|| {
                                eprintln!("pages doesn't exist");
                                exit(1);
                            });
                        let mut con_chap = true;
                        let chapter_num = chapter_attr
                            .get("chapter")
                            .and_then(Value::as_str)
                            .unwrap_or_else(|| {
                                eprintln!("chapter doesn't exist");
                                exit(1);
                            });
                        if arg_chapter == "*" || arg_chapter == chapter_num {
                            con_chap = false;
                        }
                        let vol;
                        let mut con_vol = true;
                        let title = chapter_attr
                            .get("title")
                            .and_then(Value::as_str)
                            .unwrap_or_else(|| { "" });
                        let mut pr_title = "".to_string();
                        if title != "" {
                            pr_title = format!(" - {}", title);
                        }
                        if let Some(vol_temp) = chapter_attr.get("volume").and_then(Value::as_str) {
                            if arg_volume == "*" || arg_volume == vol_temp {
                                con_vol = false;
                            }
                            vol = format!("Vol.{} ", &vol_temp);
                        } else {
                            vol = "".to_string();
                            con_vol = false;
                        }
                        let vol = &vol;
                        let folder_path = format!(
                            "{} - {}Ch.{}{}",
                            manga_name,
                            vol,
                            chapter_num,
                            pr_title
                        );
                        if
                            fs
                                ::metadata(
                                    format!(
                                        "{}\\{}.cbz",
                                        &folder,
                                        format!("{}", process_filename(folder_path.clone()))
                                    )
                                )
                                .is_ok() &&
                            !arg_force
                        {
                            let message: String = folder_path
                                .chars()
                                .filter(|&c| !"<>:|?*/\\\"'".contains(c))
                                .collect();
                            let al_dow =
                                format!("  Skipping because file is already downloaded {}", message);
                            hist.push(al_dow);
                            (moves, hist) = resolve_move(moves, hist.clone(), 3, 0);
                            continue;
                        }
                        if con_vol {
                            let message = format!(
                                "({}) Skipping because supplied volume doesn't match",
                                item as i32
                            );
                            hist.push(message);
                            (moves, hist) = resolve_move(moves, hist.clone(), 3, 0);
                            continue;
                        }
                        if con_chap {
                            let message = format!(
                                "({}) Skipping because supplied chapter doesn't match",
                                item as i32
                            );
                            hist.push(message);
                            (moves, hist) = resolve_move(moves, hist.clone(), 3, 0);
                            continue;
                        }
                        if lang == language && chapter_num != "This is test" {
                            if arg_offset > (times as i32) {
                                let message = format!(
                                    "({}) Skipping because of offset",
                                    item as i32
                                );
                                hist.push(message);
                                (moves, hist) = resolve_move(moves, hist.clone(), 3, 0);
                                times += 1;
                                continue;
                            }
                            clear_screen(3);
                            let folder_name = &format!(
                                "{} - {}Ch.{}{}",
                                manga_name,
                                vol,
                                chapter_num,
                                pr_title
                            );
                            let folder_path = format!("{}/", process_filename(folder_name.clone()));
                            let mut pr_title_full = "".to_string();
                            if title != "" {
                                pr_title_full = format!(";Title: {}", title);
                            }
                            string(
                                3,
                                0,
                                &format!(
                                    "  Metadata: Language: {};Pages: {};{};Chapter: {}{}",
                                    lang,
                                    pages,
                                    vol,
                                    chapter_num,
                                    pr_title_full
                                )
                            );
                            match get_chapter(id).await {
                                Ok(id) => {
                                    download_chapter(
                                        id,
                                        &manga_name,
                                        title,
                                        vol,
                                        chapter_num,
                                        &folder_path
                                    ).await;
                                }
                                Err(err) => eprintln!("Error: {}", err),
                            }
                            let folder_path = format!("{}/", process_filename(folder_name.clone()));
                            let folder_name = process_filename(folder_name.clone());
                            clear_screen(7);
                            string(
                                7,
                                0,
                                &format!(
                                    "  Converting images to cbz files: {}",
                                    &format!("{}.cbz", folder_path)
                                )
                            );
                            let file_name = format!("{}\\{}.cbz", &folder, folder_name);
                            let _ = zip_func::to_zip(&folder_path, &file_name).await;
                            let _ = fs::remove_dir_all(folder_path);
                            clear_screen(3);
                            downloaded.push(file_name);
                        } else {
                            string(
                                3,
                                0,
                                &format!(
                                    "  Skipping because of wrong language; found '{}', target '{}' ...",
                                    lang,
                                    language
                                )
                            );
                        }
                    }
                }
                _ => {
                    eprintln!("JSON is not an object.");
                }
            }
        Err(err) => eprintln!("Error parsing JSON: {}", err),
    }
    downloaded
}

fn resolve_move(mut moves: i32, mut hist: Vec<String>, start: i32, end: i32) -> (i32, Vec<String>) {
    if moves + start >= stdscr().get_max_y() - end {
        hist.remove(0);
    } else {
        moves += 1;
    }
    for i in 0..moves {
        if (i as usize) == hist.len() {
            break;
        }
        let message = &hist[i as usize];
        string(
            start + i,
            0,
            &format!("{}{}", message, " ".repeat((stdscr().get_max_x() as usize) - message.len()))
        );
    }
    (moves, hist)
}

fn clear_screen(from: i32) {
    for i in from..stdscr().get_max_y() {
        string(i, 0, &" ".repeat(stdscr().get_max_x() as usize));
    }
}

async fn download_chapter(
    manga_json: String,
    manga_name: &str,
    title: &str,
    vol: &str,
    chapter: &str,
    folder_name: &str
) {
    let mut pr_title = "".to_string();
    if title != "" {
        pr_title = format!(" - {}", title);
    }
    string(5, 0, &format!("  Downloading images in folder: {}/:", folder_name));
    match serde_json::from_str(&manga_json) {
        Ok(json_value) =>
            match json_value {
                Value::Object(obj) => {
                    if let Some(data_array) = obj.get("chapter") {
                        if let Some(chapter_hash) = data_array.get("hash").and_then(Value::as_str) {
                            let saver;
                            if ARGS.saver {
                                saver = "dataSaver";
                            } else {
                                saver = "data";
                            }
                            if let Some(images1) = data_array.get(saver).and_then(Value::as_array) {
                                let images_length = images1.len();
                                if let Some(images) = data_array.get(saver) {
                                    let folder_path = process_filename(
                                        format!(
                                            "{} - {}Ch.{}{}",
                                            manga_name,
                                            vol,
                                            chapter,
                                            pr_title
                                        )
                                    );
                                    let lock_file = format!("{}.lock", folder_path);
                                    let mut lock_file_inst = File::create(
                                        lock_file.clone()
                                    ).unwrap();
                                    let _ = write!(lock_file_inst, "0");
                                    let _ = fs::create_dir_all(format!("{}/", folder_path));

                                    let lock_file_wait = folder_path.clone();

                                    tokio::spawn(async move {
                                        wait_for_end(lock_file_wait, images_length).await
                                    });
                                    let _ = fs::create_dir_all(format!("{}/", folder_path));

                                    let start =
                                        stdscr().get_max_x() / 3 - (images_length as i32) / 2;

                                    let iter = ARGS.max_consecutive.parse().unwrap();

                                    let loop_for = ((images_length as f32) / (iter as f32)).ceil();

                                    let mut images_length_temp = images_length;

                                    for i in 0..loop_for as usize {
                                        let end_task;
                                        if images_length_temp > iter {
                                            end_task = (i + 1) * iter;
                                        } else {
                                            end_task = images_length;
                                        }
                                        let start_task = i * iter;
                                        images_length_temp -= iter;
                                        let tasks = (start_task..end_task).map(|item| {
                                            if let Some(image_tmp) = images.get(item) {
                                                let image_temp = image_tmp.to_string();
                                                let chapter_hash = chapter_hash.to_string();
                                                let manga_name = manga_name.to_string();
                                                let title = title.to_string();
                                                let vol = vol.to_string();
                                                let chapter = chapter.to_string();
                                                let image = image_temp
                                                    .trim_matches('"')
                                                    .to_string();

                                                tokio::spawn(async move {
                                                    download_image(
                                                        &chapter_hash,
                                                        &image,
                                                        &manga_name,
                                                        &title,
                                                        &vol,
                                                        &chapter,
                                                        item,
                                                        start,
                                                        iter,
                                                        i
                                                    ).await;
                                                })
                                            } else {
                                                tokio::spawn(async move {
                                                    download_image(
                                                        "",
                                                        "",
                                                        "",
                                                        "",
                                                        "",
                                                        "",
                                                        0,
                                                        start,
                                                        iter,
                                                        i
                                                    ).await;
                                                })
                                            }
                                        });

                                        progress_bar_preparation(start, images_length, 6);

                                        let _: Vec<_> = futures::future
                                            ::join_all(tasks).await
                                            .into_iter()
                                            .collect();
                                    }

                                    let _ = fs::remove_file(lock_file.clone());
                                }
                            } else {
                                eprintln!("Missing data for chapter")
                            }
                        } else {
                            eprintln!("Chapter number missing")
                        }
                    } else {
                        eprintln!("  JSON does not contain a 'chapter' array.");
                    }
                }
                _ => {
                    eprintln!("  JSON is not an object.");
                }
            }
        Err(err) => eprintln!("  Error parsing JSON: {}", err),
    }
}

async fn wait_for_end(file_path: String, images_length: usize) {
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

fn progress_bar_preparation(start: i32, images_length: usize, line: i32) {
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

fn process_filename(filename: String) -> String {
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
        .replace('\'', "")
}

async fn download_image(
    c_hash: &str,
    f_name: &str,
    manga_name: &str,
    name: &str,
    vol: &str,
    chapter: &str,
    page: usize,
    start: i32,
    iter: usize,
    times: usize
) {
    let mut pr_title = "".to_string();
    if name != "" {
        pr_title = format!(" - {}", name);
    }
    let page = page + 1;
    let page_str = page.to_string() + &" ".repeat(3 - page.to_string().len());
    let base_url;
    if ARGS.saver {
        base_url = "https://uploads.mangadex.org/data-saver/";
    } else {
        base_url = "https://uploads.mangadex.org/data/";
    }
    let full_url = format!("{}{}/{}", base_url, c_hash, f_name);
    let folder_name = process_filename(
        format!("{} - {}Ch.{}{}", manga_name, vol, chapter, pr_title)
    );
    let file_name = process_filename(
        format!("{} - {}Ch.{}{} - {}.jpg", manga_name, vol, chapter, pr_title, page)
    );
    let file_name_brief = process_filename(format!("{}Ch.{} - {}.jpg", vol, chapter, page));

    let lock_file = format!("{}.lock", folder_name);

    string(5 + 1, -1 + start + (page as i32), "|");
    string(5 + 1 + (page as i32), 0, "   Sleeping");
    sleep(Duration::from_millis(((page - iter * times) * 50) as u64));
    string(5 + 1 + (page as i32), 0, &format!("   {} Downloading {}", page_str, file_name_brief));
    string(5 + 1, -1 + start + (page as i32), "/");
    let full_path = format!("{}/{}", folder_name, file_name);

    let mut response = reqwest::get(full_url.clone()).await.unwrap();
    let total_size: f32 = response.content_length().unwrap_or(0) as f32;

    let mut file = File::create(full_path).unwrap();
    let mut downloaded = 0;
    let mut last_size = 0.0;

    let interval = Duration::from_millis(250);
    let mut last_check_time = Instant::now();
    string(5 + 1, -1 + start + (page as i32), "\\");
    let final_size = (total_size as f32) / (1024 as f32) / (1024 as f32);

    while fs::metadata(format!("{}.lock", lock_file)).is_ok() {
        sleep(Duration::from_millis(10));
    }
    let mut lock_file_inst = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(format!("{}_{}_final.lock", folder_name, page))
        .unwrap();
    let _ = write!(lock_file_inst, "{:.2}", total_size);

    while let Some(chunk) = response.chunk().await.unwrap() {
        let _ = file.write_all(&chunk);
        downloaded += chunk.len() as u64;
        let current_time = Instant::now();
        if current_time.duration_since(last_check_time) >= interval {
            if (downloaded as f32) != last_size {
                let mut lock_file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(format!("{}_{}.lock", folder_name, page))
                    .unwrap();
                let _ = lock_file.write(format!("{}", downloaded / 1024 / 1024).as_bytes());
            }
            last_check_time = current_time;
            let percentage = ((100.0 / (total_size as f32)) * (downloaded as f32)).round() as i64;
            let perc_string;
            if percentage < 10 {
                perc_string = format!("  {}", percentage);
            } else if percentage < 100 {
                perc_string = format!(" {}", percentage);
            } else {
                perc_string = format!("{}", percentage);
            }
            let message = format!(
                "   {} Downloading {} {}% - {:.2}mb of {:.2}mb [{:.2}mb/s]",
                page_str,
                file_name_brief,
                perc_string,
                (downloaded as f32) / (1024 as f32) / (1024 as f32),
                final_size,
                (((downloaded as f32) - last_size) * 4.0) / (1024 as f32) / (1024 as f32)
            );
            string(
                5 + 1 + (page as i32),
                0,
                &format!(
                    "{} {}",
                    message,
                    "#".repeat(
                        ((((stdscr().get_max_x() - (message.len() as i32)) as f32) /
                            (total_size as f32)) *
                            (downloaded as f32)) as usize
                    )
                )
            );
            last_size = downloaded as f32;
        }
    }

    let message = format!(
        "   {} Downloading {} {}% - {:.2}mb of {:.2}mb",
        page_str,
        file_name_brief,
        100,
        (downloaded as f32) / (1024 as f32) / (1024 as f32),
        (total_size as f32) / (1024 as f32) / (1024 as f32)
    );

    string(
        5 + 1 + (page as i32),
        0,
        &format!(
            "{} {}",
            message,
            "#".repeat(
                ((((stdscr().get_max_x() - (message.len() as i32)) as f32) / (total_size as f32)) *
                    (downloaded as f32)) as usize
            )
        )
    );
    string(5 + 1, -1 + start + (page as i32), "#");
    let mut lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(format!("{}_{}.lock", folder_name, page))
        .unwrap();
    let _ = lock_file.write(format!("{}", (downloaded as f64) / 1024.0 / 1024.0).as_bytes());
}

async fn get_manga(id: &str, offset: i32) -> Result<(String, usize), reqwest::Error> {
    let mut times = 0;
    let mut json: String;
    let mut json_2: String = String::new();
    let mut times_offset: i32;
    loop {
        times_offset = offset + 500 * times;
        string(
            1,
            0,
            &format!(
                "{} {} {}   ",
                times.to_string(),
                "Fetching data with offset",
                times_offset.to_string()
            )
        );
        let base_url = "https://api.mangadex.org/manga/";
        let full_url = format!("{}{}/feed?limit=500&offset={}", base_url, id, times_offset);

        let client = reqwest::Client
            ::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/116.0"
            )
            .build()?;
        let response = client.get(&full_url).send().await?;
        if response.status().is_success() {
            json = response.text().await?;
            if times == 0 {
                json_2 = json.clone();
            }
            let mut offset_temp: usize = 0;
            match serde_json::from_str(&json) {
                Ok(json_value) =>
                    match json_value {
                        Value::Object(obj) => {
                            if let Some(data_array) = obj.get("data").and_then(Value::as_array) {
                                string(
                                    1,
                                    0,
                                    &format!(
                                        "{} Data fetched with offset {}   ",
                                        times.to_string(),
                                        offset.to_string()
                                    )
                                );
                                offset_temp = data_array.len();
                                if offset_temp >= 500 {
                                    if times > 0 {
                                        let mut data1: Value = serde_json
                                            ::from_str(&json)
                                            .expect("Failed to parse JSON");
                                        let data2: Value = serde_json
                                            ::from_str(&json_2)
                                            .expect("Failed to parse JSON");

                                        let data1_array = data1
                                            .get_mut("data")
                                            .expect("No 'data' field found");
                                        let data2_array = data2
                                            .get("data")
                                            .expect("No 'data' field found");

                                        if let Some(data1_array) = data1_array.as_array_mut() {
                                            data1_array.extend(
                                                data2_array.as_array().unwrap().clone()
                                            );
                                        }

                                        json = serde_json
                                            ::to_string(&data1)
                                            .expect("Failed to serialize to JSON");
                                    }
                                    json_2 = json;
                                    times += 1;
                                    continue;
                                } else {
                                    offset_temp = data_array.len();
                                }
                                if times > 0 {
                                    let mut data1: Value = serde_json
                                        ::from_str(&json)
                                        .expect("Failed to parse JSON");
                                    let data2: Value = serde_json
                                        ::from_str(&json_2)
                                        .expect("Failed to parse JSON");

                                    let data1_array = data1
                                        .get_mut("data")
                                        .expect("No 'data' field found");
                                    let data2_array = data2
                                        .get("data")
                                        .expect("No 'data' field found");

                                    if let Some(data1_array) = data1_array.as_array_mut() {
                                        data1_array.extend(data2_array.as_array().unwrap().clone());
                                    }

                                    json = serde_json
                                        ::to_string(&data1)
                                        .expect("Failed to serialize to JSON");
                                }
                            }
                        }
                        _ => todo!(),
                    }
                Err(err) => eprintln!("  Error parsing JSON: {}", err),
            }
            return Ok((json, offset_temp));
        } else {
            eprintln!(
                "Error: {}",
                format!("Failed to fetch data from the API. Status code: {:?}", response.status())
            );
            exit(1);
        }
    }
}

async fn get_chapter(id: &str) -> Result<String, reqwest::Error> {
    loop {
        string(4, 0, "Retrieving chapter info");

        let base_url = "https://api.mangadex.org/at-home/server/";
        let full_url = format!("{}{}", base_url, id);

        let client = reqwest::Client
            ::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/116.0"
            )
            .build()?;

        let response = client.get(&full_url).send().await?;

        if response.status().is_success() {
            let json = response.text().await?;

            string(4, 0, "Retrieving chapter info DONE");
            return Ok(json);
        } else {
            string(
                5,
                0,
                &format!(
                    "Error: Failed to fetch data from the API. Status code: {:?} {}",
                    response.status(),
                    response.text().await.unwrap()
                )
            );
            string(6, 0, "Sleeping for 60 seconds ...");
            progress_bar_preparation(stdscr().get_max_x() - 30, 60, 7);
            for i in 0..60 {
                sleep(Duration::from_millis(1000));
                string(7, stdscr().get_max_x() - 29 + i, "#");
            }
        }
    }
}
