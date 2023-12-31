use clap::Parser;
use serde_json::Value;
use std::{ fs::{ self, File }, process::exit, io::Write };
use crosscurses::*;
use lazy_static::lazy_static;

mod zip_func;
mod download;
mod resolute;
mod getter;
mod utils;

static mut IS_END: bool = false;

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
    #[arg(short, long)]
    force_delete: bool,
}

fn string(y: i32, x: i32, value: &str) {
    stdscr().mvaddnstr(y, x, value, MAXPOINTS.max_x - x);
    stdscr().refresh();
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
    let (file_path, file_path_tm, file_path_temp) = utils::resolve_start();
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
                    manga_name = resolute::resolve(obj, id).await;
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

pub(crate) async fn download_manga(manga_json: String, manga_name: &str, arg_force: bool) -> Vec<String> {
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
                                hist.clone()
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
                        string(2, 0, &format!(" ({}) Found chapter with id: {}", item as i32, id));
                        if lang == language && chapter_num != "This is test" {
                            if arg_offset > (times as i32) {
                                (moves, hist) = utils::skip_offset(item, moves, hist);
                                times += 1;
                                continue;
                            }
                            utils::clear_screen(3);
                            let folder_path = filename.get_folder_w_end();
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
                            match getter::get_chapter(id).await {
                                Ok(id) => {
                                    download_chapter(
                                        id,
                                        &manga_name,
                                        title,
                                        &vol,
                                        chapter_num,
                                        &filename
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
                            let _ = zip_func::to_zip(&folder_path, &file_name).await;
                            let _ = fs::remove_dir_all(folder_path);
                            utils::clear_screen(3);
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

pub(crate) async fn download_chapter(
    manga_json: String,
    manga_name: &str,
    title: &str,
    vol: &str,
    chapter: &str,
    filename: &utils::FileName
) {
    string(5, 0, &format!("  Downloading images in folder: {}:", filename.get_folder_name()));
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
                                                    i
                                                ).await;
                                            })
                                        });

                                        utils::progress_bar_preparation(start, images_length, 6);

                                        let _: Vec<_> = futures::future
                                            ::join_all(tasks).await
                                            .into_iter()
                                            .collect();
                                        if (unsafe { IS_END }) || false {
                                            return;
                                        }
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
