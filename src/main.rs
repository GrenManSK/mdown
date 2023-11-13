use clap::Parser;
use serde_json::Value;
use std::fs::File;
use std::io::Write;
use std::iter::Iterator;
use std::process;
use std::thread;
mod zip_func;
use crosscurses::*;
use std::time::{ Duration, Instant };
use lazy_static::lazy_static;

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
    #[arg(short, long, default_value_t = format!("*").to_string())]
    volume: String,
    #[arg(short, long, default_value_t = format!("*").to_string())]
    chapter: String,
    #[arg(short, long, default_value_t = format!("40").to_string())]
    pack: String,
    #[arg(short, long)]
    force: bool,
}
fn string(y: i32, x: i32, value: &str) {
    stdscr().mvaddnstr(y, x, value, stdscr().get_max_x() - x);
    stdscr().refresh();
}

fn print_version() {
    let version = env!("CARGO_PKG_VERSION");
    string(stdscr().get_max_y() - 1, 0, format!("Current version: {}", version).as_str());
    thread::sleep(Duration::from_millis(5000));
    string(stdscr().get_max_y() - 1, 0, " ".repeat(stdscr().get_max_x() as usize).as_str());
}
lazy_static! {
    static ref ARGS: Args = Args::parse();
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = initscr();
    curs_set(2);
    start_color();

    let input = &ARGS.url;

    let re = regex::Regex::new(r"/title/([\w-]+)/").unwrap();
    tokio::spawn(async move { print_version() });

    if let Some(id) = re.captures(&input).and_then(|id| id.get(1)) {
        string(0, 0, format!("Extracted ID: {}", id.as_str()).as_str());
        let id = id.as_str();
        let manga_name_json = match get_manga_name(id).await {
            Ok(name) => name,
            Err(_) => process::exit(1),
        };
        match serde_json::from_str(&manga_name_json) {
            Ok(json_value) =>
                match json_value {
                    Value::Object(obj) => {
                        let title_data = obj
                            .get("data")
                            .and_then(|name_data| name_data.get("attributes"))
                            .unwrap_or_else(|| {
                                println!("attributes or title doesn't exist");
                                process::exit(1);
                            });

                        resolve_manga_verbose(id, title_data).await;
                    }
                    _ => todo!(),
                }
            Err(err) => println!("Error parsing JSON: {}", err),
        };
    }
    stdscr().getch();

    Ok(())
}

async fn resolve_manga_verbose(id: &str, title_data: &Value) {
    let manga_name;
    if ARGS.title == "*" {
        manga_name = title_data
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
            });
    } else {
        manga_name = ARGS.title.as_str();
    }
    resolve_manga(id, manga_name).await;
    let message = format!("Ending session: {} has been downloaded", manga_name);
    string(
        stdscr().get_max_y() - 1,
        0,
        format!(
            "{}{}",
            message,
            " ".repeat((stdscr().get_max_x() as usize) - message.len())
        ).as_str()
    );
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
            Err(err) => println!("Error: {}", err),
        }
        arg_force = false;
    }
    string(1, 0, "Downloaded files:");
    for i in 0..downloaded.len() {
        (_, downloaded) = resolve_move(i as i32, downloaded.clone(), 2, 1);
    }
}
async fn get_manga_name(id: &str) -> Result<String, reqwest::Error> {
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
        println!(
            "Error: {}",
            format!("Failed to fetch data from the API. Status code: {:?}", response.status())
        );
        process::exit(1);
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
    let language = ARGS.lang.as_str();
    match serde_json::from_str(&manga_json) {
        Ok(json_value) =>
            match json_value {
                Value::Object(obj) => {
                    let data_array = obj
                        .get("data")
                        .and_then(Value::as_array)
                        .unwrap_or_else(|| {
                            println!("data doesn't exist");
                            process::exit(1);
                        });
                    let data_array = sort(data_array);
                    let mut times = 0;
                    let mut moves = 0;
                    let mut hist: Vec<String> = vec![];
                    for item in 0..data_array.len() {
                        let array_item = data_array.get(item).unwrap_or_else(|| {
                            println!("{} doesn't exist", item);
                            process::exit(1);
                        });
                        let field_value = array_item.get("id").unwrap_or_else(|| {
                            println!("id doesn't exist");
                            process::exit(1);
                        });
                        let value = &field_value.to_string();
                        let id = value.trim_matches('"');
                        string(
                            2,
                            0,
                            format!(" ({}) Found chapter with id: {}", item as i32, id).as_str()
                        );
                        let chapter_attr = array_item.get("attributes").unwrap_or_else(|| {
                            println!("attributes doesn't exist");
                            process::exit(1);
                        });
                        let lang = chapter_attr
                            .get("translatedLanguage")
                            .and_then(Value::as_str)
                            .unwrap_or_else(|| {
                                println!("translatedLanguage doesn't exist");
                                process::exit(1);
                            });
                        let pages = chapter_attr
                            .get("pages")
                            .and_then(Value::as_u64)
                            .unwrap_or_else(|| {
                                println!("pages doesn't exist");
                                process::exit(1);
                            });
                        let mut con_chap = true;
                        let chapter_num = chapter_attr
                            .get("chapter")
                            .and_then(Value::as_str)
                            .unwrap_or_else(|| {
                                println!("chapter doesn't exist");
                                process::exit(1);
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
                        let vol = vol.as_str();
                        let folder_path = format!(
                            "{} - {}Ch.{}{}",
                            manga_name,
                            vol,
                            chapter_num,
                            pr_title
                        );
                        if
                            tokio::fs
                                ::metadata(
                                    format!(
                                        "{}.cbz",
                                        format!(
                                            "{}",
                                            folder_path
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
                                        )
                                    )
                                ).await
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
                            let folder_path = format!(
                                "{}/",
                                folder_name
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
                            );
                            let mut pr_title_full = "".to_string();
                            if title != "" {
                                pr_title_full = format!(";Title: {}", title);
                            }
                            string(
                                3,
                                0,
                                format!(
                                    "  Metadata: Language: {};Pages: {};Vol: {};Chapter: {}{}",
                                    lang,
                                    pages,
                                    vol,
                                    chapter_num,
                                    pr_title_full
                                ).as_str()
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
                                Err(err) => println!("Error: {}", err),
                            }
                            let folder_path = format!(
                                "{}/",
                                folder_name
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
                            );
                            let folder_name = folder_name
                                .replace('<', "")
                                .replace('>', "")
                                .replace(':', "")
                                .replace('|', "")
                                .replace('?', "")
                                .replace('*', "")
                                .replace('/', "")
                                .replace('\\', "")
                                .replace('"', "")
                                .replace('\'', "");
                            clear_screen(7);
                            string(
                                7,
                                0,
                                format!(
                                    "  Converting images to cbz files: {}",
                                    format!("{}.cbz", folder_path).as_str()
                                ).as_str()
                            );
                            let file_name = format!("{}.cbz", folder_name);
                            let _ = zip_func::to_zip(
                                folder_path.as_str(),
                                file_name.as_str()
                            ).await;
                            let _ = tokio::fs::remove_dir_all(folder_path).await;
                            clear_screen(3);
                            downloaded.push(file_name);
                        } else {
                            string(
                                3,
                                0,
                                format!(
                                    "  Skipping because of wrong language; found '{}', target '{}' ...",
                                    lang,
                                    language
                                ).as_str()
                            );
                        }
                    }
                }
                _ => {
                    println!("JSON is not an object.");
                }
            }
        Err(err) => println!("Error parsing JSON: {}", err),
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
        let message = hist[i as usize].as_str();
        string(
            start + i,
            0,
            format!(
                "{}{}",
                message,
                " ".repeat((stdscr().get_max_x() as usize) - message.len())
            ).as_str()
        );
    }
    (moves, hist)
}

fn clear_screen(from: i32) {
    for i in from..stdscr().get_max_y() {
        string(i, 0, " ".repeat(stdscr().get_max_x() as usize).as_str());
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
    string(5, 0, format!("  Downloading images in folder: {}/:", folder_name).as_str());
    match serde_json::from_str(&manga_json) {
        Ok(json_value) =>
            match json_value {
                Value::Object(obj) => {
                    if let Some(data_array) = obj.get("chapter") {
                        if let Some(chapter_hash) = data_array.get("hash").and_then(Value::as_str) {
                            if let Some(images1) = data_array.get("data").and_then(Value::as_array) {
                                let images_length = images1.len();
                                if let Some(images) = data_array.get("data") {
                                    let folder_path = format!(
                                        "{} - {}Ch.{}{}",
                                        manga_name,
                                        vol,
                                        chapter,
                                        pr_title
                                    );
                                    let _ = tokio::fs::create_dir_all(
                                        format!(
                                            "{}/",
                                            folder_path
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
                                        )
                                    ).await;

                                    let folder_path = format!(
                                        "{} - {}Ch.{}{}",
                                        manga_name,
                                        vol,
                                        chapter,
                                        pr_title
                                    );
                                    let _ = tokio::fs::create_dir_all(
                                        format!(
                                            "{}/",
                                            folder_path
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
                                        )
                                    );

                                    let start =
                                        stdscr().get_max_x() / 3 - (images_length as i32) / 2;

                                    let iter = ARGS.pack.parse().unwrap();

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
                                }
                            } else {
                                println!("Missing data for chapter")
                            }
                        } else {
                            println!("Chapter number missing")
                        }
                    } else {
                        println!("  JSON does not contain a 'chapter' array.");
                    }
                }
                _ => {
                    println!("  JSON is not an object.");
                }
            }
        Err(err) => println!("  Error parsing JSON: {}", err),
    }
}

fn progress_bar_preparation(start: i32, images_length: usize, line: i32) {
    string(line, 0, format!("{}|", "-".repeat((start as usize) - 1).as_str()).as_str());
    string(
        line,
        start + (images_length as i32),
        format!(
            "|{}",
            "-"
                .repeat(
                    (stdscr().get_max_x() as usize) -
                        ((start + (images_length as i32) + 1) as usize)
                )
                .as_str()
        ).as_str()
    );
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
    let page_str = page.to_string() + " ".repeat(3 - page.to_string().len()).as_str();
    let base_url = "https://uploads.mangadex.org/data/";
    let full_url = format!("{}{}/{}", base_url, c_hash, f_name);

    let folder_name = format!("{} - {}Ch.{}{}", manga_name, vol, chapter, pr_title);
    let file_name = format!("{} - {}Ch.{}{} - {}.jpg", manga_name, vol, chapter, pr_title, page);
    let file_name_brief = format!("{}Ch.{} - {}.jpg", vol, chapter, page);
    let folder_name = folder_name
        .replace('<', "")
        .replace('>', "")
        .replace(':', "")
        .replace('|', "")
        .replace('?', "")
        .replace('*', "")
        .replace('/', "")
        .replace('\\', "")
        .replace('"', "")
        .replace('\'', "");
    let file_name = file_name
        .replace('<', "")
        .replace('>', "")
        .replace(':', "")
        .replace('|', "")
        .replace('?', "")
        .replace('*', "")
        .replace('/', "")
        .replace('\\', "")
        .replace('"', "")
        .replace('\'', "");
    let file_name_brief = file_name_brief
        .replace('<', "")
        .replace('>', "")
        .replace(':', "")
        .replace('|', "")
        .replace('?', "")
        .replace('*', "")
        .replace('/', "")
        .replace('\\', "")
        .replace('"', "")
        .replace('\'', "");

    string(5 + 1, -1 + start + (page as i32), "|");
    string(5 + 1 + (page as i32), 0, "   Sleeping");
    thread::sleep(Duration::from_millis(((page - iter * times) * 50) as u64));
    string(
        5 + 1 + (page as i32),
        0,
        format!("   {} Downloading {}", page_str, file_name_brief).as_str()
    );
    string(5 + 1, -1 + start + (page as i32), "/");
    let full_path = format!("{}/{}", folder_name, file_name);

    // Reqwest setupClient
    let mut response = reqwest::get(full_url.clone()).await.unwrap();
    let total_size = response.content_length().unwrap_or(0);

    // download chunks
    let mut file = File::create(full_path).unwrap();
    let mut downloaded = 0;
    let mut last_size = 0.0;

    let interval = Duration::from_millis(250);
    let mut last_check_time = Instant::now();
    string(5 + 1, -1 + start + (page as i32), "\\");

    while let Some(chunk) = response.chunk().await.unwrap() {
        let _ = file.write_all(&chunk);
        downloaded += chunk.len() as u64;

        let current_time = Instant::now();
        if current_time.duration_since(last_check_time) >= interval {
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
                (total_size as f32) / (1024 as f32) / (1024 as f32),
                (((downloaded as f32) - last_size) * 4.0) / (1024 as f32) / (1024 as f32)
            );
            string(
                5 + 1 + (page as i32),
                0,
                format!(
                    "{} {}",
                    message,
                    "#".repeat(
                        ((((stdscr().get_max_x() - (message.len() as i32)) as f32) /
                            (total_size as f32)) *
                            (downloaded as f32)) as usize
                    )
                ).as_str()
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
        format!(
            "{} {}",
            message,
            "#".repeat(
                ((((stdscr().get_max_x() - (message.len() as i32)) as f32) / (total_size as f32)) *
                    (downloaded as f32)) as usize
            )
        ).as_str()
    );
    string(5 + 1, -1 + start + (page as i32), "#");
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
            format!(
                "{} {} {}",
                times.to_string(),
                "Fetching data with offset",
                times_offset.to_string()
            ).as_str()
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
                                    format!(
                                        "{} {} {}",
                                        times.to_string(),
                                        "Data fetched with offset",
                                        offset.to_string()
                                    ).as_str()
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
                Err(err) => println!("  Error parsing JSON: {}", err),
            }
            return Ok((json, offset_temp));
        } else {
            println!(
                "Error: {}",
                format!("Failed to fetch data from the API. Status code: {:?}", response.status())
            );
            process::exit(1);
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
                format!(
                    "Error: {}",
                    format!(
                        "Failed to fetch data from the API. Status code: {:?} {}",
                        response.status(),
                        response.text().await?
                    )
                ).as_str()
            );
            string(6, 0, "Sleeping for 60 seconds ...");
            progress_bar_preparation(stdscr().get_max_x() - 30, 60, 7);
            for i in 0..60 {
                thread::sleep(Duration::from_millis(1000));
                string(7, stdscr().get_max_x() - 29 + i, "#");
            }
        }
    }
}
