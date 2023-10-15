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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    url: String,
    #[arg(short, long, default_value_t)]
    lang: String,
    #[arg(short, long, default_value_t)]
    offset: String,
}
fn string(y: i32, x: i32, value: &str) {
    stdscr().mvaddnstr(y, x, value, stdscr().get_max_x() - x);
    stdscr().refresh();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = initscr();
    curs_set(2);
    start_color();
    let args = Args::parse();

    let language = match args.lang.as_str() {
        "" => "en",
        x => x,
    };
    let arg_offset = match args.offset.as_str() {
        "" => "0",
        x => x,
    };
    let arg_offset = arg_offset.parse().unwrap();

    let input = args.url;

    let re = regex::Regex::new(r"/title/([\w-]+)/").unwrap();

    if let Some(captures) = re.captures(&input) {
        if let Some(id) = captures.get(1) {
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
                            if let Some(name_data) = obj.get("data") {
                                if let Some(attr_data) = name_data.get("attributes") {
                                    if let Some(title_data) = attr_data.get("title") {
                                        if
                                            let Some(manga_name) = title_data
                                                .get("en")
                                                .and_then(Value::as_str)
                                        {
                                            let going_offset = arg_offset;
                                            for _ in 0..2 {
                                                match get_manga(id, going_offset).await {
                                                    Ok((json, _offset)) => {
                                                        download_manga(
                                                            json,
                                                            language,
                                                            manga_name
                                                        ).await;
                                                    }
                                                    Err(err) => println!("Error: {}", err),
                                                }
                                            }
                                            string(
                                                stdscr().get_max_y(),
                                                0,
                                                format!("Ending session: {} has been downloaded", manga_name).as_str()
                                            );
                                        } else {
                                            println!("eng_title is not available");
                                        }
                                    } else {
                                        println!("title is not available");
                                    }
                                } else {
                                    println!("attributes is not available");
                                }
                            } else {
                                println!("data is not available");
                            }
                        }
                        _ => todo!(),
                    }
                Err(err) => println!("Error parsing JSON: {}", err),
            };
        }
    } else {
        println!("ID not found in the URL.");
    }

    Ok(())
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

async fn download_manga(manga_json: String, language: &str, manga_name: &str) {
    match serde_json::from_str(&manga_json) {
        Ok(json_value) =>
            match json_value {
                Value::Object(obj) => {
                    if let Some(data_array) = obj.get("data").and_then(Value::as_array) {
                        let data_array = sort(data_array);
                        for item in 0..data_array.len() {
                            if let Some(array_item) = data_array.get(item) {
                                if let Some(field_value) = array_item.get("id") {
                                    let value = &field_value.to_string();
                                    let id = value.trim_matches('"');
                                    string(
                                        2,
                                        0,
                                        format!(
                                            " ({}) Found chapter with id: {}",
                                            item as i32,
                                            id
                                        ).as_str()
                                    );
                                    if let Some(chapter_attr) = array_item.get("attributes") {
                                        if
                                            let Some(lang) = chapter_attr
                                                .get("translatedLanguage")
                                                .and_then(Value::as_str)
                                        {
                                            if
                                                let Some(pages) = chapter_attr
                                                    .get("pages")
                                                    .and_then(Value::as_u64)
                                            {
                                                if
                                                    let Some(chapter_num) = chapter_attr
                                                        .get("chapter")
                                                        .and_then(Value::as_str)
                                                {
                                                    let title;
                                                    let vol;
                                                    if
                                                        let Some(title_temp) = chapter_attr
                                                            .get("title")
                                                            .and_then(Value::as_str)
                                                    {
                                                        title = title_temp;
                                                    } else {
                                                        title = "";
                                                    }
                                                    if
                                                        let Some(vol_temp) = chapter_attr
                                                            .get("volume")
                                                            .and_then(Value::as_str)
                                                    {
                                                        vol = format!("Vol.{} ", &vol_temp);
                                                    } else {
                                                        vol = "".to_string();
                                                    }
                                                    let vol = vol.as_str();

                                                    let folder_path = format!(
                                                        "{} - {}Ch.{} - {}",
                                                        manga_name,
                                                        vol,
                                                        chapter_num,
                                                        title
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
                                                            .is_ok()
                                                    {
                                                        string(
                                                            3,
                                                            0,
                                                            format!(
                                                                "  Skipping because file is already downloaded {}",
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
                                                            ).as_str()
                                                        );
                                                        continue;
                                                    }
                                                    if
                                                        lang == language &&
                                                        chapter_num != "This is test"
                                                    {
                                                        let folder_name = &format!(
                                                            "{} - {}Ch.{} - {}",
                                                            manga_name,
                                                            vol,
                                                            chapter_num,
                                                            title
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
                                                        string(
                                                            3,
                                                            0,
                                                            format!(
                                                                "  Metadata: Language: {};Pages: {};Vol: {};Chapter: {};Title: {}",
                                                                lang,
                                                                pages,
                                                                vol,
                                                                chapter_num,
                                                                title
                                                            ).as_str()
                                                        );
                                                        match get_chapter(id).await {
                                                            Ok(id) => {
                                                                download_chapter(
                                                                    id,
                                                                    manga_name,
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
                                                        for i in 7..stdscr().get_max_y() {
                                                            string(
                                                                i as i32,
                                                                0,
                                                                " "
                                                                    .repeat(
                                                                        stdscr().get_max_x() as usize
                                                                    )
                                                                    .as_str()
                                                            );
                                                        }
                                                        string(
                                                            7,
                                                            0,
                                                            format!(
                                                                "  Converting images to cbz files: {}",
                                                                format!("{}.cbz", folder_path).as_str()
                                                            ).as_str()
                                                        );
                                                        let _ = zip_func::to_zip(
                                                            folder_path.as_str(),
                                                            format!("{}.cbz", folder_name).as_str()
                                                        ).await;
                                                        let _ =
                                                            tokio::fs::remove_dir_all(
                                                                folder_path
                                                            ).await;
                                                        for i in 3..stdscr().get_max_y() {
                                                            string(
                                                                i as i32,
                                                                0,
                                                                " "
                                                                    .repeat(
                                                                        stdscr().get_max_x() as usize
                                                                    )
                                                                    .as_str()
                                                            );
                                                        }
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
                                                    std::thread::sleep(Duration::from_millis(10));
                                                } else {
                                                    println!("Chapter number found");
                                                }
                                            } else {
                                                println!("Pages count not found");
                                            }
                                        } else {
                                            println!("Language not found in chapter attributes");
                                        }
                                    } else {
                                        println!("attributes not found in chapter");
                                    }
                                }
                            }
                        }
                    } else {
                        println!("JSON does not contain a 'data' array.");
                    }
                }
                _ => {
                    println!("JSON is not an object.");
                }
            }
        Err(err) => println!("Error parsing JSON: {}", err),
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
                                        "{} - {}Ch.{} - {}",
                                        manga_name,
                                        vol,
                                        chapter,
                                        title
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

                                    let start =
                                        stdscr().get_max_x() / 3 - (images_length as i32) / 2;

                                    let tasks = (0..images_length).map(|item| {
                                        if let Some(image_tmp) = images.get(item) {
                                            let image_temp = image_tmp.to_string();

                                            let folder_path = format!(
                                                "{} - {}Ch.{} - {}",
                                                manga_name,
                                                vol,
                                                chapter,
                                                title
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

                                            let chapter_hash = chapter_hash.to_string();
                                            let image = image_temp.trim_matches('"').to_string();
                                            let manga_name = manga_name.to_string();
                                            let title = title.to_string();
                                            let vol = vol.to_string();
                                            let chapter = chapter.to_string();

                                            tokio::spawn(async move {
                                                download_image(
                                                    &chapter_hash,
                                                    &image,
                                                    &manga_name,
                                                    &title,
                                                    &vol,
                                                    &chapter,
                                                    item,
                                                    start
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
                                                    start
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
    start: i32
) {
    let page = page + 1;
    let page_str = page.to_string() + " ".repeat(3 - page.to_string().len()).as_str();
    let base_url = "https://uploads.mangadex.org/data/";
    let full_url = format!("{}{}/{}", base_url, c_hash, f_name);

    let folder_name = format!("{} - {}Ch.{} - {}", manga_name, vol, chapter, name);
    let file_name = format!("{} - {}Ch.{} - {} - {}.jpg", manga_name, vol, chapter, name, page);
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
    string(5 + 1 + (page as i32), 0, "  Sleeping");
    thread::sleep(Duration::from_millis((page * 50) as u64));
    string(
        5 + 1 + (page as i32),
        0,
        format!("   {} {} {}", page_str, "Downloading", file_name_brief).as_str()
    );
    string(5 + 1, -1 + start + (page as i32), "/");
    let full_path = format!("{}/{}", folder_name, file_name);

    // Reqwest setupClient
    let mut response = reqwest::get(full_url.clone()).await.unwrap();
    let total_size = response.content_length().unwrap_or(0);

    // download chunks
    let mut file = File::create(full_path).unwrap();
    let mut downloaded = 0;

    let interval = Duration::from_millis(250);
    let mut last_check_time = Instant::now();
    string(5 + 1, -1 + start + (page as i32), "\\");

    while let Some(chunk) = response.chunk().await.unwrap() {
        let _ = file.write_all(&chunk);
        downloaded += chunk.len() as u64;

        let current_time = Instant::now();
        if current_time.duration_since(last_check_time) >= interval {
            last_check_time = current_time;
            let message = format!(
                "   {} {} {} {}%",
                page_str,
                "Downloading",
                file_name_brief,
                ((100.0 / (total_size as f32)) * (downloaded as f32)).round() as i64
            );
            string(
                5 + 1 + (page as i32),
                0,
                format!(
                    "   {} {}",
                    message,
                    "#".repeat(
                        ((((stdscr().get_max_x() - (message.len() as i32)) as f32) /
                            (total_size as f32)) *
                            (downloaded as f32)) as usize
                    )
                ).as_str()
            );
        }
    }

    let message = format!("   {} {} {} {}%", page_str, "Downloading", file_name_brief, 100);

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
