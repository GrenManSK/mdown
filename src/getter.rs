use serde_json::Value;
use std::process::exit;

use crate::{
    args::{ self, ARGS },
    download::get_response_client,
    debug,
    error::MdownError,
    log,
    metadata,
    resolute,
    string,
    tutorial,
    utils,
};

/// Retrieves the directory path of the currently running executable.
///
/// This function obtains the path to the executable file of the currently running program
/// and then extracts the directory path containing the executable. It handles errors related
/// to obtaining the executable path and converting it to a string.
///
/// # Returns
/// * `Ok(String)` - The directory path of the executable if successful.
/// * `Err(MdownError)` - An error of type `MdownError` if any issues occur during the process.
///
/// # Errors
/// The function may return errors if:
/// * The current executable path cannot be determined.
/// * The parent directory of the executable cannot be found.
/// * The directory path cannot be converted to a string.
///
/// # Examples
/// ```rust
/// match get_exe_path() {
///     Ok(path) => println!("Executable path: {}", path),
///     Err(err) => handle_error(&err, None),
/// }
/// ```
///
/// # Notes
/// * The function returns a `Result<String, MdownError>` where `String` is the directory path of the executable
///   and `MdownError` is a custom error type used for handling different kinds of errors.
pub(crate) fn get_exe_path() -> Result<String, MdownError> {
    // Attempt to get the path of the current executable.
    let current = match std::env::current_exe() {
        Ok(value) => value,
        Err(err) => {
            return Err(
                MdownError::IoError(
                    err,
                    String::from("The path to your executable file is invalid")
                )
            );
        }
    };

    // Attempt to get the parent directory of the executable path.
    let parent = match current.parent() {
        Some(value) => value,
        None => {
            return Err(MdownError::NotFoundError(String::from("Parent directory not found")));
        }
    };

    // Attempt to convert the parent directory path to a string.
    let path = match parent.to_str() {
        Some(value) => value.to_string(),
        None => {
            return Err(
                MdownError::ConversionError(String::from("Failed to convert path to string"))
            );
        }
    };

    // Return the directory path as a result.
    Ok(path)
}

/// Retrieves the path to the `backup` folder used by the application.
///
/// This function uses `get_exe_path` to obtain the base executable path, then appends
/// `\backup` to it, returning the full path as a `String`.
///
/// # Returns
/// - `Ok(String)`: The full path to `backup` if `get_exe_path` succeeds.
/// - `Err(MdownError)`: If an error occurs when trying to retrieve the executable path.
pub(crate) fn get_bac_path() -> Result<String, MdownError> {
    let path = match get_exe_path() {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };
    Ok(format!("{}\\backup", path))
}

/// Retrieves the path to the `dat.json` file used by the application.
///
/// This function uses `get_exe_path` to obtain the base executable path, then appends
/// `\dat.json` to it, returning the full path as a `String`.
///
/// # Returns
/// - `Ok(String)`: The full path to `dat.json` if `get_exe_path` succeeds.
/// - `Err(MdownError)`: If an error occurs when trying to retrieve the executable path.
pub(crate) fn get_dat_path() -> Result<String, MdownError> {
    let path = match get_exe_path() {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };
    Ok(format!("{}\\dat.json", path))
}

/// Retrieves the path to the `resources.db` file used by the application.
///
/// This function calls `get_exe_path` to obtain the base path, appends `\resources.db`
/// to it, and returns the resulting full path as a `String`.
///
/// # Returns
/// - `Ok(String)`: The full path to `resources.db` if `get_exe_path` succeeds.
/// - `Err(MdownError)`: If an error occurs when trying to retrieve the executable path.
pub(crate) fn get_db_path() -> Result<String, MdownError> {
    let path = match get_exe_path() {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };
    Ok(format!("{}\\resources.db", path))
}

/// Retrieves the path to the `log.json` file used by the application for logging purposes.
///
/// This function uses `get_exe_path` to obtain the executable path, appends `\log.json`
/// to it, and returns the full path as a `String`.
///
/// # Returns
/// - `Ok(String)`: The full path to `log.json` if `get_exe_path` succeeds.
/// - `Err(MdownError)`: If an error occurs when trying to retrieve the executable path.
pub(crate) fn get_log_path() -> Result<String, MdownError> {
    let path: String = match get_exe_path() {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };
    Ok(format!("{}\\log.json", path))
}

/// Retrieves the path to the `log.lock` file used by the application to manage logging locks.
///
/// This function uses `get_exe_path` to obtain the executable path, appends `\log.lock`
/// to it, and returns the full path as a `String`.
///
/// # Returns
/// - `Ok(String)`: The full path to `log.lock` if `get_exe_path` succeeds.
/// - `Err(MdownError)`: If an error occurs when trying to retrieve the executable path.
pub(crate) fn get_log_lock_path() -> Result<String, MdownError> {
    let path: String = match get_exe_path() {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };
    Ok(format!("{}\\log.lock", path))
}

/// Extracts query parameters from a URL path.
///
/// This function parses a URL path to extract the query parameters into a `HashMap`. It expects that
/// the URL path is in the format of a typical URL where the query parameters follow a '?' character
/// and are separated by '&'. Each query parameter is split into key and value by the '=' character.
///
/// # Arguments
/// * `parts` - A `Vec<&str>` where the second element (index 1) contains the URL path with query parameters.
///
/// # Returns
/// * A `HashMap<String, String>` where each key-value pair corresponds to a query parameter and its value.
///
/// # Panics
/// * The function assumes that the input `parts` vector has at least two elements. If the vector is
///   shorter, it may panic due to index out-of-bounds access. Ensure that `parts` has at least two
///   elements before calling this function.
///
/// # Examples
/// ```rust
/// let path_parts = vec!["", "https://example.com/page?key1=value1&key2=value2"];
/// let query_params = get_query(path_parts);
/// assert_eq!(query_params.get("key1"), Some(&"value1".to_string()));
/// assert_eq!(query_params.get("key2"), Some(&"value2".to_string()));
/// ```
///
/// # Note
/// * The function handles cases where query parameters are missing or have empty values, and will
///   include them in the resulting `HashMap` with empty strings as values.
#[cfg(any(feature = "server", feature = "web"))]
pub(crate) fn get_query(parts: Vec<&str>) -> std::collections::HashMap<String, String> {
    parts[1]
        .split('?')
        .nth(1)
        .unwrap_or_default()
        .split('&')
        .map(|param| {
            let mut iter = param.split('=');
            let key = match iter.next() {
                Some(key) => key.to_string(),
                None => String::new(),
            };
            let value = match iter.next() {
                Some(value) => value.to_string(),
                None => String::new(),
            };
            (key, value)
        })
        .collect()
}

/// Retrieves the folder name based on the current ARGS settings.
///
/// This function processes the folder name from the global `ARGS` configuration and returns it as a
/// static string slice. It utilizes `utils::process_filename` to process the folder name. If the
/// processed folder name equals "name", it returns the value from `resolute::MANGA_NAME`. Otherwise,
/// it returns the processed folder name itself.
///
/// # Returns
/// * A `&'static str` representing the folder name. This string is guaranteed to be valid for the
///   duration of the program.
///
/// # Safety
/// * This function uses `Box::leak` to convert a `String` into a static string slice. This is safe
///   here because the strings are meant to be used throughout the lifetime of the program, and
///   no memory management issues should occur. However, using `Box::leak` has the side effect of leaking
///   memory, as the `Box`'s memory is not reclaimed.
///
/// # Examples
/// ```rust
/// let folder_name = get_folder_name();
/// println!("Folder name: {}", folder_name);
/// ```
///
/// # Note
/// * Ensure that `utils::process_filename` and the `ARGS` global configuration are properly initialized
///   before calling this function. Misconfigured or uninitialized values could lead to incorrect results.
pub(crate) fn get_folder_name() -> &'static str {
    let folder_name = utils::process_filename(&ARGS.lock().folder.clone());
    if folder_name == "name" {
        Box::leak(resolute::MANGA_NAME.lock().clone().into_boxed_str())
    } else {
        Box::leak(folder_name.into_boxed_str())
    }
}

/// Retrieves and processes the manga name from the given JSON `title_data`.
///
/// This function attempts to extract the manga title based on a preferred language. It first checks
/// if the title exists in the preferred language specified in the global `LANGUAGE` setting. If the
/// title is not available in the preferred language, it looks into alternative titles provided in
/// the `altTitles` field of the JSON data. The function prioritizes English (`"en"`) and Japanese
/// romanized (`"ja-ro"`) titles if the preferred language title is not available.
///
/// # Arguments
///
/// * `title_data` - A reference to a `serde_json::Value` representing the JSON data containing title
///   information for the manga.
///
/// # Returns
///
/// A `String` containing the processed manga name. If a suitable title cannot be found, it returns
/// `"Unrecognized title"`. The resulting string is trimmed and cleaned of certain characters. If
/// the name exceeds 70 characters, it is truncated to 70 characters and appended with `"__"`.
///
/// # Details
///
/// 1. **Preferred Language:** Checks for the title in the language specified by `LANGUAGE`.
/// 2. **Alternative Titles:** If not found, checks the `altTitles` field for an English or Japanese
///    romanized title.
/// 3. **Fallback:** If no suitable title is found, it tries a general fallback to English and Japanese
///    romanized titles in the `title` field of the JSON data.
/// 4. **Cleanup:** Removes quotes and question marks from the title and trims it to a maximum of 70
///    characters if necessary.
///
/// # Examples
///
/// ```rust
/// let title_data = /* JSON data here */;
/// let manga_name = get_manga_name(&title_data);
/// println!("Manga name: {}", manga_name);
/// ```
///
/// # Note
///
/// Ensure that `resolute::LANGUAGE` is properly initialized before calling this function. The function
/// relies on this global setting to determine the preferred language for the title.
pub(crate) fn get_manga_name(title_data: &Value) -> String {
    let lang = resolute::LANGUAGE.lock().clone();
    let name = (
        match
            title_data
                .get("title")
                .and_then(|attr_data| attr_data.get(lang.clone()))
                .and_then(Value::as_str)
        {
            // If there is manga name with language from args
            Some(manga_name) => {
                drop(lang);
                manga_name.to_string()
            }
            None => {
                // Check altTitles for language that corresponds to args language
                drop(lang);
                let mut return_title = String::from("Unrecognized title");
                let get = title_data.get("altTitles").and_then(|val| val.as_array());
                if let Some(get) = get {
                    if let Some(title_object) = get.iter().next() {
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
                    }
                    if return_title == "Unrecognized title" {
                        // If not found check for japanese and english language
                        for i in [String::from("ja-ro"), String::from("en")] {
                            match
                                title_data
                                    .get("title")
                                    .and_then(|attr_data| attr_data.get(i))
                                    .and_then(Value::as_str)
                            {
                                Some(value) => {
                                    return_title = value.to_string();
                                    break;
                                }
                                None => {
                                    return_title = String::from("Unrecognized title");
                                }
                            };
                        }
                    }

                    // If still not found checks for english and japanese title in title data

                    if return_title == "Unrecognized title" {
                        let mut get_final: serde_json::Map<String, Value> = serde_json::Map::new();

                        for obj in get {
                            if let Value::Object(inner_map) = obj {
                                for (key, value) in inner_map {
                                    get_final.insert(key.to_string(), value.clone());
                                }
                            }
                        }
                        for (lang, title) in get_final {
                            if lang == "en" || lang == "ja-ro" {
                                return_title = title.to_string();
                                break;
                            }
                        }
                    }
                }
                return_title
            }
        }
    )
        .replace("\"", "")
        .replace("?", "")
        .trim()
        .to_string();
    let name = if name.len() > 70 { format!("{}__", &name[0..70]) } else { name };
    utils::process_filename(&name)
}

/// Asynchronously fetches the JSON data for a manga from the MangaDex API.
///
/// This function constructs a URL to fetch manga information by its ID, including cover art. It sends
/// an HTTP GET request to the MangaDex API and processes the response.
///
/// # Arguments
///
/// * `id` - A string slice representing the manga ID.
///
/// # Returns
///
/// * `Ok(String)` - On success, returns the response body as a JSON string.
/// * `Err(MdownError)` - On failure, returns an error of type `MdownError`
///
/// # Errors
///
/// The function will return an `MdownError` if:
/// - The HTTP request fails (`get_response_client` returns an error).
/// - The HTTP response status is not successful.
/// - An error occurs while reading the response body as text.
///
/// # Examples
///
/// ```rust
/// let manga_id = "12345";
/// match get_manga_json(manga_id).await {
///     Ok(json) => println!("Manga JSON: {}", json),
///     Err(e) => eprintln!("Error fetching manga JSON: {:?}", e),
/// }
/// ```
///
/// # Notes
///
/// Ensure the `get_response_client` function is properly implemented to handle HTTP requests.
pub(crate) async fn get_manga_json(id: &str) -> Result<String, MdownError> {
    let full_url = format!("https://api.mangadex.org/manga/{}?includes[]=cover_art", id);

    debug!("sending request to: {}", full_url);

    let response = match get_response_client(&full_url).await {
        Ok(res) => res,
        Err(err) => {
            return Err(err);
        }
    };

    debug!("got response (get_manga_response)");

    if response.status().is_success() {
        debug!("response is success (get_manga_response)\n");
        match response.text().await {
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
        }
    } else {
        debug!("response is error (get_manga_response)");
        eprintln!(
            "Error: get manga json Failed to fetch data from the API. Status code: {:?}",
            response.status()
        );
        Err(MdownError::StatusError(response.status()))
    }
}

/// Asynchronously fetches the JSON data for manga statistics from the MangaDex API.
///
/// This function constructs a URL to fetch manga statistics by its ID. It sends an HTTP GET request to
/// the MangaDex API and processes the response.
///
/// # Arguments
///
/// * `id` - A string slice representing the manga ID.
///
/// # Returns
///
/// * `Ok(String)` - On success, returns the response body as a JSON string.
/// * `Err(MdownError)` - On failure, returns an error of type `MdownError`
///
/// # Errors
///
/// The function will return an `MdownError` if:
/// - The HTTP request fails (`get_response_client` returns an error).
/// - The HTTP response status is not successful.
/// - An error occurs while reading the response body as text.
///
/// # Examples
///
/// ```rust
/// let manga_id = "12345";
/// match get_statistic_json(manga_id).await {
///     Ok(json) => println!("Statistics JSON: {}", json),
///     Err(e) => eprintln!("Error fetching statistics JSON: {:?}", e),
/// }
/// ```
///
/// # Notes
///
/// Ensure the `get_response_client` function is properly implemented to handle HTTP requests.
pub(crate) async fn get_statistic_json(id: &str) -> Result<String, MdownError> {
    let full_url = format!("https://api.mangadex.org/statistics/manga/{}", id);

    debug!("sending request to: {}", full_url);

    let response = match get_response_client(&full_url).await {
        Ok(res) => res,
        Err(err) => {
            return Err(err);
        }
    };
    debug!("got response (get_statistic_json)");
    if response.status().is_success() {
        debug!("response is success (get_statistic_json)");
        let json = match response.text().await {
            Ok(res) => res,
            Err(err) => {
                return Err(MdownError::JsonError(err.to_string()));
            }
        };

        Ok(json)
    } else {
        debug!("response is error (get_statistic_json)");
        eprintln!(
            "Error: get statistic json Failed to fetch data from the API. Status code: {:?}",
            response.status()
        );
        Err(MdownError::StatusError(response.status()))
    }
}

/// Asynchronously retrieves chapter information from the MangaDex API.
///
/// This function constructs a URL to fetch chapter data using its ID. It repeatedly sends an HTTP GET request
/// to the MangaDex API until a successful response is received. The function also handles various errors that
/// may occur during the request process.
///
/// # Arguments
///
/// * `id` - A string slice representing the chapter ID.
///
/// # Returns
///
/// * `Ok(String)` - On success, returns the response body as a JSON string containing chapter information.
/// * `Err(MdownError)` - On failure, returns an error of type `MdownError`.
///
/// # Errors
///
/// The function will return an `MdownError` if:
/// - The HTTP request fails (`get_response_client` returns an error).
/// - The HTTP response status is not successful, and an error occurs while reading the response body as text.
///
/// # Notes
///
/// The function uses a loop to retry the request until a successful response is received. Make sure the `get_response_client`
/// function is properly implemented to handle HTTP requests.
///
/// # Examples
///
/// ```rust
/// let chapter_id = "123456";
/// match get_chapter(chapter_id).await {
///     Ok(json) => println!("Chapter JSON: {}", json),
///     Err(e) => eprintln!("Error fetching chapter JSON: {:?}", e),
/// }
/// ```
pub(crate) async fn get_chapter(id: &str) -> Result<String, MdownError> {
    loop {
        string(3, 0, "Retrieving chapter info");
        if *tutorial::TUTORIAL.lock() && *tutorial::TUTORIAL_CHAPTER_INFO.lock() {
            tutorial::chapter_info();
            *tutorial::TUTORIAL_CHAPTER_INFO.lock() = false;
        }

        let base_url = "https://api.mangadex.org/at-home/server/";
        let full_url = format!("{}{}", base_url, id);

        debug!("sending request to: {}", full_url);

        let response = match get_response_client(&full_url).await {
            Ok(res) => res,
            Err(err) => {
                return Err(err);
            }
        };

        debug!("got response of chapter images");

        if response.status().is_success() {
            debug!("response is success");
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
            debug!("response is not successful");
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

/// Retrieves the scanlation group ID from a list of chapter relation responses.
///
/// This function iterates through the provided list of chapter relation responses and searches for a relation
/// with the type "scanlation_group". If found, it returns the ID of the scanlation group.
///
/// # Arguments
///
/// * `json` - A vector of `metadata::ChapterRelResponse` objects representing chapter relations.
///
/// # Returns
///
/// * `Option<String>` - An `Option` containing the scanlation group ID as a `String` if found; otherwise, `None`.
///
/// # Examples
///
/// ```rust
/// let relations = vec![ /* populate with chapter relation data */ ];
/// match get_scanlation_group(&relations) {
///     Some(group_id) => println!("Scanlation Group ID: {}", group_id),
///     None => println!("Scanlation group not found."),
/// }
/// ```
///
/// # Notes
///
/// Ensure that `metadata::ChapterRelResponse` is properly defined to include the `r#type` and `id` fields.
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

/// Asynchronously fetches manga data from the MangaDex API with pagination.
///
/// This function retrieves manga data by making repeated HTTP GET requests to the MangaDex API with different offsets
/// until all data is fetched. It accumulates the fetched data and returns it along with the number of items retrieved.
///
/// # Arguments
///
/// * `id` - A string slice representing the manga ID to fetch data for.
/// * `offset` - The initial offset to start fetching data from.
///
/// # Returns
///
/// * `Ok((String, usize))` - On success, returns a tuple containing the combined JSON response as a `String` and the number
///   of items retrieved in the current session as `usize`.
/// * `Err(MdownError)` - On failure, returns an error of type `MdownError`.
///
/// # Errors
///
/// The function will return an `MdownError` if:
/// - The HTTP request fails (`get_response_client` returns an error).
/// - The HTTP response status is not successful, and an error occurs while reading the response body as text.
/// - There is an error parsing the JSON response or combining the data.
///
/// # Notes
///
/// The function uses a loop to handle pagination by updating the offset and making requests until fewer items than the
/// maximum per session are received. The `crossfade_data` function is used to merge the data from multiple requests.
///
/// # Examples
///
/// ```rust
/// let manga_id = "123456";
/// let offset = 0;
/// match get_manga(manga_id, offset).await {
///     Ok((json, count)) => println!("Fetched {} items: {}", count, json),
///     Err(e) => eprintln!("Error fetching manga data: {:?}", e),
/// }
/// ```
pub(crate) async fn get_manga(id: &str, offset: u32) -> Result<(String, usize), MdownError> {
    let mut times = 0;
    let mut json;
    let mut json_2 = String::new();
    let mut times_offset: u32;
    let max_per_session = 500;
    let stat = match ARGS.lock().stat {
        true => 1,
        false => 0,
    };
    loop {
        times_offset = offset + 500 * times;
        string(
            3 + times + stat,
            0,
            &format!("{} {} {}   ", times, "Fetching data with offset", times_offset)
        );
        debug!("fetching data with offset {}", times_offset);
        let full_url = format!(
            "https://api.mangadex.org/manga/{}/feed?limit={}&offset={}",
            id,
            max_per_session,
            times_offset
        );
        if *tutorial::TUTORIAL.lock() && times == 0 {
            tutorial::feed(stat);
        }

        debug!("sending request to: {}", full_url);

        let response = match get_response_client(&full_url).await {
            Ok(res) => res,
            Err(err) => {
                return Err(err);
            }
        };
        debug!("got response");
        if !response.status().is_success() {
            debug!("response is not a success");
            eprintln!(
                "Error: get manga Failed to fetch data from the API. Status code: {:?} ({})",
                response.status(),
                full_url
            );
            return Err(MdownError::StatusError(response.status()));
        }
        json = match response.text().await {
            Ok(text) => text,
            Err(err) => {
                return Err(
                    MdownError::StatusError(match err.status() {
                        Some(status) => status,
                        None => {
                            return Err(
                                MdownError::NotFoundError(String::from("StatusCode (get_manga)"))
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
        debug!("data parsed");
        match json_value {
            Value::Object(obj) => {
                if let Some(data_array) = obj.get("data").and_then(Value::as_array) {
                    let naive_time_str = chrono::Utc
                        ::now()
                        .naive_utc()
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string();

                    resolute::DATE_FETCHED.lock().push(naive_time_str);
                    let message = format!("{} Data fetched with offset {}   ", times, offset);
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
                    if offset_temp >= max_per_session {
                        debug!("data is at or exceeded maximum {}", max_per_session);
                        json_2 = json;
                        times += 1;
                        continue;
                    } else {
                        offset_temp = data_array.len();
                    }
                    if times > 0 {
                        debug!("joining data");
                        json = match crossfade_data(&json, &json_2) {
                            Ok(value) => value,
                            Err(err) => {
                                return Err(err);
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
    }
}

/// Merges two JSON strings by appending the data from the second JSON to the first JSON.
///
/// This function takes two JSON strings, parses them, and combines their "data" arrays. The combined JSON is returned
/// as a string.
///
/// # Arguments
///
/// * `json` - A string containing the first JSON object to which data will be appended.
/// * `json_2` - A string containing the second JSON object whose data will be appended to the first JSON object.
///
/// # Returns
///
/// * `Ok(String)` - On success, returns the combined JSON as a string.
/// * `Err(MdownError)` - On failure, returns an error of type `MdownError` if there are issues parsing the JSON or merging the data.
///
/// # Errors
///
/// The function will return an `MdownError` if:
/// - There is an error parsing either JSON string.
/// - The "data" arrays are not found in the JSON objects.
///
/// # Examples
///
/// ```rust
/// let json1 = r#"{"data": [{"id": "1"}]}"#;
/// let json2 = r#"{"data": [{"id": "2"}]}"#;
/// match crossfade_data(json1, json2) {
///     Ok(combined_json) => println!("Combined JSON: {}", combined_json),
///     Err(e) => eprintln!("Error combining JSON: {:?}", e),
/// }
/// ```
///
/// # Notes
///
/// Ensure that the input JSON strings have a "data" field that contains arrays of JSON objects.
fn crossfade_data(json: &str, json_2: &str) -> Result<String, MdownError> {
    // Add json_2.data to json.data
    let mut data1 = match utils::get_json(json) {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };
    let data2 = match utils::get_json(json_2) {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };

    let data1_array = match data1.get_mut("data") {
        Some(value) => value,
        None => {
            return Err(MdownError::JsonError(String::from("Didn't found data")));
        }
    };
    let data2_array = match data2.get("data") {
        Some(value) => value,
        None => {
            return Err(MdownError::JsonError(String::from("Didn't found data")));
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

    match serde_json::to_string(&data1) {
        Ok(value) => Ok(value),
        Err(err) => { Err(MdownError::JsonError(err.to_string())) }
    }
}

/// Retrieves a reference to an element in a slice of `String` based on the given index.
///
/// This function returns a reference to the `String` at the specified index in the provided slice. If the index is out of bounds,
/// it prints an error message and exits the program.
///
/// # Arguments
///
/// * `data_array` - A slice of `String` containing the data.
/// * `item` - The index of the element to retrieve.
///
/// # Returns
///
/// * `&String` - A reference to the `String` at the specified index.
///
/// # Errors
///
/// This function will print an error message and exit the program if the index is out of bounds.
///
/// # Examples
///
/// ```rust
/// let data = vec![String::from("first"), String::from("second")];
/// let item = 1;
/// let value = get_attr_as_same_as_index(&data, item);
/// println!("{}", value); // Prints: second
/// ```
pub(crate) fn get_attr_as_same_as_index(data_array: &[String], item: usize) -> &String {
    match data_array.get(item) {
        Some(value) => value,
        None => {
            eprintln!("{}", MdownError::NotFoundError(String::from("get_attr_as_same_as_index")));
            exit(10801);
        }
    }
}

/// Retrieves a reference to an element in a slice of `metadata::ChapterResponse` based on the given index.
///
/// This function returns a reference to the `metadata::ChapterResponse` at the specified index in the provided slice. If the index is out of bounds,
/// it prints an error message and exits the program.
///
/// # Arguments
///
/// * `data_array` - A slice of `metadata::ChapterResponse` containing the data.
/// * `item` - The index of the element to retrieve.
///
/// # Returns
///
/// * `&metadata::ChapterResponse` - A reference to the `metadata::ChapterResponse` at the specified index.
///
/// # Errors
///
/// This function will print an error message and exit the program if the index is out of bounds.
///
/// # Examples
///
/// ```rust
/// let data = vec![metadata::ChapterResponse { /* fields */ }, metadata::ChapterResponse { /* fields */ }];
/// let item = 0;
/// let value = get_attr_as_same_from_vec(&data, item);
/// println!("{:?}", value); // Prints: The ChapterResponse at index 0
/// ```
pub(crate) fn get_attr_as_same_from_vec(
    data_array: &[metadata::ChapterResponse],
    item: usize
) -> &metadata::ChapterResponse {
    match data_array.get(item) {
        Some(value) => value,
        None => {
            eprintln!("{}", MdownError::NotFoundError(String::from("get_attr_as_same_from_vec")));
            exit(10802);
        }
    }
}

/// Extracts and returns metadata attributes from a `metadata::ChapterResponse` object.
///
/// This function extracts attributes from a `metadata::ChapterResponse` object and returns them as a tuple. The returned tuple includes
/// the chapter attributes, the language, the number of pages, the chapter number, and the title.
///
/// # Arguments
///
/// * `array_item` - A reference to a `metadata::ChapterResponse` object from which metadata is extracted.
///
/// # Returns
///
/// * A tuple containing:
///   - `metadata::ChapterAttrResponse` - The attributes of the chapter.
///   - `String` - The language of the chapter.
///   - `u64` - The number of pages in the chapter.
///   - `String` - The chapter number.
///   - `String` - The title of the chapter.
///
/// # Examples
///
/// ```rust
/// let chapter_response = metadata::ChapterResponse { /* fields */ };
/// let (attr, lang, pages, chapter_num, title) = get_metadata(&chapter_response);
/// println!("Chapter Title: {}", title); // Prints: Chapter Title
/// ```
pub(crate) fn get_metadata(
    array_item: &metadata::ChapterResponse
) -> (metadata::ChapterAttrResponse, String, u64, String, String) {
    let chapter_attr = array_item.attributes.clone();
    let lang = chapter_attr.translatedLanguage.clone().unwrap_or_default();
    let pages = chapter_attr.pages;
    let chapter_num = chapter_attr.chapter.clone().unwrap_or_default();
    let title = chapter_attr.title.clone().unwrap_or_default();
    (chapter_attr, lang, pages, chapter_num, title)
}

/// Returns a formatted argument string, defaulting to "*" if the argument is empty.
///
/// This function checks if the provided argument is an empty string and returns `"*"` in that case. Otherwise, it returns the argument itself.
///
/// # Arguments
///
/// * `arg` - The argument string to be formatted.
///
/// # Returns
///
/// * `&str` - The formatted argument string.
///
/// # Examples
///
/// ```rust
/// let arg = "";
/// let result = get_arg(arg);
/// println!("{}", result); // Prints: *
///
/// let arg = "some_value";
/// let result = get_arg(arg);
/// println!("{}", result); // Prints: some_value
/// ```
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

    *resolute::LANGUAGE.lock() = String::from("en");

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

// Retrieves manga name based on the provided language in title_data

#[test]
fn retrieves_manga_name_based_on_language() {
    let title_data =
        serde_json::json!({
            "title": {
                "en": "One Piece",
                "ja-ro": "Wan Pīsu"
            },
            "altTitles": [
                {
                    "en": "One Piece",
                    "ja-ro": "Wan Pīsu"
                }
            ]
        });

    let result = get_manga_name(&title_data);
    assert_eq!(result, "One Piece");
}

// Handles missing or null title_data gracefully

#[test]
fn handles_missing_or_null_title_data() {
    let title_data = serde_json::json!({});

    let result = get_manga_name(&title_data);
    assert_eq!(result, "Unrecognized title");
}
