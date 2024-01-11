use serde_json::Value;
use std::{ thread::sleep, time::Duration, process::exit };
use tracing::info;

use crate::{
    ARGS,
    string,
    utils::progress_bar_preparation,
    getter,
    MAXPOINTS,
    download::get_response_client,
};

pub(crate) fn get_folder_name(manga_name: &str) -> String {
    if ARGS.folder == "name" { manga_name.to_owned() } else { ARGS.folder.as_str().to_string() }
}

pub(crate) fn get_manga_name(title_data: &Value) -> String {
    title_data
        .get("title")
        .and_then(|attr_data| attr_data.get("en"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| {
            let get = title_data.get("altTitles").and_then(|val| val.as_array());
            if get.is_some() {
                let mut return_title = "*";
                for title_object in get.unwrap() {
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
                    let and_then = title_data
                        .get("title")
                        .and_then(|attr_data| attr_data.get("ja-ro"))
                        .and_then(Value::as_str);
                    if and_then.is_some() {
                        and_then.unwrap()
                    } else {
                        "Unrecognized title"
                    }
                } else {
                    return_title
                }
            } else {
                let and_then = title_data
                    .get("title")
                    .and_then(|attr_data| attr_data.get("ja-ro"))
                    .and_then(Value::as_str);
                if and_then.is_some() {
                    and_then.unwrap()
                } else {
                    "Unrecognized title"
                }
            }
        })
        .to_string()
}

pub(crate) async fn get_manga_json(id: &str) -> Result<String, reqwest::StatusCode> {
    let full_url = format!("https://api.mangadex.org/manga/{}?includes[]=cover_art", id);

    let response = get_response_client(full_url).await.unwrap();

    if response.status().is_success() {
        let json = response.text().await.unwrap();

        Ok(json)
    } else {
        eprintln!(
            "Error: {}",
            format!("Failed to fetch data from the API. Status code: {:?}", response.status())
        );
        Err(response.status())
    }
}

pub(crate) async fn get_statistic_json(id: &str) -> Result<String, reqwest::StatusCode> {
    let full_url = format!("https://api.mangadex.org/statistics/manga/{}", id);

    let response = get_response_client(full_url).await.unwrap();

    if response.status().is_success() {
        let json = response.text().await.unwrap();

        Ok(json)
    } else {
        eprintln!(
            "Error: {}",
            format!("Failed to fetch data from the API. Status code: {:?}", response.status())
        );
        Err(response.status())
    }
}

pub(crate) async fn get_chapter(id: &str) -> Result<String, reqwest::Error> {
    loop {
        string(4, 0, "Retrieving chapter info");

        let base_url = "https://api.mangadex.org/at-home/server/";
        let full_url = format!("{}{}", base_url, id);

        let response = get_response_client(full_url).await.unwrap();

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
            progress_bar_preparation(MAXPOINTS.max_x - 30, 60, 7);
            for i in 0..60 {
                sleep(Duration::from_millis(1000));
                string(7, MAXPOINTS.max_x - 29 + i, "#");
            }
        }
    }
}

pub(crate) fn get_scanlation_group(json: &Vec<Value>) -> Option<&str> {
    for relation in json {
        let relation_type = relation.get("type");
        if relation_type.is_none() {
            return None;
        }
        if relation_type.unwrap() == "scanlation_group" {
            return relation.get("id").and_then(Value::as_str);
        }
    }
    None
}

pub(crate) async fn get_manga(
    id: &str,
    offset: i32,
    handle_id: Option<String>
) -> Result<(String, usize), reqwest::Error> {
    let handle_id = handle_id.unwrap();
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
        let full_url = format!(
            "https://api.mangadex.org/manga/{}/feed?limit=500&offset={}",
            id,
            times_offset
        );

        let response = get_response_client(full_url).await.unwrap();

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
                                let message = format!(
                                    "{} Data fetched with offset {}   ",
                                    times.to_string(),
                                    offset.to_string()
                                );
                                string(1, 0, &message);
                                if ARGS.web {
                                    info!("@{} {}", handle_id, message);
                                }
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
    if ARGS.saver { String::from("dataSaver") } else { String::from("data") }
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
    let title_data =
        serde_json::json!({
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
