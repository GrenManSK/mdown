use serde_json::Value;
use std::process::exit;

use crate::{
    args::{ self, ARGS },
    download::get_response_client,
    error::MdownError,
    getter,
    log,
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

pub(crate) fn get_scanlation_group(json: &Vec<Value>) -> Option<&str> {
    for relation in json {
        if let Some(relation_type) = relation.get("type") {
            if relation_type == "scanlation_group" {
                return relation.get("id").and_then(Value::as_str);
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

pub(crate) fn get_attr_as_str<'a>(obj: &'a Value, attr: &'a str) -> &'a str {
    match obj.get(attr).and_then(Value::as_str) {
        Some(value) => value,
        None => "",
    }
}

pub(crate) fn get_attr_as_u64<'a>(obj: &'a Value, attr: &'a str) -> u64 {
    match obj.get(attr).and_then(Value::as_u64) {
        Some(value) => value,
        None => 0,
    }
}

pub(crate) fn get_attr_as_same<'a>(obj: &'a Value, attr: &'a str) -> &'a Value {
    match obj.get(attr) {
        Some(value) => value,
        None => {
            eprintln!("{}", MdownError::NotFoundError(String::from("get_attr_as_same")));
            exit(1);
        }
    }
}

pub(crate) fn get_attr_as_same_as_index(data_array: &Value, item: usize) -> &Value {
    match data_array.get(item) {
        Some(value) => value,
        None => {
            eprintln!("{}", MdownError::NotFoundError(String::from("get_attr_as_same_as_index")));
            exit(1);
        }
    }
}

pub(crate) fn get_attr_as_same_from_vec(data_array: &Vec<Value>, item: usize) -> &Value {
    match data_array.get(item) {
        Some(value) => value,
        None => {
            eprintln!("{}", MdownError::NotFoundError(String::from("get_attr_as_same_from_vec")));
            exit(1);
        }
    }
}

pub(crate) fn get_metadata(array_item: &Value) -> (&Value, &str, u64, &str, &str) {
    let chapter_attr = getter::get_attr_as_same(array_item, "attributes");
    let lang = getter::get_attr_as_str(chapter_attr, "translatedLanguage");
    let pages = getter::get_attr_as_u64(chapter_attr, "pages");
    let chapter_num = getter::get_attr_as_str(chapter_attr, "chapter");
    let title = getter::get_attr_as_str(chapter_attr, "title");
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

// Returns a tuple with five elements when given a valid Value object.
#[test]
fn test_get_metadata_should_return_tuple_with_five_elements() {
    // Arrange
    let array_item = Value::from(
        serde_json::json!({
        "attributes": {
            "translatedLanguage": "English",
            "pages": 10,
            "chapter": "Chapter 1",
            "title": "Title"
        }
    })
    );

    // Act
    let result = get_metadata(&array_item);

    // Assert
    assert_eq!(result.1, "English");
    assert_eq!(result.2, 10);
    assert_eq!(result.3, "Chapter 1");
    assert_eq!(result.4, "Title");
}

// Returns an empty string for the language when the 'translatedLanguage' attribute is missing.
#[test]
fn test_get_metadata_should_return_empty_string_for_language_when_translated_language_attribute_is_missing() {
    // Arrange
    let array_item = Value::from(
        serde_json::json!({
        "attributes": {
            "pages": 10,
            "chapter": "Chapter 1",
            "title": "Title"
        }
    })
    );

    // Act
    let result = get_metadata(&array_item);

    // Assert
    assert_eq!(result.1, "");
}

// Returns 0 for the number of pages when the 'pages' attribute is missing.
#[test]
fn test_get_metadata_should_return_zero_for_number_of_pages_when_pages_attribute_is_missing() {
    // Arrange
    let array_item = Value::from(
        serde_json::json!({
        "attributes": {
            "translatedLanguage": "English",
            "chapter": "Chapter 1",
            "title": "Title"
        }
    })
    );

    // Act
    let result = get_metadata(&array_item);

    // Assert
    assert_eq!(result.2, 0);
}

// Returns an empty string for the chapter number when the 'chapter' attribute is missing.
#[test]
fn test_get_metadata_should_return_empty_string_for_chapter_number_when_chapter_attribute_is_missing() {
    // Arrange
    let array_item = Value::from(
        serde_json::json!({
        "attributes": {
            "translatedLanguage": "English",
            "pages": 10,
            "title": "Title"
        }
    })
    );

    // Act
    let result = get_metadata(&array_item);

    // Assert
    assert_eq!(result.3, "");
}

// returns the value of the attribute as a string if it exists in the given JSON object
#[test]
fn test_get_attr_as_str_returns_value_if_attribute_exists() {
    let json = serde_json::json!({
        "attr": "value"
    });
    let result = get_attr_as_str(&json, "attr");
    assert_eq!(result, "value");
}

// returns an empty string if the attribute exists in the given JSON object but its value is not a string
#[test]
fn test_get_attr_as_str_returns_empty_string_if_attribute_value_string() {
    let json = serde_json::json!({
        "attr": 123
    });
    let result = get_attr_as_str(&json, "attr");
    assert_eq!(result, "");
}

// Returns the u64 value of the given attribute if it exists in the given JSON object
#[test]
fn test_get_attr_as_u64_returns_u64_value_if_attribute_exists() {
    let json = serde_json::json!({
        "attr": 10
    });
    let obj = &json;
    let attr = "attr";

    let result = get_attr_as_u64(obj, attr);

    assert_eq!(result, 10);
}

// Returns 0 if the given JSON object is null
#[test]
fn test_get_attr_as_u64_returns_0_if_json_object_null() {
    let obj = serde_json::Value::Null;
    let attr = "attr";

    let result = get_attr_as_u64(&obj, attr);

    assert_eq!(result, 0);
}

// Returns 0 if the given attribute is null
#[test]
fn test_get_attr_as_u64_returns_0_if_attribute_null() {
    let json = serde_json::json!({
        "attr": serde_json::Value::Null
    });
    let obj = &json;
    let attr = "attr";

    let result = get_attr_as_u64(obj, attr);

    assert_eq!(result, 0);
}

// Returns the value at the specified index in the given JSON array.
#[test]
fn test_get_attr_as_same_as_index_returns_value_at_specified_index() {
    let data_array = serde_json::json!([1, 2, 3, 4, 5]);

    let result = get_attr_as_same_as_index(&data_array, 2);

    assert_eq!(result, &serde_json::json!(3));
}

// Returns the first value if the index is 0.
#[test]
fn test_get_attr_as_same_as_index_returns_first_value_if_index_is_zero() {
    let data_array = serde_json::json!([1, 2, 3, 4, 5]);

    let result = get_attr_as_same_as_index(&data_array, 0);

    assert_eq!(result, &serde_json::json!(1));
}

// Returns the last value if the index is the last index of the array.
#[test]
fn test_get_attr_as_same_as_index_returns_last_value_if_index_is_last_index() {
    let data_array = serde_json::json!([1, 2, 3, 4, 5]);

    let result = get_attr_as_same_as_index(&data_array, 4);

    assert_eq!(result, &serde_json::json!(5));
}

// Returns the scanlation group ID if it exists in the JSON array
#[test]
fn test_get_scanlation_group_returns_scanlation_group_id_if_exists() {
    let json = vec![
        serde_json::json!({
            "type": "scanlation_group",
            "id": "group1"
        }),
        serde_json::json!({
            "type": "other",
            "id": "group2"
        })
    ];

    let result = get_scanlation_group(&json);

    assert_eq!(result, Some("group1"));
}

// Returns None if no scanlation group ID exists in the JSON array
#[test]
fn test_get_scanlation_group_returns_none_if_no_scanlation_group_id_exists() {
    let json = vec![
        serde_json::json!({
            "type": "other",
            "id": "group1"
        }),
        serde_json::json!({
            "type": "other",
            "id": "group2"
        })
    ];

    let result = get_scanlation_group(&json);

    assert_eq!(result, None);
}

// Returns None if the input JSON array is empty
#[test]
fn test_get_scanlation_group_returns_none_if_input_json_array_is_empty() {
    let json: Vec<serde_json::Value> = vec![];

    let result = get_scanlation_group(&json);

    assert_eq!(result, None);
}

// Returns None if the input JSON array does not contain any relation objects
#[test]
fn test_get_scanlation_group_returns_none_if_input_json_array_does_not_contain_relation_objects() {
    let json = vec![
        serde_json::json!({
            "type": "other",
            "id": "group1"
        }),
        serde_json::json!({
            "type": "other",
            "id": "group2"
        })
    ];

    let result = get_scanlation_group(&json);

    assert_eq!(result, None);
}

// Returns None if the relation object does not contain a 'type' field
#[test]
fn test_get_scanlation_group_returns_none_if_relation_object_does_not_contain_type_field() {
    let json = vec![
        serde_json::json!({
            "id": "group1"
        }),
        serde_json::json!({
            "type": "other",
            "id": "group2"
        })
    ];

    let result = get_scanlation_group(&json);

    assert_eq!(result, None);
}

// Returns None if the 'type' field of the relation object is not 'scanlation_group'
#[test]
fn test_get_scanlation_group_returns_none_if_type_field_is_not_scanlation_group() {
    let json = vec![
        serde_json::json!({
            "type": "other",
            "id": "group1"
        }),
        serde_json::json!({
            "type": "other",
            "id": "group2"
        })
    ];

    let result = get_scanlation_group(&json);

    assert_eq!(result, None);
}
