use serde_json::{ Value, Map };
use std::{ fs::{ self, OpenOptions }, process::exit, io::Write, sync::Mutex };
use lazy_static::lazy_static;

use crate::{
    ARGS,
    MAXPOINTS,
    download_manga,
    string,
    getter::{ get_manga, get_manga_name, get_folder_name, get_scanlation_group },
    download,
    utils::clear_screen,
};

lazy_static! {
    static ref SCANLATION_GROUPS: Mutex<Vec<String>> = Mutex::new(Vec::new());
}

pub(crate) async fn resolve(obj: Map<String, Value>, id: &str) -> String {
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
    if ARGS.lang != orig_lang && !final_lang.contains(&ARGS.lang.as_str()) {
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
        for lang in languages {
            langs.push_str(&format!("{} ", lang));
        }
        string(
            1,
            0,
            &format!(
                "Language is not available\nSelected language: {}\nAvailable languages: {}\nOriginal language: \"{}\"",
                ARGS.lang,
                langs,
                orig_lang
            )
        );
        return manga_name;
    }

    let was_rewritten = fs::metadata(folder.clone()).is_ok();
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
        .open(format!("{}\\{}_description.txt", folder, get_manga_name(title_data)))
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
    download::download_cover(id, cover, &folder).await;

    resolve_manga(id, &manga_name, was_rewritten).await;

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

async fn resolve_manga(id: &str, manga_name: &str, was_rewritten: bool) {
    let going_offset: i32 = ARGS.database_offset.as_str().parse().unwrap();
    let mut arg_force = ARGS.force;
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
    } else {
        if !was_rewritten {
            let _ = fs::remove_dir_all(get_folder_name(manga_name));
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
        string(
            start + i,
            0,
            &format!("{}{}", message, " ".repeat((MAXPOINTS.max_x as usize) - message.len()))
        );
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
