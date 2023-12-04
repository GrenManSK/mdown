use serde_json::Value;
use std::{ thread::sleep, time::Duration, process::exit };
use crosscurses::*;

use crate::{ ARGS, string, utils::progress_bar_preparation, getter };

pub(crate) fn get_folder_name(manga_name: &str) -> String {
    if ARGS.folder == "name" {
        return manga_name.to_owned();
    } else {
        return ARGS.folder.as_str().to_string();
    }
}

pub(crate) fn get_manga_name(title_data: &Value) -> &str {
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

pub(crate) async fn get_manga_json(id: &str) -> Result<String, reqwest::Error> {
    let base_url = "https://api.mangadex.org/manga/";
    let full_url = format!("{}{}?includes[]=cover_art", base_url, id);

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

pub(crate) async fn get_chapter(id: &str) -> Result<String, reqwest::Error> {
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

pub(crate) async fn get_manga(id: &str, offset: i32) -> Result<(String, usize), reqwest::Error> {
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

pub(crate) fn get_attr_as_str<'a>(obj: &'a Value, attr: &'a str) -> &'a str {
    obj.get(attr)
        .and_then(Value::as_str)
        .unwrap_or_else(|| { "" })
}

pub(crate) fn get_attr_as_u64<'a>(obj: &'a Value, attr: &'a str) -> u64 {
    obj.get(attr)
        .and_then(Value::as_u64)
        .unwrap_or_else(|| { 0 })
}

pub(crate) fn get_attr_as_same<'a>(obj: &'a Value, attr: &'a str) -> &'a Value {
    obj.get(attr).unwrap_or_else(|| {
        eprintln!("{} doesn't exist", attr);
        exit(1);
    })
}

pub(crate) fn get_attr_as_same_as_index(data_array: &Value, item: usize) -> &Value {
    data_array.get(item).unwrap_or_else(move || {
        eprintln!("{} doesn't exist", item);
        exit(1);
    })
}

pub(crate) fn get_attr_as_same_from_vec(data_array: &Vec<Value>, item: usize) -> &Value {
    data_array.get(item).unwrap_or_else(move || {
        eprintln!("{} doesn't exist", item);
        exit(1);
    })
}

pub(crate) fn get_metadata(array_item: &Value) -> (&Value, &str, u64, &str, &str) {
    let chapter_attr = getter::get_attr_as_same(array_item, "attributes");
    let lang = getter::get_attr_as_str(chapter_attr, "translatedLanguage");
    let pages = getter::get_attr_as_u64(chapter_attr, "pages");
    let chapter_num = getter::get_attr_as_str(chapter_attr, "chapter");
    let title = getter::get_attr_as_str(chapter_attr, "title");
    (chapter_attr, lang, pages, chapter_num, title)
}

pub(crate) fn get_arg(arg: String) -> String {
    match arg.as_str() {
        "" => String::from("*"),
        x => String::from(x),
    }
}

pub(crate) fn get_saver() -> String {
    if ARGS.saver {
        return String::from("dataSaver");
    } else {
        return String::from("data");
    }
}
