use clap::Parser;
use serde_json::Value;
use std::{ fs::{ self, File }, process::exit, io::Write, env, sync::Mutex };
use crosscurses::*;
use lazy_static::lazy_static;
use ctrlc;

mod zip_func;
mod download;
mod resolute;
mod getter;
mod utils;
mod web;

lazy_static! {
    pub(crate) static ref IS_END: Mutex<bool> = Mutex::new(false);
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("None"))]
    url: String,
    #[arg(short, long, default_value_t = String::from("en"))]
    lang: String,
    #[arg(short, long, default_value_t = String::from("0"))]
    offset: String,
    #[arg(short, long, default_value_t = String::from("0"))]
    database_offset: String,
    #[arg(short, long, default_value_t = String::from("*"))]
    title: String,
    #[arg(short, long, default_value_t = String::from("."))]
    folder: String,
    #[arg(short, long, default_value_t = String::from("*"))]
    volume: String,
    #[arg(short, long, default_value_t = String::from("*"))]
    chapter: String,
    #[arg(short, long, default_value_t = String::from("40"))]
    max_consecutive: String,
    #[arg(long)]
    force: bool,
    #[arg(short, long)]
    saver: bool,
    #[arg(long)]
    force_delete: bool,
    #[arg(long, default_value_t = String::from("./"))]
    cwd: String,
    #[arg(long)]
    stat: bool,
    #[arg(short, long)]
    web: bool,
    #[arg(short, long, default_value_t = String::from(""))]
    encode: String,
    #[arg(long)]
    log: bool,
}

fn string(y: i32, x: i32, value: &str) {
    if !ARGS.web {
        stdscr().mvaddnstr(y, x, value, MAXPOINTS.max_x - x);
        stdscr().refresh();
    }
}
pub(crate) struct MaxPoints {
    max_x: i32,
    max_y: i32,
}

lazy_static! {
    pub(crate) static ref ARGS: Args = Args::parse();
    pub(crate) static ref MAXPOINTS: MaxPoints = MaxPoints {
        max_x: stdscr().get_max_x(),
        max_y: stdscr().get_max_y(),
    };
}
#[tokio::main]
async fn main() {
    // cwd
    if let Err(err) = env::set_current_dir(ARGS.cwd.as_str()) {
        eprintln!("Failed to set working directory: {}", err);
        exit(1);
    }
    if ARGS.encode != "" {
        println!("{}", web::encode(&ARGS.encode));
        exit(0);
    }
    let _ = fs::create_dir(".cache");
    // web
    if ARGS.web {
        ctrlc
            ::set_handler(|| {
                println!("[user] Ctrl+C received! Exiting...\n[web] Closing server");
                if utils::is_directory_empty(".cache\\") {
                    let _ = fs::remove_dir_all(".cache");
                }
                std::process::exit(0);
            })
            .expect("Error setting Ctrl+C handler");
        web::web().await;
        exit(0);
    }
    let (file_path, file_path_tm, file_path_temp) = utils::resolve_start();
    let _ = initscr();
    curs_set(2);
    start_color();
    tokio::spawn(async move { utils::print_version(file_path_tm).await });
    tokio::spawn(async move { utils::ctrl_handler(file_path_temp).await });

    let mut manga_name: String = String::from("!");
    let mut status_code = reqwest::StatusCode::from_u16(200).unwrap();
    if let Some(id) = utils::resolve_regex(&ARGS.url) {
        let id: &str = id.as_str();
        string(0, 0, &format!("Extracted ID: {}", id));
        match getter::get_manga_json(id).await {
            Ok(manga_name_json) => {
                let json_value = serde_json::from_str(&manga_name_json).unwrap();
                if let Value::Object(obj) = json_value {
                    manga_name = resolute::resolve(obj, id, Some(String::new())).await;
                } else {
                    eprintln!("Unexpected JSON value");
                    return;
                }
            }
            Err(code) => {
                status_code = code;
            }
        }
    }

    utils::resolve_end(file_path, manga_name, status_code);

    // Final key input is in utils::ctrl_handler
}

pub(crate) async fn download_manga(
    manga_json: String,
    manga_name: &str,
    arg_force: bool,
    handle_id: Option<String>
) -> Vec<String> {
    let handle_id = handle_id.unwrap_or_default();
    let folder = getter::get_folder_name(manga_name);
    let arg_volume = getter::get_arg(ARGS.volume.to_string());
    let arg_chapter = getter::get_arg(ARGS.chapter.to_string());
    let arg_offset: i32 = getter::get_arg(ARGS.offset.to_string()).parse().unwrap();
    let (mut downloaded, mut hist) = (vec![], vec![]);
    let (mut times, mut moves) = (0, 0);
    let language = ARGS.lang.to_string();
    match serde_json::from_str(&manga_json) {
        Ok(json_value) =>
            match json_value {
                Value::Object(obj) => {
                    let data_array = utils::sort(
                        obj
                            .get("data")
                            .and_then(Value::as_array)
                            .unwrap_or_else(|| {
                                eprintln!("data doesn't exist");
                                exit(1);
                            })
                    );
                    for item in 0..data_array.len() {
                        let array_item = getter::get_attr_as_same_from_vec(&data_array, item);
                        let value = getter::get_attr_as_same(array_item, "id").to_string();
                        let id = value.trim_matches('"');

                        let message = format!(" ({}) Found chapter with id: {}", item as i32, id);
                        if ARGS.web {
                            println!("[downloader @{}] {}", handle_id, message);
                        }
                        string(2, 0, &message);

                        let (chapter_attr, lang, pages, chapter_num, mut title) =
                            getter::get_metadata(array_item);

                        title = resolute::title(title);

                        let con_chap = resolute::resolve_skip(arg_chapter.clone(), chapter_num);
                        let vol;
                        let vol_temp = getter::get_attr_as_str(chapter_attr, "volume");
                        if vol_temp == "" {
                            vol = String::from("");
                        } else {
                            vol = format!("Vol.{} ", &vol_temp);
                        }

                        let con_vol = resolute::resolve_skip(arg_volume.clone(), vol_temp);

                        let filename = utils::FileName {
                            manga_name: manga_name.to_string(),
                            vol: vol.to_string(),
                            chapter_num: chapter_num.to_string(),
                            title: title.to_string(),
                            folder: folder.to_string(),
                        };
                        let folder_path = filename.get_folder_name();
                        if fs::metadata(filename.get_file_w_folder()).is_ok() && !arg_force {
                            (moves, hist) = utils::skip(
                                folder_path
                                    .chars()
                                    .filter(|&c| !"<>:|?*/\\\"'".contains(c))
                                    .collect(),
                                item,
                                moves,
                                hist.clone(),
                                handle_id.clone()
                            );
                            continue;
                        }

                        if con_vol {
                            (moves, hist) = utils::skip_didnt_match(
                                "volume",
                                item,
                                moves,
                                hist.clone()
                            );
                            continue;
                        }
                        if con_chap {
                            (moves, hist) = utils::skip_didnt_match(
                                "chapter",
                                item,
                                moves,
                                hist.clone()
                            );
                            continue;
                        }
                        if lang == language && chapter_num != "This is test" {
                            if arg_offset > (times as i32) {
                                (moves, hist) = utils::skip_offset(
                                    item,
                                    moves,
                                    hist,
                                    handle_id.clone()
                                );
                                times += 1;
                                continue;
                            }
                            utils::clear_screen(3);
                            let folder_path = filename.get_folder_w_end();
                            let mut pr_title_full = "".to_string();
                            if title != "" {
                                pr_title_full = format!(";Title: {}", title);
                            }
                            let message = format!(
                                "  Metadata: Language: {};Pages: {};{};Chapter: {}{}",
                                lang,
                                pages,
                                vol,
                                chapter_num,
                                pr_title_full
                            );
                            if ARGS.web {
                                println!("[downloader @{}] {}", handle_id, message);
                            }
                            string(3, 0, &message);

                            match getter::get_chapter(id).await {
                                Ok(id) => {
                                    download_chapter(
                                        id,
                                        &manga_name,
                                        title,
                                        &vol,
                                        chapter_num,
                                        &filename,
                                        handle_id.clone()
                                    ).await;
                                }
                                Err(err) => eprintln!("Error: {}", err),
                            }
                            resolute::resolve_group(array_item, manga_name).await;
                            utils::clear_screen(7);
                            string(
                                7,
                                0,
                                &format!("  Converting images to cbz files: {}.cbz", folder_path)
                            );
                            let file_name = filename.get_file_w_folder();
                            let _ = zip_func::to_zip(
                                &folder_path,
                                &file_name,
                                handle_id.clone()
                            ).await;
                            let _ = fs::remove_dir_all(folder_path);
                            utils::clear_screen(3);
                            if ARGS.web {
                                resolute::DOWNLOADED.lock().unwrap().push(file_name);
                            } else {
                                downloaded.push(filename.get_file_w_folder_w_cwd());
                            }
                        } else {
                            let message = format!(
                                "Skipping because of wrong language; found '{}', target '{}' ...",
                                lang,
                                language
                            );
                            string(3, 0, &format!("  {}", message));

                            if ARGS.web {
                                println!("[downloader @{}]   ({}) {}", handle_id, item, message);
                            }
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

pub(crate) async fn download_chapter(
    manga_json: String,
    manga_name: &str,
    title: &str,
    vol: &str,
    chapter: &str,
    filename: &utils::FileName,
    handle_id: String
) {
    string(5, 0, &format!("  Downloading images in folder: {}:", filename.get_folder_name()));
    if ARGS.web {
        println!(
            "[downloader @{}] Downloading images in folder: {}",
            handle_id,
            filename.get_folder_name()
        );
        let mut current_chapter = resolute::CURRENT_CHAPTER.lock().unwrap();
        current_chapter.clear();
        current_chapter.push_str(&&filename.get_folder_name());
    }
    match serde_json::from_str(&manga_json) {
        Ok(json_value) =>
            match json_value {
                Value::Object(obj) => {
                    if let Some(data_array) = obj.get("chapter") {
                        if let Some(chapter_hash) = data_array.get("hash").and_then(Value::as_str) {
                            let saver = getter::get_saver();
                            if
                                let Some(images1) = data_array
                                    .get(saver.clone())
                                    .and_then(Value::as_array)
                            {
                                let images_length = images1.len();

                                *resolute::CURRENT_PAGE_MAX.lock().unwrap() =
                                    images_length.clone() as u64;

                                if let Some(images) = data_array.get(saver) {
                                    let lock_file = filename.get_lock();
                                    let mut lock_file_inst = File::create(
                                        lock_file.clone()
                                    ).unwrap();
                                    let _ = write!(lock_file_inst, "0");
                                    let _ = fs::create_dir_all(filename.get_folder_w_end());

                                    let lock_file_wait = filename.get_folder_name();

                                    tokio::spawn(async move {
                                        utils::wait_for_end(lock_file_wait, images_length).await
                                    });
                                    let _ = fs::create_dir_all(filename.get_folder_w_end());

                                    let start = MAXPOINTS.max_x / 3 - (images_length as i32) / 2;

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
                                            let image_temp = getter
                                                ::get_attr_as_same_as_index(images, item)
                                                .to_string();
                                            let chapter_hash = chapter_hash.to_string();
                                            let manga_name = manga_name.to_string();
                                            let title = title.to_string();
                                            let vol = vol.to_string();
                                            let chapter = chapter.to_string();
                                            let image = image_temp.trim_matches('"').to_string();
                                            let handle_id_tmp = handle_id.clone();

                                            tokio::spawn(async move {
                                                download::download_image(
                                                    &chapter_hash,
                                                    &image,
                                                    &manga_name,
                                                    &title,
                                                    &vol,
                                                    &chapter,
                                                    item,
                                                    start,
                                                    iter,
                                                    i,
                                                    handle_id_tmp
                                                ).await;
                                            })
                                        });

                                        utils::progress_bar_preparation(start, images_length, 6);

                                        let _: Vec<_> = futures::future
                                            ::join_all(tasks).await
                                            .into_iter()
                                            .collect();
                                        if *IS_END.lock().unwrap() || false {
                                            return;
                                        }
                                    }

                                    *resolute::CURRENT_PAGE.lock().unwrap() = 0;
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
