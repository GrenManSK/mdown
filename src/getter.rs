use serde_json::Value;
use std::process::exit;

use crate::{
    args::{ self, ARGS },
    download::get_response_client,
    error::MdownError,
    log,
    metadata,
    resolute,
    string,
    utils,
};

fn get_exe_path() -> Result<String, MdownError> {
    let current = match std::env::current_exe() {
        Ok(value) => value,
        Err(err) => {
            return Err(
                MdownError::IoError(
                    err,
                    String::from("? your path to your exe file is invalid bro")
                )
            );
        }
    };
    let parent = match current.parent() {
        Some(value) => value,
        None => {
            return Err(MdownError::NotFoundError(String::from("Parent not found")));
        }
    };
    let path = match parent.to_str() {
        Some(value) => value.to_string(),
        None => {
            return Err(MdownError::ConversionError(String::from("Transition to str failed")));
        }
    };
    Ok(path)
}

pub(crate) fn get_dat_path() -> Result<String, MdownError> {
    let path = match get_exe_path() {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };
    Ok(format!("{}\\dat.json", path))
}
pub(crate) fn get_db_path() -> Result<String, MdownError> {
    let path = match get_exe_path() {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };
    Ok(format!("{}\\resources.db", path))
}
pub(crate) fn get_log_path() -> Result<String, MdownError> {
    let path: String = match get_exe_path() {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };
    Ok(format!("{}\\log.json", path))
}
pub(crate) fn get_log_lock_path() -> Result<String, MdownError> {
    let path: String = match get_exe_path() {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };
    Ok(format!("{}\\log.lock", path))
}

#[cfg(any(feature = "server", feature = "web"))]
pub(crate) fn get_query(parts: Vec<&str>) -> std::collections::HashMap<String, String> {
    (
        match parts[1].split('?').nth(1) {
            Some(value) => value,
            None => "",
        }
    )
        .split('&')
        .filter_map(|param| {
            let mut iter = param.split('=');
            let key = match iter.next() {
                Some(key) => key.to_string(),
                None => String::from(""),
            };
            let value = match iter.next() {
                Some(key) => key.to_string(),
                None => String::from(""),
            };
            Some((key, value))
        })
        .collect()
}

pub(crate) fn get_folder_name<'a>(manga_name: &'a str) -> &'a str {
    let folder_name = ARGS.lock().folder.clone();
    if folder_name == "name" {
        manga_name
    } else {
        Box::leak(folder_name.into_boxed_str())
    }
}

pub(crate) fn get_manga_name(title_data: &Value) -> String {
    let lang = resolute::LANGUAGE.lock().clone();
    let name = (
        match
            title_data
                .get("title")
                .and_then(|attr_data| attr_data.get(lang.clone()))
                .and_then(Value::as_str)
        {
            Some(manga_name) => {
                drop(lang);
                manga_name.to_string()
            }
            None => {
                drop(lang);
                let mut return_title = String::from("*");
                let get = title_data.get("altTitles").and_then(|val| val.as_array());
                if let Some(get) = get {
                    for title_object in get {
                        if let Some(lang_object) = title_object.as_object() {
                            for (lang, title) in lang_object.iter() {
                                if lang == "en" {
                                    return_title = match title.as_str() {
                                        Some(s) => s.to_string(),
                                        None => String::new(),
                                    };
                                    break;
                                }
                            }
                        }
                        break;
                    }
                    if return_title == "*" {
                        return_title = match
                            title_data
                                .get("title")
                                .and_then(|attr_data| attr_data.get("ja-ro"))
                                .and_then(Value::as_str)
                        {
                            Some(value) => value.to_string(),
                            None => String::from("*"),
                        };
                    } else {
                        return return_title.replace("\"", "");
                    }
                    let get = title_data.get("altTitles").and_then(|val| val.as_array());

                    if let Some(get) = get {
                        let mut get_final: serde_json::Map<String, Value> = serde_json::Map::new();

                        for obj in get {
                            if let Value::Object(inner_map) = obj {
                                for (key, value) in inner_map {
                                    get_final.insert(key.to_string(), value.clone());
                                }
                            }
                        }
                        for (lang, title) in &get_final {
                            if *lang == *resolute::LANGUAGE.lock() {
                                return_title = title.to_string();
                                break;
                            }
                        }
                        if return_title == String::from("*") {
                            for (lang, title) in get_final {
                                if lang == "en" {
                                    return_title = title.to_string();
                                    break;
                                }
                            }
                        }
                    }
                }
                if return_title == String::from("*") {
                    match
                        title_data
                            .get("title")
                            .and_then(|attr_data| attr_data.get("en"))
                            .and_then(Value::as_str)
                    {
                        Some(manga_name) => manga_name.to_string(),
                        None => {
                            match
                                title_data
                                    .get("title")
                                    .and_then(|attr_data| attr_data.get("ja-ro"))
                                    .and_then(Value::as_str)
                            {
                                Some(manga_name) => manga_name.to_string(),
                                None => String::from("Unrecognized title"),
                            }
                        }
                    }
                } else {
                    return_title
                }
            }
        }
    )
        .replace("\"", "")
        .replace("?", "")
        .trim()
        .to_string();
    if name.len() > 70 {
        return format!("{}__", &name[0..70]);
    } else {
        name
    }
}

pub(crate) async fn get_manga_json(id: &str) -> Result<String, MdownError> {
    let full_url = format!("https://api.mangadex.org/manga/{}?includes[]=cover_art", id);

    let response = match get_response_client(&full_url).await {
        Ok(res) => res,
        Err(err) => {
            return Err(err);
        }
    };

    if response.status().is_success() {
        return match response.text().await {
            Ok(text) => Ok(text),
            Err(err) =>
                Err(
                    MdownError::StatusError(match err.status() {
                        Some(status) => status,
                        None => {
                            return Err(
                                MdownError::NotFoundError(
                                    String::from("StatusCode (get_manga_json)")
                                )
                            );
                        }
                    })
                ),
        };
    } else {
        eprintln!(
            "Error: get manga json Failed to fetch data from the API. Status code: {:?}",
            response.status()
        );
        Err(MdownError::StatusError(response.status()))
    }
}

pub(crate) async fn get_statistic_json(id: &str) -> Result<String, MdownError> {
    let full_url = format!("https://api.mangadex.org/statistics/manga/{}", id);

    let response = match get_response_client(&full_url).await {
        Ok(res) => res,
        Err(err) => {
            return Err(err);
        }
    };
    if response.status().is_success() {
        let json = match response.text().await {
            Ok(res) => res,
            Err(err) => {
                return Err(MdownError::JsonError(err.to_string()));
            }
        };

        Ok(json)
    } else {
        eprintln!(
            "Error: get statistic json Failed to fetch data from the API. Status code: {:?}",
            response.status()
        );
        Err(MdownError::StatusError(response.status()))
    }
}

pub(crate) async fn get_chapter(id: &str) -> Result<String, MdownError> {
    loop {
        string(3, 0, "Retrieving chapter info");

        let base_url = "https://api.mangadex.org/at-home/server/";
        let full_url = format!("{}{}", base_url, id);

        let response = match get_response_client(&full_url).await {
            Ok(res) => res,
            Err(err) => {
                return Err(err);
            }
        };
        if response.status().is_success() {
            let json = match response.text().await {
                Ok(text) => text,
                Err(err) => {
                    return Err(
                        MdownError::StatusError(match err.status() {
                            Some(status) => status,
                            None => {
                                return Err(
                                    MdownError::NotFoundError(
                                        String::from("StatusCode (get_chapter)")
                                    )
                                );
                            }
                        })
                    );
                }
            };

            string(3, 0, "Retrieving chapter info DONE");
            return Ok(json);
        } else {
            string(
                5,
                0,
                &format!(
                    "get chapter Failed to fetch data from the API. Status code: {:?} {}",
                    response.status(),
                    match response.text().await {
                        Ok(text) => text,
                        Err(err) => {
                            return Err(
                                MdownError::StatusError(match err.status() {
                                    Some(status) => status,
                                    None => {
                                        return Err(
                                            MdownError::NotFoundError(
                                                String::from("StatusCode (get_chapter)")
                                            )
                                        );
                                    }
                                })
                            );
                        }
                    }
                )
            );
        }
    }
}

pub(crate) fn get_scanlation_group(json: &Vec<metadata::ChapterRelResponse>) -> Option<String> {
    for relation in json {
        match relation.r#type.as_str() {
            "scanlation_group" => {
                return Some(relation.id.clone());
            }
            _ => {
                continue;
            }
        }
    }
    None
}

pub(crate) async fn get_manga(id: &str, offset: u32) -> Result<(String, usize), MdownError> {
    let mut times = 0;
    let mut json;
    let mut json_2 = String::new();
    let mut times_offset: u32;
    let stat = match ARGS.lock().stat {
        true => 1,
        false => 0,
    };
    loop {
        times_offset = offset + 500 * times;
        string(
            3 + times + stat,
            0,
            &format!(
                "{} {} {}   ",
                times.to_string(),
                "Fetching data with offset",
                times_offset.to_string()
            )
        );
        let full_url = format!(
            "https://api.mangadex.org/manga/{}/feed?limit=500&offset={}",
            id,
            times_offset
        );

        let response = match get_response_client(&full_url).await {
            Ok(res) => res,
            Err(err) => {
                return Err(err);
            }
        };
        if response.status().is_success() {
            json = match response.text().await {
                Ok(text) => text,
                Err(err) => {
                    return Err(
                        MdownError::StatusError(match err.status() {
                            Some(status) => status,
                            None => {
                                return Err(
                                    MdownError::NotFoundError(
                                        String::from("StatusCode (get_manga)")
                                    )
                                );
                            }
                        })
                    );
                }
            };
            if times == 0 {
                json_2 = json.clone();
            }
            let mut offset_temp: usize = 0;
            let json_value = match utils::get_json(&json) {
                Ok(value) => value,
                Err(err) => {
                    return Err(err);
                }
            };
            match json_value {
                Value::Object(obj) => {
                    if let Some(data_array) = obj.get("data").and_then(Value::as_array) {
                        let naive_time_str = chrono::Utc
                            ::now()
                            .naive_utc()
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string();

                        resolute::DATE_FETCHED.lock().push(naive_time_str);
                        let message = format!(
                            "{} Data fetched with offset {}   ",
                            times.to_string(),
                            offset.to_string()
                        );
                        string(3 + times + stat, 0, &message);
                        if
                            *args::ARGS_WEB ||
                            *args::ARGS_GUI ||
                            *args::ARGS_CHECK ||
                            *args::ARGS_UPDATE ||
                            *args::ARGS_LOG
                        {
                            log!(&message);
                        }
                        offset_temp = data_array.len();
                        if offset_temp >= 500 {
                            if times > 0 {
                                let mut data1 = match utils::get_json(&json) {
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(err);
                                    }
                                };
                                let data2 = match utils::get_json(&json_2) {
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(err);
                                    }
                                };

                                let data1_array = match data1.get_mut("data") {
                                    Some(value) => value,
                                    None => {
                                        return Err(
                                            MdownError::JsonError(String::from("Didn't found data"))
                                        );
                                    }
                                };
                                let data2_array = match data2.get("data") {
                                    Some(value) => value,
                                    None => {
                                        return Err(
                                            MdownError::JsonError(String::from("Didn't found data"))
                                        );
                                    }
                                };
                                let empty_array = vec![];

                                if let Some(data1_array) = data1_array.as_array_mut() {
                                    data1_array.extend(
                                        (
                                            match data2_array.as_array() {
                                                Some(array) => array,
                                                None => &empty_array,
                                            }
                                        ).clone()
                                    );
                                }

                                json = match serde_json::to_string(&data1) {
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(MdownError::JsonError(err.to_string()));
                                    }
                                };
                            }
                            json_2 = json;
                            times += 1;
                            continue;
                        } else {
                            offset_temp = data_array.len();
                        }
                        if times > 0 {
                            let mut data1 = match utils::get_json(&json) {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(err);
                                }
                            };
                            let data2 = match utils::get_json(&json_2) {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(err);
                                }
                            };

                            let data1_array = match data1.get_mut("data") {
                                Some(value) => value,
                                None => {
                                    return Err(
                                        MdownError::JsonError(String::from("Did not find data"))
                                    );
                                }
                            };
                            let data2_array = match data2.get("data") {
                                Some(value) => value,
                                None => {
                                    return Err(
                                        MdownError::JsonError(String::from("Did not find data"))
                                    );
                                }
                            };

                            let empty_array = vec![];
                            if let Some(data1_array) = data1_array.as_array_mut() {
                                data1_array.extend(
                                    (
                                        match data2_array.as_array() {
                                            Some(array) => array,
                                            None => &empty_array,
                                        }
                                    ).clone()
                                );
                            }

                            json = match serde_json::to_string(&data1) {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(MdownError::JsonError(err.to_string()));
                                }
                            };
                        }
                    }
                }
                _ => {
                    return Err(MdownError::JsonError(String::from("Could not parse manga json")));
                }
            }

            return Ok((json, offset_temp));
        } else {
            eprintln!(
                "Error: get manga Failed to fetch data from the API. Status code: {:?} ({})",
                response.status(),
                full_url
            );
            exit(1);
        }
    }
}

pub(crate) fn get_attr_as_same_as_index(data_array: &Vec<String>, item: usize) -> &String {
    match data_array.get(item) {
        Some(value) => value,
        None => {
            eprintln!("{}", MdownError::NotFoundError(String::from("get_attr_as_same_as_index")));
            exit(1);
        }
    }
}

pub(crate) fn get_attr_as_same_from_vec(
    data_array: &Vec<metadata::ChapterResponse>,
    item: usize
) -> &metadata::ChapterResponse {
    match data_array.get(item) {
        Some(value) => value,
        None => {
            eprintln!("{}", MdownError::NotFoundError(String::from("get_attr_as_same_from_vec")));
            exit(1);
        }
    }
}

pub(crate) fn get_metadata(
    array_item: &metadata::ChapterResponse
) -> (metadata::ChapterAttrResponse, String, u64, String, String) {
    let chapter_attr = array_item.attributes.clone();
    let lang = match chapter_attr.translatedLanguage.clone() {
        Some(value) => value,
        None => String::new(),
    };
    let pages = chapter_attr.pages.clone();
    let chapter_num = match chapter_attr.chapter.clone() {
        Some(value) => value,
        None => String::new(),
    };
    let title = match chapter_attr.title.clone() {
        Some(value) => value,
        None => String::new(),
    };
    (chapter_attr, lang, pages, chapter_num, title)
}

pub(crate) fn get_arg(arg: &str) -> &str {
    match arg {
        "" => "*",
        x => x,
    }
}

// returns english title if exists in title_data
#[test]
fn test_get_manga_name_returns_english_title_if_exists() {
    let title_data =
        serde_json::json!({
        "title": {
            "en": "English Title"
        }
    });

    let result = get_manga_name(&title_data);

    assert_eq!(result, "English Title");
}

// returns english title if exists in alt_titles with english language
#[test]
fn test_get_manga_name_returns_english_title_if_exists_in_alt_titles() {
    let title_data =
        serde_json::json!({
        "altTitles": [
            {
                "en": "English Title"
            }
        ]
    });

    let result = get_manga_name(&title_data);

    assert_eq!(result, "English Title");
}

// returns japanese romanized title if english title not found
#[test]
fn test_get_manga_name_returns_japanese_romanized_title_if_english_title_not_found() {
    let title_data =
        serde_json::json!({
        "title": {
            "ja-ro": "Japanese Romanized Title"
        }
    });

    let result = get_manga_name(&title_data);

    assert_eq!(result, "Japanese Romanized Title");
}

// returns first english title found in alt_titles with multiple languages
#[test]
fn test_get_manga_name_returns_first_english_title_found_in_alt_titles() {
    let title_data =
        serde_json::json!({
        "altTitles": [
            {
                "en": "English Title"
            },
            {
                "fr": "French Title"
            }
        ]
    });

    let result = get_manga_name(&title_data);

    assert_eq!(result, "English Title");
}

// returns empty string if title in alt_titles but no english language available
#[test]
fn test_get_manga_name_returns_empty_string_if_title_in_alt_titles_but_no_english_language_available() {
    let title_data =
        serde_json::json!({
        "altTitles": [
            {
                "fr": "French Title"
            }
        ]
    });

    let result = get_manga_name(&title_data);

    assert_eq!(result, "Unrecognized title");
}

// returns empty string if title in alt_titles but no language available
#[test]
fn test_get_manga_name_returns_empty_string_if_title_in_alt_titles_but_no_language_available() {
    let title_data = serde_json::json!({
        "altTitles": [
            {}
        ]
    });

    let result = get_manga_name(&title_data);

    assert_eq!(result, "Unrecognized title");
}
