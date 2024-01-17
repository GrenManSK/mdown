use serde_json::{ Value, Map, json };
use std::{ fs::{ self, File, OpenOptions }, process::exit, io::{ Write, Read }, sync::Mutex };
use lazy_static::lazy_static;
use tracing::info;

use crate::{
    ARGS,
    MAXPOINTS,
    download_manga,
    string,
    getter::{ self, get_manga, get_manga_name, get_folder_name, get_scanlation_group },
    download,
    utils::clear_screen,
    MANGA_ID,
};

lazy_static! {
    pub(crate) static ref SCANLATION_GROUPS: Mutex<Vec<String>> = Mutex::new(Vec::new());
    pub(crate) static ref DOWNLOADED: Mutex<Vec<String>> = Mutex::new(Vec::new());
    pub(crate) static ref MANGA_NAME: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref CHAPTERS: Mutex<Vec<String>> = Mutex::new(Vec::new());
    pub(crate) static ref MWD: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref TO_DOWNLOAD: Mutex<Vec<String>> = Mutex::new(Vec::new());
    pub(crate) static ref CURRENT_CHAPTER: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref CURRENT_PAGE: Mutex<u64> = Mutex::new(0);
    pub(crate) static ref CURRENT_PAGE_MAX: Mutex<u64> = Mutex::new(0);
    pub(crate) static ref CURRENT_PERCENT: Mutex<f64> = Mutex::new(0.0);
    pub(crate) static ref CURRENT_SIZE: Mutex<f64> = Mutex::new(0.0);
    pub(crate) static ref CURRENT_SIZE_MAX: Mutex<f64> = Mutex::new(0.0);
}

pub(crate) async fn resolve_check() {
    if fs::metadata(getter::get_dat_path()).is_err() {
        return;
    }
    let result = match get_dat_content() {
        Some(value) => value,
        None => {
            return;
        }
    };
    match result {
        Ok(mut json) => {
            if let Some(data) = json.get_mut("data").and_then(Value::as_array_mut) {
                let mut iter: i32 = -1;
                let mut to_remove = vec![];
                for item in data.iter_mut() {
                    iter += 1;
                    println!("Checking {}\r", item.get("name").and_then(Value::as_str).unwrap());
                    let past_mwd = std::env::current_dir().unwrap().to_str().unwrap().to_string();
                    let mwd = item.get("mwd").and_then(Value::as_str).unwrap();
                    if let Err(_) = std::env::set_current_dir(mwd) {
                        to_remove.push(iter);
                        continue;
                    }

                    let _ = std::fs::rename(
                        format!("{past_mwd}\\.cache"),
                        format!("{mwd}\\.cache")
                    );
                    let id = item.get("id").and_then(Value::as_str).unwrap();
                    match getter::get_manga_json(id).await {
                        Ok(manga_name_json) => {
                            let json_value = serde_json::from_str(&manga_name_json).unwrap();
                            if let Value::Object(obj) = json_value {
                                let title_data = obj
                                    .get("data")
                                    .and_then(|name_data| name_data.get("attributes"))
                                    .unwrap_or_else(|| {
                                        eprintln!("attributes or title doesn't exist");
                                        exit(1);
                                    });
                                *MANGA_NAME.lock().unwrap() = get_manga_name(title_data);
                                resolve_manga(
                                    id,
                                    get_manga_name(title_data).as_str(),
                                    false,
                                    Some(String::new())
                                ).await;
                            } else {
                                eprintln!("Unexpected JSON value");
                                return;
                            }
                        }
                        Err(_) => {}
                    }
                    if let Some(chapters) = item.get_mut("chapters").and_then(Value::as_array_mut) {
                        chapters.clear();
                        chapters.extend(
                            CHAPTERS.lock().unwrap().iter().cloned().map(Value::String)
                        );
                    }

                    if ARGS.check {
                        println!("Checked {}", MANGA_NAME.lock().unwrap());
                        if !TO_DOWNLOAD.lock().unwrap().is_empty() {
                            println!("Chapters available");
                            for chapter in TO_DOWNLOAD.lock().unwrap().iter() {
                                println!(" {}", chapter);
                            }
                        } else {
                            println!("Up to-date");
                        }
                    }
                    CHAPTERS.lock().unwrap().clear();
                }
                for &index in to_remove.iter().rev() {
                    data.remove(index as usize);
                }
                let mut file = File::create(getter::get_dat_path()).unwrap();

                let json_string = serde_json::to_string_pretty(&json);
                if let Err(err) = json_string {
                    eprintln!("Error serializing to JSON: {:?}", err);
                    return;
                }

                if let Err(err) = writeln!(file, "{}", json_string.unwrap()) {
                    eprintln!("Error writing to file: {:?}", err);
                    return;
                }
            }
        }
        Err(err) => {
            eprintln!("Error parsing JSON: {:?}", err);
        }
    }
}

pub(crate) fn resolve_dat() {
    if fs::metadata(getter::get_dat_path()).is_err() {
        let mut file = fs::File::create(getter::get_dat_path()).unwrap();

        let content = "{\n  \"data\": []\n}";

        file.write_all(content.as_bytes()).unwrap();
    }
    let result = match get_dat_content() {
        Some(value) => value,
        None => {
            return;
        }
    };
    match result {
        Ok(mut json) => {
            if let Some(data) = json.get_mut("data").and_then(Value::as_array_mut) {
                let manga_names: Vec<&str> = data
                    .iter()
                    .filter_map(|item| item.get("name").and_then(Value::as_str))
                    .collect();

                if manga_names.contains(&MANGA_NAME.lock().unwrap().as_str()) {
                    for item in data.iter_mut() {
                        if let Some(name) = item.get("name").and_then(Value::as_str) {
                            if name == MANGA_NAME.lock().unwrap().as_str() {
                                let existing_chapters = item
                                    .get_mut("chapters")
                                    .and_then(Value::as_array_mut)
                                    .unwrap();

                                let mut new_chapters: Vec<_> = CHAPTERS.lock()
                                    .unwrap()
                                    .iter()
                                    .cloned()
                                    .map(Value::String)
                                    .filter(|chapter| !existing_chapters.contains(chapter))
                                    .collect();

                                new_chapters.sort_by(|a, b| {
                                    let a_num = a.as_str().unwrap().parse::<u32>().unwrap_or(0);
                                    let b_num = b.as_str().unwrap().parse::<u32>().unwrap_or(0);
                                    a_num.cmp(&b_num)
                                });

                                existing_chapters.extend(new_chapters);

                                break;
                            }
                        }
                    }
                } else {
                    let mwd = format!("{}", MWD.lock().unwrap());
                    let manga_data =
                        json!({
                    "name": MANGA_NAME.lock().unwrap().clone(),
                    "id": MANGA_ID.lock().unwrap().to_string(),
                    "chapters": CHAPTERS.lock().unwrap().clone(),
                    "mwd": mwd,
                    });

                    data.push(manga_data.clone());
                }

                let mut file = File::create(getter::get_dat_path()).unwrap();

                let json_string = serde_json::to_string_pretty(&json);
                if let Err(err) = json_string {
                    eprintln!("Error serializing to JSON: {:?}", err);
                    return;
                }

                if let Err(err) = writeln!(file, "{}", json_string.unwrap()) {
                    eprintln!("Error writing to file: {:?}", err);
                    return;
                }
            }
        }
        Err(err) => {
            eprintln!("Error parsing JSON: {:?}", err);
        }
    }
}

fn get_dat_content() -> Option<Result<Value, serde_json::Error>> {
    let file = File::open(getter::get_dat_path());
    if let Err(err) = file {
        eprintln!("Error opening file: {:?}", err);
        return None;
    }
    let mut file = file.unwrap();
    let mut contents = String::new();
    if let Err(err) = file.read_to_string(&mut contents) {
        eprintln!("Error reading file: {:?}", err);
        return None;
    }
    let result: Result<Value, _> = serde_json::from_str(&contents);
    Some(result)
}

pub(crate) async fn resolve(
    obj: Map<String, Value>,
    id: &str,
    handle_id: Option<String>
) -> String {
    let title_data = obj
        .get("data")
        .and_then(|name_data| name_data.get("attributes"))
        .unwrap_or_else(|| {
            eprintln!("attributes or title doesn't exist");
            exit(1);
        });

    let manga_name = if ARGS.title == "*" {
        get_manga_name(title_data)
    } else {
        ARGS.title.to_string()
    };
    *MANGA_NAME.lock().unwrap() = manga_name.clone();
    let folder = get_folder_name(&manga_name);

    let orig_lang = title_data.get("originalLanguage").and_then(Value::as_str).unwrap();
    let languages = title_data
        .get("availableTranslatedLanguages")
        .and_then(Value::as_array)
        .unwrap();
    let mut final_lang = vec![];
    for lang in languages {
        final_lang.push(lang.as_str().unwrap());
    }
    if ARGS.lang != orig_lang && !final_lang.contains(&ARGS.lang.as_str()) && ARGS.lang != "*" {
        let languages = title_data
            .get("availableTranslatedLanguages")
            .and_then(Value::as_array)
            .unwrap();
        let mut final_lang = vec![];
        for lang in languages {
            final_lang.push(lang.as_str().unwrap());
        }
        let orig_lang = title_data.get("originalLanguage").and_then(Value::as_str).unwrap();
        let mut langs = String::new();
        let mut lang_range: usize = 0;
        for lang in languages {
            langs.push_str(&format!("{} ", lang));
            lang_range += 1 + lang.to_string().len();
        }
        lang_range -= 1;
        string(
            1,
            0,
            &format!(
                "Language is not available\nSelected language: {}\nOriginal language: {}\nAvailable languages: {}\nChoose from these    {}",
                ARGS.lang,
                orig_lang,
                langs,
                "^".repeat(lang_range)
            )
        );
        return manga_name;
    }

    let was_rewritten = fs::metadata(folder.clone()).is_ok();
    let _ = fs::create_dir(&folder);
    *MWD.lock().unwrap() = std::fs::canonicalize(&folder).unwrap().to_str().unwrap().to_string();
    println!("{}", MWD.lock().unwrap());
    let desc = title_data
        .get("description")
        .and_then(|description| description.get("en"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut desc_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(format!("{}\\_description.txt", folder))
        .unwrap();
    let _ = write!(desc_file, "{}", desc);

    let folder = get_folder_name(&manga_name);
    let cover = obj
        .get("data")
        .and_then(|name_data| name_data.get("relationships"))
        .and_then(Value::as_array)
        .and_then(|data| {
            let mut cover = "";
            for el in data {
                if el.get("type").unwrap() == "cover_art" {
                    cover = el
                        .get("attributes")
                        .and_then(|dat| dat.get("fileName"))
                        .and_then(Value::as_str)
                        .unwrap();
                }
            }
            Option::Some(cover)
        })
        .unwrap();
    download::download_cover(id, cover, &folder, handle_id.clone()).await;

    if ARGS.stat {
        download::download_stat(id, &folder, &manga_name, handle_id.clone()).await;
    }

    resolve_manga(id, &manga_name, was_rewritten, handle_id.clone()).await;

    if ARGS.web || ARGS.check || ARGS.update {
        info!("@{} Downloaded manga", handle_id.unwrap_or_default());
    }
    manga_name
}

pub(crate) async fn resolve_group(array_item: &Value, manga_name: &str) {
    let scanlation_group = array_item.get("relationships").and_then(Value::as_array).unwrap();
    let scanlation_group_id = get_scanlation_group(scanlation_group).unwrap_or_default();
    if scanlation_group_id.is_empty() {
        return;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{}\\_scanlation_groups.txt", get_folder_name(manga_name)))
        .unwrap();
    let (name, website) = resolve_group_metadata(scanlation_group_id).await.unwrap();
    if name != "Unknown" && !SCANLATION_GROUPS.lock().unwrap().contains(&name) {
        SCANLATION_GROUPS.lock().unwrap().push(name.clone());

        let _ = file.write_all(format!("{} - {}\n", name, website).as_bytes());
    }
}

pub(crate) async fn resolve_group_metadata(id: &str) -> Option<(String, String)> {
    let base_url = "https://api.mangadex.org/group/";
    let full_url = format!("{}\\{}", base_url, id);

    let response = download::get_response_client(full_url).await.unwrap();

    if response.status().is_success() {
        let json = response.text().await.unwrap();

        match serde_json::from_str(&json) {
            Ok(json_value) =>
                match json_value {
                    Value::Object(obj) => {
                        let attr = obj.get("data").unwrap().get("attributes").unwrap();
                        let name = attr.get("name").and_then(Value::as_str).unwrap().to_owned();
                        let website = attr
                            .get("website")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned();
                        return Some((name, website));
                    }
                    _ => todo!(),
                }
            Err(err) => {
                eprintln!("Error parsing JSON: {}", err);
                return None;
            }
        };
    } else {
        eprintln!(
            "Error: {}",
            format!("Failed to fetch data from the API. Status code: {:?}", response.status())
        );
        exit(1);
    }
}

async fn resolve_manga(id: &str, manga_name: &str, was_rewritten: bool, handle_id: Option<String>) {
    let going_offset: i32 = ARGS.database_offset.as_str().parse().unwrap();
    let mut arg_force = ARGS.force;
    let end = if ARGS.check || ARGS.update { 1 } else { 2 };
    let mut downloaded: Vec<String> = vec![];
    for _ in 0..end {
        match get_manga(id, going_offset, handle_id.clone()).await {
            Ok((json, _offset)) => {
                let downloaded_temp = download_manga(
                    json,
                    manga_name,
                    arg_force,
                    handle_id.clone()
                ).await;
                for i in 0..downloaded_temp.len() {
                    downloaded.push(downloaded_temp[i].clone());
                }
                clear_screen(1);
            }
            Err(err) => eprintln!("Error: {}", err),
        }
        arg_force = false;
    }
    if !ARGS.web || ARGS.check || ARGS.update {
        if downloaded.len() != 0 {
            string(1, 0, "Downloaded files:");
            for i in 0..downloaded.len() {
                (_, downloaded) = resolve_move(i as i32, downloaded.clone(), 2, 1);
            }
        } else {
            if !was_rewritten {
                let _ = fs::remove_dir_all(get_folder_name(manga_name));
            }
        }
    }
}

pub(crate) fn resolve_move(
    mut moves: i32,
    mut hist: Vec<String>,
    start: i32,
    end: i32
) -> (i32, Vec<String>) {
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
            string(start + i, 0, &format!("{}", message));
        }
    }
    (moves, hist)
}

pub(crate) fn title(mut title: &str) -> &str {
    if title.chars().last().unwrap_or_default() == '.' {
        title = &title[..title.len() - 1];
    }
    title
}

pub(crate) fn resolve_skip(arg: String, with: &str) -> bool {
    if arg == "*" || arg == with {
        return false;
    }
    true
}
