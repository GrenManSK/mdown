use lazy_static::lazy_static;
use percent_encoding::{ NON_ALPHANUMERIC, percent_decode_str, percent_encode };
use serde_json::{ json, Value };
use std::{ collections::HashMap, fs::File, io::{ Read, Write }, net::TcpListener };

use crate::{
    args,
    db,
    error::MdownError,
    getter,
    handle_error,
    log,
    log_end,
    resolute::{
        self,
        CURRENT_CHAPTER,
        CURRENT_CHAPTER_PARSED,
        CURRENT_CHAPTER_PARSED_MAX,
        CURRENT_PAGE,
        CURRENT_PAGE_MAX,
        CURRENT_PERCENT,
        CURRENT_SIZE,
        CURRENT_SIZE_MAX,
        MANGA_NAME,
        SCANLATION_GROUPS,
        WEB_DOWNLOADED,
    },
    utils,
    version_manager::get_current_version,
    zip_func,
};

lazy_static! {
    static ref RAMBLING_PLEAT_OGG: Vec<u8> = {
        let db_path = match getter::get_db_path() {
            Ok(path) => path,
            Err(_err) => std::process::exit(11308),
        };
        let conn = match rusqlite::Connection::open(db_path) {
            Ok(conn) => conn,
            Err(_err) => std::process::exit(11309),
        };
        match db::read_resource(&conn, "1001") {
            Ok(value) =>
                match value {
                    Some(value) => value,
                    None => std::process::exit(1001),
                }
            Err(_err) => std::process::exit(1001),
        }
    };

    static ref SYSTEM_HAVEN_OGG: Vec<u8> = {
        let db_path = match getter::get_db_path() {
            Ok(path) => path,
            Err(_err) => std::process::exit(11310),
        };
        let conn = match rusqlite::Connection::open(db_path) {
            Ok(conn) => conn,
            Err(_err) => std::process::exit(11311),
        };
        match db::read_resource(&conn, "1002") {
            Ok(value) =>
                match value {
                    Some(value) => value,
                    None => std::process::exit(1002),
                }
            Err(_err) => std::process::exit(1002),
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/error_404_jpg.rs"));

/// Decodes a percent-encoded URL string.
///
/// # Parameters
/// - `url`: A percent-encoded string.
///
/// # Returns
/// - A decoded `String` where percent-encoded sequences are replaced with their UTF-8 representation.
///
/// # Example
/// ```
/// let decoded = decode("hello%20world");
/// assert_eq!(decoded, "hello world");
/// ```
fn decode(url: &str) -> String {
    percent_decode_str(url).decode_utf8_lossy().to_string()
}

/// Encodes a string into a percent-encoded format.
///
/// # Parameters
/// - `url`: A regular string to be percent-encoded.
///
/// # Returns
/// - A `String` where non-alphanumeric characters are percent-encoded.
///
/// # Example
/// ```
/// let encoded = encode("hello world");
/// assert_eq!(encoded, "hello%20world");
/// ```
pub(crate) fn encode(url: &str) -> String {
    percent_encode(url.as_bytes(), NON_ALPHANUMERIC).to_string()
}

/// Resolves and downloads manga information based on a given URL.
///
/// # Parameters
/// - `url`: A string slice representing the manga URL or identifier.
///
/// # Returns
/// - `Ok(String)`: A JSON response string containing manga details if successful.
/// - `Err(MdownError)`: If an error occurs during resolution or data retrieval.
///
/// # Behavior
/// - Extracts the manga ID from the given URL using regex or UUID validation.
/// - Fetches the manga JSON data from an external source.
/// - Parses and processes the manga metadata.
/// - Returns a JSON object with:
///     - `"status": "ok"`
///     - `"name"`: Manga title
///     - `"files"`: List of downloaded files
///     - `"scanlation_groups"`: List of associated scanlation groups.
///
/// # Example JSON Response
/// ```json
/// {
///   "status": "ok",
///   "name": "Manga Title",
///   "files": ["chapter1.zip", "chapter2.zip"],
///   "scanlation_groups": ["Group A", "Group B"]
/// }
/// ```
async fn resolve_web_download(url: &str) -> Result<String, MdownError> {
    let handle_id = resolute::HANDLE_ID.lock().clone();
    let mut manga_name = String::from("!");
    let id;
    if let Some(id_temp) = utils::resolve_regex(url) {
        id = id_temp.as_str();
    } else if utils::is_valid_uuid(url) {
        id = url;
    } else {
        log!(&format!("@{} Didn't find any id", handle_id), handle_id);
        return Ok(String::from("!"));
    }
    *resolute::MANGA_ID.lock() = id.to_string();
    log!(&format!("@{} Found {}", handle_id, id), handle_id);
    if let Ok(manga_name_json) = getter::get_manga_json(id).await {
        let json_value = match utils::get_json(&manga_name_json) {
            Ok(value) => value,
            Err(err) => {
                return Err(MdownError::ChainedError(Box::new(err), 11312));
            }
        };
        match json_value {
            Value::Object(obj) => {
                manga_name = match resolute::resolve(obj, id).await {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(MdownError::ChainedError(Box::new(err), 11313));
                    }
                };
            }
            _ => {
                return Err(
                    MdownError::JsonError(String::from("Could not parse manga json"), 11300)
                );
            }
        }
    }

    if manga_name.eq("!") {
        Ok(String::from("!"))
    } else {
        let downloaded_files = WEB_DOWNLOADED.lock().clone();
        let scanlation = SCANLATION_GROUPS.lock().clone();

        let response_map: HashMap<&str, serde_json::Value> = [
            ("status", serde_json::Value::String("ok".to_string())),
            ("name", serde_json::Value::String(manga_name.to_string())),
            (
                "files",
                serde_json::Value::Array(
                    downloaded_files.into_iter().map(serde_json::Value::String).collect()
                ),
            ),
            (
                "scanlation_groups",
                serde_json::Value::Array(
                    scanlation
                        .clone()
                        .into_iter()
                        .map(|x| x.name)
                        .map(serde_json::Value::String)
                        .collect()
                ),
            ),
        ]
            .iter()
            .cloned()
            .collect();

        match serde_json::to_string(&response_map) {
            Ok(value) => Ok(value),
            Err(err) => { Err(MdownError::JsonError(err.to_string(), 11301)) }
        }
    }
}

/// Handles an incoming TCP client connection for the server.
///
/// # Parameters
/// - `stream`: A `TcpStream` representing the client connection.
///
/// # Returns
/// - `Ok(())` if the request is processed successfully.
/// - `Err(MdownError)` if an error occurs during request handling.
///
/// # Behavior
/// - Reads the request from the client.
/// - Parses the request path and handles different endpoints:
///     - `/manga?url=...` → Handles manga downloads.
///     - `/__get__?path=...` → Serves static resources.
///     - `/__confetti__` → Extracts and serves images from a ZIP archive.
///     - `/manga-result?id=...` → Retrieves download progress.
///     - `/__version__` → Returns the current application version.
///     - `/end` → Signals the server to exit.
///     - `/` → Handles the main request.
/// - Sends appropriate HTTP responses based on the request type.
/// - Logs requests and errors.
/// - Calls `std::process::exit(0)` if the `/end` endpoint is requested.
///
/// # Example Usage
/// ```no_run
/// use std::net::{TcpListener, TcpStream};
/// use std::thread;
///
/// fn main() {
///     let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
///     for stream in listener.incoming() {
///         match stream {
///             Ok(stream) => {
///                 thread::spawn(|| {
///                     let _ = handle_client(stream);
///                 });
///             }
///             Err(e) => eprintln!("Connection failed: {}", e),
///         }
///     }
/// }
/// ```
async fn handle_client(mut stream: std::net::TcpStream) -> Result<(), MdownError> {
    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(_n) => (),
        Err(err) => {
            return Err(MdownError::IoError(err, String::new(), 11302));
        }
    }

    let mut end = false;

    let request = String::from_utf8_lossy(&buffer[..]);

    let url_param = "url=";

    let parts: Vec<&str> = request.split_whitespace().collect();

    let path = match parts.get(1) {
        Some(part) => *part,
        None => {
            return Err(MdownError::NotFoundError(String::from("Invalid request"), 11315));
        }
    };

    if parts.len() >= 2 {
        let response;
        if path.starts_with("/manga?") && path.contains(url_param) {
            log!("REQUEST RECEIVED");
            log!("REQUEST Type: download");

            let query_params = getter::get_query(parts);
            if let Some(manga_url) = query_params.get("url").cloned() {
                let handle_id = match query_params.get("id").cloned() {
                    Some(id) => id.into_boxed_str(),
                    None => String::from("0").into_boxed_str(),
                };
                let decoded_url = decode(&manga_url);

                *resolute::HANDLE_ID.lock() = handle_id.clone();
                let json = match resolve_web_download(&decoded_url).await {
                    Ok(response) =>
                        format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}", response),

                    Err(err) => {
                        handle_error!(&err, String::from("web_manga"));
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}",
                            r#"{"status": "error"}"#
                        )
                    }
                };

                log_end(handle_id);
                *resolute::HANDLE_ID.lock() = String::new().into_boxed_str();
                response = json;
            } else {
                response = String::from(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"error\"}"
                );
            }
            match stream.write_all(response.as_bytes()) {
                Ok(()) => (),
                Err(_err) => (),
            }
        } else if path.starts_with("/__get__?") {
            log!("REQUEST Type: GET");
            let query_params = getter::get_query(parts);
            let file_path = match query_params.get("path").cloned() {
                Some(value) => value,
                None => {
                    return Ok(());
                }
            };

            log!(&format!("REQUESTING: {}", file_path));

            let content = match file_path.as_str() {
                "error_404" => ERROR_404_JPG,
                "rambling_pleat" => &RAMBLING_PLEAT_OGG,
                "system_haven" => &SYSTEM_HAVEN_OGG,
                _ => {
                    return Err(
                        MdownError::CustomError(
                            String::from("Didn't find resource"),
                            String::from("Resource"),
                            11303
                        )
                    );
                }
            };
            match stream.write_all(content) {
                Ok(()) => (),
                Err(_err) => (),
            }
        } else if path.starts_with("/__confetti__") {
            let content: Vec<Vec<u8>> = match zip_func::extract_images_from_zip() {
                Ok(content) => content,
                Err(err) => {
                    return Err(MdownError::ChainedError(Box::new(err), 11314));
                }
            };

            #[allow(deprecated)]
            let base64_content: Vec<String> = content.iter().map(base64::encode).collect();

            match
                stream.write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}",
                        json!({ "images": base64_content })
                    ).as_bytes()
                )
            {
                Ok(()) => (),
                Err(_err) => (),
            }
        } else if path.starts_with("/manga-result") {
            let query_params = getter::get_query(parts);
            if let Some(id) = query_params.get("id").cloned() {
                log!("REQUEST RECEIVED", id.clone().into_boxed_str());
                log!("REQUEST Type: progress", id.clone().into_boxed_str());
                match parse_request(String::from("progress")) {
                    Ok(value) => {
                        response = value;
                    }
                    Err(err) => {
                        handle_error!(&err, String::from("main"));
                        response = String::from(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"error\"}"
                        );
                    }
                };
            } else {
                response = String::from(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"error\"}"
                );
            }
            match stream.write_all(response.as_bytes()) {
                Ok(()) => (),
                Err(_err) => (),
            }
        } else if path.starts_with("/__version__") {
            response = format!(
                "{}{}",
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n",
                get_current_version()
            );
            match stream.write_all(response.as_bytes()) {
                Ok(()) => (),
                Err(_err) => (),
            }
        } else if path.starts_with("/end") {
            log!("REQUEST Type: end");
            end = true;
            response = String::from(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"ok\"}"
            );
            match stream.write_all(response.as_bytes()) {
                Ok(()) => (),
                Err(_err) => (),
            }
        } else if path.eq("/") {
            log!("REQUEST Type: main");
            match parse_request(String::from("main")) {
                Ok(value) => {
                    response = value;
                }
                Err(err) => {
                    handle_error!(&err, String::from("main"));
                    response = String::from(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"error\"}"
                    );
                }
            }
            match stream.write_all(response.as_bytes()) {
                Ok(()) => (),
                Err(_err) => (),
            }
        } else {
            response = format!(
                "{}{}",
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n",
                get_error_html()
            );
            match stream.write_all(response.as_bytes()) {
                Ok(()) => (),
                Err(_err) => (),
            }
        }
    } else {
        match
            stream.write_all(
                String::from(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"error\"}"
                ).as_bytes()
            )
        {
            Ok(()) => (),
            Err(_err) => (),
        }
    }
    match stream.flush() {
        Ok(()) => (),
        Err(_err) => (),
    }

    if end {
        log!("[user] Exit event received! Exiting...");
        log!("[web] Closing server");

        match utils::remove_cache() {
            Ok(()) => (),
            Err(err) => {
                handle_error!(&err, String::from("ctrl_handler"));
            }
        }
        std::process::exit(0);
    }
    Ok(())
}

/// Parses an incoming request and generates an appropriate HTTP response.
///
/// # Parameters
/// - `url`: A `String` representing the request type.
///
/// # Returns
/// - `Ok(String)`: A formatted HTTP response string.
/// - `Err(MdownError)`: If the request type is unrecognized or JSON serialization fails.
///
/// # Behavior
/// - If `url == "main"`:
///   - Returns an HTML response containing the main page content.
/// - If `url == "progress"`:
///   - Constructs a JSON response containing download progress, including:
///     - `status`: `"ok"`
///     - `name`: Manga title
///     - `current`: Current chapter
///     - `current_page`: Page number progress
///     - `current_percent`: Download percentage
///     - `files`: List of downloaded files
///     - `scanlation_groups`: List of scanlation groups
/// - If the request type is unrecognized, returns a `NotFoundError`.
///
/// # Example JSON Response (progress)
/// ```json
/// {
///   "status": "ok",
///   "name": "Manga Title",
///   "current": "Chapter 5",
///   "current_page": "12",
///   "current_page_max": "30",
///   "current_percent": "40.00",
///   "current_size": "15.23",
///   "current_size_max": "37.50",
///   "current_chapter_parsed": "5",
///   "current_chapter_parsed_max": "10",
///   "files": ["chapter1.zip", "chapter2.zip"],
///   "scanlation_groups": ["Group A", "Group B"]
/// }
/// ```
fn parse_request(url: String) -> Result<String, MdownError> {
    if url == *"main" {
        let html = get_html();
        Ok(format!("{}{}", "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n", html))
    } else if url == *"progress" {
        let downloaded_files = WEB_DOWNLOADED.lock().clone();
        let scanlation = SCANLATION_GROUPS.lock().clone();
        let response_map: HashMap<&str, serde_json::Value> = [
            ("status", serde_json::Value::String("ok".to_string())),
            ("name", serde_json::Value::String(MANGA_NAME.lock().to_string())),
            ("current", serde_json::Value::String(CURRENT_CHAPTER.lock().to_string())),
            ("current_page", serde_json::Value::String(CURRENT_PAGE.lock().to_string())),
            ("current_page_max", serde_json::Value::String(CURRENT_PAGE_MAX.lock().to_string())),
            (
                "current_percent",
                serde_json::Value::String(format!("{:.2}", CURRENT_PERCENT.lock())),
            ),
            ("current_size", serde_json::Value::String(format!("{:.2}", CURRENT_SIZE.lock()))),
            (
                "current_size_max",
                serde_json::Value::String(format!("{:.2}", CURRENT_SIZE_MAX.lock())),
            ),
            (
                "current_chapter_parsed",
                serde_json::Value::String(CURRENT_CHAPTER_PARSED.lock().to_string()),
            ),
            (
                "current_chapter_parsed_max",
                serde_json::Value::String(CURRENT_CHAPTER_PARSED_MAX.lock().to_string()),
            ),
            (
                "files",
                serde_json::Value::Array(
                    downloaded_files.into_iter().map(serde_json::Value::String).collect()
                ),
            ),
            (
                "scanlation_groups",
                serde_json::Value::Array(
                    scanlation
                        .clone()
                        .into_iter()
                        .map(|x| x.name)
                        .map(serde_json::Value::String)
                        .collect()
                ),
            ),
        ]
            .iter()
            .cloned()
            .collect();
        let json = match serde_json::to_string(&response_map) {
            Ok(value) => value,
            Err(err) => {
                return Err(MdownError::JsonError(err.to_string(), 11304));
            }
        };
        Ok(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}", json))
    } else {
        Err(MdownError::NotFoundError(String::new(), 11305))
    }
}

/// Loads the HTML content for the web interface.
///
/// # Returns
/// - A `String` containing the HTML content.
/// - If in development mode (`ARGS_DEV` is `true`), attempts to read `web.html`.
/// - If `web.html` cannot be read, returns the error page from `get_error_html()`.
/// - If not in development mode, returns a placeholder string.
///
/// # Behavior
/// - When `ARGS_DEV` is `true`:
///   - Tries to open `web.html`.
///   - If successful, reads and returns its contents.
///   - If unsuccessful, returns the error page.
/// - When `ARGS_DEV` is `false`:
///   - Returns `"..."` (default production content).
///
/// # Example
/// ```rust
/// let html = get_html();
/// println!("{}", html); // Outputs either the web page content or an error page.
/// ```
fn get_html() -> String {
    if *args::ARGS_DEV {
        let err_404 = get_error_html();
        let mut file = match File::open("web.html") {
            Ok(file) => file,
            Err(_err) => {
                return err_404;
            }
        };

        let mut contents = String::new();
        match file.read_to_string(&mut contents) {
            Ok(_) => (),
            Err(_err) => {
                return err_404;
            }
        }
        contents
    } else {
        String::from(
            "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"UTF-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"><title>Mdown</title><style>body {font-family: Arial, sans-serif;background-color: #121212;color: #fff;margin: 0;padding: 0;box-sizing: border-box;transition: background-color 0.5s;}body.dark-mode {background-color: #fff;color: #121212;}.title {margin-left: 44vw;color: inherit;display: flex;align-items: center;}.mangaForm {max-width: 400px;margin: 20px auto;background-color: #272727;padding: 20px;border-radius: 8px;box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);}.mangaForm.dark-mode {color: #FFF;background-color: #FFF;}.urlInput {display: block;margin-bottom: 8px;color: #fff;}.urlInput.dark-mode {color: #000;}input {width: 100%;padding: 10px;margin-bottom: 16px;box-sizing: border-box;border: 1px solid #555;border-radius: 4px;background-color: #333;color: #fff;}.exit-button {background-color: #FFF;color: #000;padding: 10px 15px;border: none;border-radius: 50%;cursor: pointer;position: fixed;top: 20px;left: 20px;font-size: 20px;}.dark-mode-toggle {background-color: #FFF;color: #000;padding: 10px 15px;border: none;border-radius: 50%;cursor: pointer;position: fixed;top: 20px;right: 20px;font-size: 20px;}.dark-mode-toggle:hover {background-color: grey;}.download {background-color: #4caf50;color: #fff;padding: 10px 15px;border: none;border-radius: 4px;cursor: pointer;}.download:hover {background-color: #45a049;}#resultMessage {margin: 20px auto;max-width: 600px;background-color: #272727;padding: 50px;border-radius: 8px;box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);}ul {list-style-type: none;padding: 0;}li {margin-bottom: 8px;}#result {color: #FFF;}#resultEnd {margin: 20px auto;max-width: 600px;background-color: #272727;padding: 50px;border-radius: 8px;box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);animation: popUp 1s ease-out;display: none;transform: scale(0);opacity: 0;}#resultEnd.dark-mode {color: #000}#resultEnd.visible {display: block;position: absolute;z-index: 10;top: 30%;left: 40vw;color: #FFF;animation: popUp 1s ease-out forwards;}@keyframes popUp {0% {transform: scale(0);opacity: 0;}95% {transform: scale(4);opacity: 1;}100% {transform: scale(2);opacity: 1;}}#imageContainer {position: fixed;top: 0;left: 0;width: 100%;height: 100%;pointer-events: none;overflow: hidden;}.flying-image {position: absolute;animation: fly 200s linear infinite;max-width: 20vw;animation-direction: alternate;animation-timing-function: ease-in-out;}@keyframes fly {0% {transform: translateX(-100vw) rotate(-20deg);}100% {transform: translateX(200vw) rotate(20deg);}}#version {margin-left: 5px;}</style></head><body><button type=\"button\" onclick=\"exitApp()\" class=\"exit-button\" id=\"exitButton\">Exit</button>    <button type=\"button\" onclick=\"toggleDarkMode()\" class=\"dark-mode-toggle\" id=\"darkModeToggle\">&#x2600;</button>    <h1 class=\"title\">mdown <p id=\"version\"></p></h1><form class=\"mangaForm\"><label class=\"urlInput\" for=\"urlInput\">Enter Manga URL:</label><input type=\"text\" id=\"urlInput\" name=\"url\" required><button type=\"button\" class=\"download\" onclick=\"downloadManga()\">Download</button></form><div id=\"resultMessage\"></div><div id=\"resultEnd\"></div><div id=\"imageContainer\"></div><audio id=\"downloadedMusic\" src=\"__get__?path=rambling_pleat\" loop></audio><audio id=\"downloadMusic\" src=\"__get__?path=system_haven\" loop></audio><script>fetch(\'__version__\').then(response => {if (!response.ok) {throw new Error(\'Network response was not ok\');}return response.text();}).then(text => {document.getElementById(\'version\').textContent = `v${text}`;}).catch(error => {console.error(\'There was a problem fetching the text:\', error);});function delay(time) {return new Promise(resolve => setTimeout(resolve, time));}let id = \"\";let isPostRequestInProgress = false;let isPostRequestInProgress_tmp = true;let images = [];let times = 0;let end = false;function sleep(ms) {return new Promise(resolve => setTimeout(resolve, ms));}function clickHandler(event) {end = true;const resultEndDiv = document.getElementById(\'resultEnd\');resultEndDiv.classList.remove(\'visible\');const downloadedMusic = document.getElementById(\'downloadedMusic\');downloadedMusic.pause();downloadedMusic.currentTime = 0;const imageContainer = document.getElementById(\'imageContainer\');imageContainer.innerHTML = \'\';}function createFlyingImage() {const imageContainer = document.getElementById(\'imageContainer\');const img = document.createElement(\'img\');console.log(images.length);var randomIndex = Math.floor(Math.random() * images.length);var randomImage = images[randomIndex];img.src = \"data:image/png;base64,\" + images[randomIndex];img.classList.add(\'flying-image\');img.style.zIndex = Math.random() >= 0.5 ? \"1\" : \"20\";const initialPosition = \"0vw\";img.style.left = initialPosition;img.style.top = `${(Math.random() * 100) - 25}vh`;img.style.animationDuration = `${5 + Math.random() * 20}s`;imageContainer.appendChild(img);img.addEventListener(\'animationiteration\', () => {const newInitialPosition = initialPosition === \'-100vw\' ? \'200vw\' : \'-100vw\';img.style.left = newInitialPosition;});}async function get_confetti() {try {const response = await fetch(\'__confetti__\');if (!response.ok) {throw new Error(\'Network response was not ok\');}const data = await response.json();images = data.images;} catch (error) {console.error(\'Error:\', error);throw error;}}function start_confetti_event() {if (end) {return;}times += 1;const randomInterval = Math.random() * (2000 - 500) + 500;setTimeout(() => {if (times % 10 === 0) {start_confetti_big();} else {start_confetti();}start_confetti_event();}, randomInterval);}function start_confetti() {confetti({particleCount: 250,spread: 100,origin: { y: Math.random(), x: Math.random() }});}function start_confetti_big() {confetti({particleCount: 250,spread: 100,origin: { y: Math.random(), x: Math.random() }});confetti({particleCount: 250,spread: 100,origin: { y: Math.random(), x: Math.random() }});confetti({particleCount: 250,spread: 100,origin: { y: Math.random(), x: Math.random() }});}function downloadManga() {id = generateRandomId(10);if (isPostRequestInProgress) {alert(\'A download is already in progress. Please wait.\');return;}isPostRequestInProgress = true;const downloadMusic = document.getElementById(\'downloadMusic\');downloadMusic.play().catch(error => console.log(\'Error playing sound:\', error));var mangaUrl = document.getElementById(\'urlInput\').value;var encodedUrl = encodeURIComponent(mangaUrl);var url = \"http://127.0.0.1:8080/manga\";fetch(url + \"?url=\" + encodedUrl + \"&id=\" + id, {method: \'POST\',headers: {\'Content-Type\': \'application/json\',},}).then(response => {if (!response.ok) {throw new Error(\'Network response was not ok\');}return response.json();}).then(async result => {const resultMessageDiv = document.getElementById(\'resultMessage\');if (result.status == \"ok\") {end = false;console.log(\'Scanlation Groups:\', result.scanlation_groups);console.log(\'Files:\', result.files);console.log(\'Manga Name:\', result.name);console.log(\'Status:\', result.status);resultMessageDiv.innerHTML = \"<p id=\\\'result\\\'>Download successful!</p>\";if (result.files && result.files.length > 0) {resultMessageDiv.innerHTML += \"<p id=\\\'result\\\'>Downloaded Files:</p>\";resultMessageDiv.innerHTML += \"<ul id=\\\'result\\\'>\";result.files.forEach(file => {resultMessageDiv.innerHTML += \"<li id=\\\'result\\\'>\" + file + \"</li>\";});resultMessageDiv.innerHTML += \"</ul>\";}if (result.scanlation_groups && result.scanlation_groups.length > 0) {resultMessageDiv.innerHTML += \"<p id=\\\'result\\\'>Scanlation Groups:</p>\";resultMessageDiv.innerHTML += \"<ul id=\\\'result\\\'>\";result.scanlation_groups.forEach(group => {resultMessageDiv.innerHTML += \"<li id=\\\'result\\\'>\" + group + \"</li>\";});resultMessageDiv.innerHTML += \"</ul>\";}isPostRequestInProgress = false;isPostRequestInProgress_tmp = true;await get_confetti();const resultEnd = document.getElementById(\'resultEnd\');resultEnd.innerHTML = `<p>${result.name} has been downloaded</p>`;const downloadMusic = document.getElementById(\'downloadMusic\');downloadMusic.pause();downloadMusic.currentTime = 0;const downloadedMusic = document.getElementById(\'downloadedMusic\');downloadedMusic.play().catch(error => console.log(\'Error playing sound:\', error));const body = document.body;setTimeout(() => {body.style.transition = \"0s\";body.style.backgroundColor = \"#FFF\";}, 100);setTimeout(() => {body.style.backgroundColor = \"#cfff01\";}, 200);setTimeout(() => {body.style.backgroundColor = \"#2da657\";}, 300);setTimeout(() => {body.style.backgroundColor = \"#0763cc\";}, 400);setTimeout(() => {body.style.backgroundColor = \"#cc074c\";}, 500);setTimeout(() => {body.style.backgroundColor = \"#121212\";body.style.transition = \"background-color 0.5s\";confetti({particleCount: 250,spread: 100,origin: { y: 0.6 }});confetti({particleCount: 250,spread: 100,origin: { y: 0.8, x: 0.25 }});confetti({particleCount: 250,spread: 100,origin: { y: 0.8, x: 0.75 }});start_confetti_event();}, 900);showResultEnd();for (let i = 0; i < 10; i++) {createFlyingImage();}document.addEventListener(\'click\', clickHandler);}}).catch(error => {console.error(\'Error during POST request:\', error);document.getElementById(\'resultMessage\').innerHTML = \"<p id=\'result\'>Error during download. Please try again.<p>\";isPostRequestInProgress = false;isPostRequestInProgress_tmp = true;});}function fetchWhilePostInProgress() {var parsed = 0;var total = 0;var current = 0;setInterval(async () => {if (!isPostRequestInProgress) {return;}if (isPostRequestInProgress_tmp) {await delay(1000);isPostRequestInProgress_tmp = false;}fetch(\"http://127.0.0.1:8080/manga-result?id=\" + id).then(response => response.json()).then(async result => {if (result.status === \"ok\") {const resultMessageDiv = document.getElementById(\'resultMessage\');resultMessageDiv.innerHTML = `<p id=\'result\'>In Progress!</p><p id=\'result\'>Parsed chapters: ${result.current_chapter_parsed}/${result.current_chapter_parsed_max}</p>${result.current ? `<p id=\'result\'>Current chapter: ${result.current}</p>` : \'\'}`;let progressElement = document.getElementById(\'progress\');if (!progressElement) {progressElement = document.createElement(\'div\');progressElement.id = \'progress\';resultMessageDiv.appendChild(progressElement);}if (result.current_page && result.current_page_max) {for (let i = current; i <= result.current_page; i++) {let progressHTML = `<p id=\'result\'>${\"#\".repeat(i)}  ${i}|${result.current_page_max}</p>`;progressElement.innerHTML = progressHTML;await delay(10);}current = result.current_page;}if (result.current_percent && result.current_size && result.current_size_max) {resultMessageDiv.innerHTML += `<p id=\'result\'>${result.current_percent} | ${result.current_size}mb/${result.current_size_max}mb</p>`;}if (result.files && result.files.length > 0) {let filesHTML = `<p id=\'result\'>Downloaded Files:</p><ul id=\'result\'>${result.files.map(file => `<li id=\'result\'>${file}</li>`).join(\'\')}</ul>`;resultMessageDiv.innerHTML += filesHTML;}if (result.scanlation_groups && result.scanlation_groups.length > 0) {let groupsHTML = `<p id=\'result\'>Scanlation Groups:</p><ul id=\'result\'>${result.scanlation_groups.map(group => `<li id=\'result\'>${group}</li>`).join(\'\')}</ul>`;resultMessageDiv.innerHTML += groupsHTML;}parsed = result.current_chapter_parsed;    total = result.current_page_max;}}).catch(error => {console.error(\'Error during GET request:\', error);});}, 500);}fetchWhilePostInProgress();function showResultEnd() {const resultEndDiv = document.getElementById(\'resultEnd\');resultEndDiv.classList.add(\'visible\');}function generateRandomId(length) {const CHARSET = \'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789\';let id = \'\';for (let i = 0; i < length; i++) {const randomIndex = Math.floor(Math.random() * CHARSET.length);id += CHARSET.charAt(randomIndex);}return id;}function toggleDarkMode() {const body = document.body;body.classList.toggle(\'dark-mode\');const button = document.getElementById(\'darkModeToggle\');const exit_button = document.getElementById(\'exitButton\');if (body.classList.contains(\'dark-mode\')) {button.innerHTML = \'\\u{1F319}\';button.style.backgroundColor = \"#000\";button.style.color = \"#FFF\";exit_button.style.backgroundColor = \"#000\";exit_button.style.color = \"#FFF\";} else {button.innerHTML = \'\\u{2600}\';button.style.backgroundColor = \"#FFF\";button.style.color = \"#000\";exit_button.style.backgroundColor = \"#FFF\";exit_button.style.color = \"#000\";}}function exitApp() {fetch(\"http://127.0.0.1:8080/end\", {method: \'GET\'}).then(response => {if (response.ok) {window.close();} else {console.error(\'Failed to send exit request\');}}).catch(error => {console.error(\'Error while sending exit request:\', error);});}!function (t, e) { !function t(e, n, a, i) { var o = !!(e.Worker && e.Blob && e.Promise && e.OffscreenCanvas && e.OffscreenCanvasRenderingContext2D && e.HTMLCanvasElement && e.HTMLCanvasElement.prototype.transferControlToOffscreen && e.URL && e.URL.createObjectURL); function r() { } function l(t) { var a = n.exports.Promise, i = void 0 !== a ? a : e.Promise; return \"function\" == typeof i ? new i(t) : (t(r, r), null) } var c, s, u, d, f, h, m, g, b, v = (u = Math.floor(1e3 / 60), d = {}, f = 0, \"function\" == typeof requestAnimationFrame && \"function\" == typeof cancelAnimationFrame ? (c = function (t) { var e = Math.random(); return d[e] = requestAnimationFrame((function n(a) { f === a || f + u - 1 < a ? (f = a, delete d[e], t()) : d[e] = requestAnimationFrame(n) })), e }, s = function (t) { d[t] && cancelAnimationFrame(d[t]) }) : (c = function (t) { return setTimeout(t, u) }, s = function (t) { return clearTimeout(t) }), { frame: c, cancel: s }), p = (g = {}, function () { if (h) return h; if (!a && o) { var e = [\"var CONFETTI, SIZE = {}, module = {};\", \"(\" + t.toString() + \")(this, module, true, SIZE);\", \"onmessage = function(msg) {\", \"  if (msg.data.options) {\", \"CONFETTI(msg.data.options).then(function () {\", \"  if (msg.data.callback) {\", \"postMessage({ callback: msg.data.callback });\", \"  }\", \"});\", \"  } else if (msg.data.reset) {\", \"CONFETTI.reset();\", \"  } else if (msg.data.resize) {\", \"SIZE.width = msg.data.resize.width;\", \"SIZE.height = msg.data.resize.height;\", \"  } else if (msg.data.canvas) {\", \"SIZE.width = msg.data.canvas.width;\", \"SIZE.height = msg.data.canvas.height;\", \"CONFETTI = module.exports.create(msg.data.canvas);\", \"  }\", \"}\"].join(\"\\n\"); try { h = new Worker(URL.createObjectURL(new Blob([e]))) } catch (t) { return void 0 !== typeof console && \"function\" == typeof console.warn && console.warn(\"🎊 Could not load worker\", t), null } !function (t) { function e(e, n) { t.postMessage({ options: e || {}, callback: n }) } t.init = function (e) { var n = e.transferControlToOffscreen(); t.postMessage({ canvas: n }, [n]) }, t.fire = function (n, a, i) { if (m) return e(n, null), m; var o = Math.random().toString(36).slice(2); return m = l((function (a) { function r(e) { e.data.callback === o && (delete g[o], t.removeEventListener(\"message\", r), m = null, i(), a()) } t.addEventListener(\"message\", r), e(n, o), g[o] = r.bind(null, { data: { callback: o } }) })) }, t.reset = function () { for (var e in t.postMessage({ reset: !0 }), g) g[e](), delete g[e] } }(h) } return h }), y = { particleCount: 50, angle: 90, spread: 45, startVelocity: 45, decay: .9, gravity: 1, drift: 0, ticks: 200, x: .5, y: .5, shapes: [\"square\", \"circle\"], zIndex: 100, colors: [\"#26ccff\", \"#a25afd\", \"#ff5e7e\", \"#88ff5a\", \"#fcff42\", \"#ffa62d\", \"#ff36ff\"], disableForReducedMotion: !1, scalar: 1 }; function M(t, e, n) { return function (t, e) { return e ? e(t) : t }(t && null != t[e] ? t[e] : y[e], n) } function w(t) { return t < 0 ? 0 : Math.floor(t) } function x(t) { return parseInt(t, 16) } function C(t) { return t.map(k) } function k(t) { var e = String(t).replace(/[^0-9a-f]/gi, \"\"); return e.length < 6 && (e = e[0] + e[0] + e[1] + e[1] + e[2] + e[2]), { r: x(e.substring(0, 2)), g: x(e.substring(2, 4)), b: x(e.substring(4, 6)) } } function I(t) { t.width = document.documentElement.clientWidth, t.height = document.documentElement.clientHeight } function S(t) { var e = t.getBoundingClientRect(); t.width = e.width, t.height = e.height } function T(t, e, n, o, r) { var c, s, u = e.slice(), d = t.getContext(\"2d\"), f = l((function (e) { function l() { c = s = null, d.clearRect(0, 0, o.width, o.height), r(), e() } c = v.frame((function e() { !a || o.width === i.width && o.height === i.height || (o.width = t.width = i.width, o.height = t.height = i.height), o.width || o.height || (n(t), o.width = t.width, o.height = t.height), d.clearRect(0, 0, o.width, o.height), u = u.filter((function (t) { return function (t, e) { e.x += Math.cos(e.angle2D) * e.velocity + e.drift, e.y += Math.sin(e.angle2D) * e.velocity + e.gravity, e.wobble += e.wobbleSpeed, e.velocity *= e.decay, e.tiltAngle += .1, e.tiltSin = Math.sin(e.tiltAngle), e.tiltCos = Math.cos(e.tiltAngle), e.random = Math.random() + 2, e.wobbleX = e.x + 10 * e.scalar * Math.cos(e.wobble), e.wobbleY = e.y + 10 * e.scalar * Math.sin(e.wobble); var n = e.tick++ / e.totalTicks, a = e.x + e.random * e.tiltCos, i = e.y + e.random * e.tiltSin, o = e.wobbleX + e.random * e.tiltCos, r = e.wobbleY + e.random * e.tiltSin; return t.fillStyle = \"rgba(\" + e.color.r + \", \" + e.color.g + \", \" + e.color.b + \", \" + (1 - n) + \")\", t.beginPath(), \"circle\" === e.shape ? t.ellipse ? t.ellipse(e.x, e.y, Math.abs(o - a) * e.ovalScalar, Math.abs(r - i) * e.ovalScalar, Math.PI / 10 * e.wobble, 0, 2 * Math.PI) : function (t, e, n, a, i, o, r, l, c) { t.save(), t.translate(e, n), t.rotate(o), t.scale(a, i), t.arc(0, 0, 1, r, l, c), t.restore() }(t, e.x, e.y, Math.abs(o - a) * e.ovalScalar, Math.abs(r - i) * e.ovalScalar, Math.PI / 10 * e.wobble, 0, 2 * Math.PI) : (t.moveTo(Math.floor(e.x), Math.floor(e.y)), t.lineTo(Math.floor(e.wobbleX), Math.floor(i)), t.lineTo(Math.floor(o), Math.floor(r)), t.lineTo(Math.floor(a), Math.floor(e.wobbleY))), t.closePath(), t.fill(), e.tick < e.totalTicks }(d, t) })), u.length ? c = v.frame(e) : l() })), s = l })); return { addFettis: function (t) { return u = u.concat(t), f }, canvas: t, promise: f, reset: function () { c && v.cancel(c), s && s() } } } function E(t, n) { var a, i = !t, r = !!M(n || {}, \"resize\"), c = M(n, \"disableForReducedMotion\", Boolean), s = o && !!M(n || {}, \"useWorker\") ? p() : null, u = i ? I : S, d = !(!t || !s) && !!t.__confetti_initialized, f = \"function\" == typeof matchMedia && matchMedia(\"(prefers-reduced-motion)\").matches; function h(e, n, i) { for (var o, r, l, c, s, d = M(e, \"particleCount\", w), f = M(e, \"angle\", Number), h = M(e, \"spread\", Number), m = M(e, \"startVelocity\", Number), g = M(e, \"decay\", Number), b = M(e, \"gravity\", Number), v = M(e, \"drift\", Number), p = M(e, \"colors\", C), y = M(e, \"ticks\", Number), x = M(e, \"shapes\"), k = M(e, \"scalar\"), I = function (t) { var e = M(t, \"origin\", Object); return e.x = M(e, \"x\", Number), e.y = M(e, \"y\", Number), e }(e), S = d, E = [], F = t.width * I.x, N = t.height * I.y; S--;)E.push((o = { x: F, y: N, angle: f, spread: h, startVelocity: m, color: p[S % p.length], shape: x[(c = 0, s = x.length, Math.floor(Math.random() * (s - c)) + c)], ticks: y, decay: g, gravity: b, drift: v, scalar: k }, r = void 0, l = void 0, r = o.angle * (Math.PI / 180), l = o.spread * (Math.PI / 180), { x: o.x, y: o.y, wobble: 10 * Math.random(), wobbleSpeed: Math.min(.11, .1 * Math.random() + .05), velocity: .5 * o.startVelocity + Math.random() * o.startVelocity, angle2D: -r + (.5 * l - Math.random() * l), tiltAngle: (.5 * Math.random() + .25) * Math.PI, color: o.color, shape: o.shape, tick: 0, totalTicks: o.ticks, decay: o.decay, drift: o.drift, random: Math.random() + 2, tiltSin: 0, tiltCos: 0, wobbleX: 0, wobbleY: 0, gravity: 3 * o.gravity, ovalScalar: .6, scalar: o.scalar })); return a ? a.addFettis(E) : (a = T(t, E, u, n, i)).promise } function m(n) { var o = c || M(n, \"disableForReducedMotion\", Boolean), m = M(n, \"zIndex\", Number); if (o && f) return l((function (t) { t() })); i && a ? t = a.canvas : i && !t && (t = function (t) { var e = document.createElement(\"canvas\"); return e.style.position = \"fixed\", e.style.top = \"0px\", e.style.left = \"0px\", e.style.pointerEvents = \"none\", e.style.zIndex = t, e }(m), document.body.appendChild(t)), r && !d && u(t); var g = { width: t.width, height: t.height }; function b() { if (s) { var e = { getBoundingClientRect: function () { if (!i) return t.getBoundingClientRect() } }; return u(e), void s.postMessage({ resize: { width: e.width, height: e.height } }) } g.width = g.height = null } function v() { a = null, r && e.removeEventListener(\"resize\", b), i && t && (document.body.removeChild(t), t = null, d = !1) } return s && !d && s.init(t), d = !0, s && (t.__confetti_initialized = !0), r && e.addEventListener(\"resize\", b, !1), s ? s.fire(n, g, v) : h(n, g, v) } return m.reset = function () { s && s.reset(), a && a.reset() }, m } function F() { return b || (b = E(null, { useWorker: !0, resize: !0 })), b } n.exports = function () { return F().apply(this, arguments) }, n.exports.reset = function () { F().reset() }, n.exports.create = E }(function () { return void 0 !== t ? t : \"undefined\" != typeof self ? self : this || {} }(), e, !1), t.confetti = e.exports }(window, {});</script></body></html>"
        )
    }
}

/// Generates an HTML page for a 404 error.
///
/// # Returns
/// - A `String` containing the HTML content for a "404 Error - Page Not Found" page.
/// - The page includes basic styling and a message explaining that the requested page is unavailable.
/// - Provides a link to return to the home page.
///
/// # Example
/// ```rust
/// let error_page = get_error_html();
/// println!("{}", error_page); // Outputs the 404 error page HTML.
/// ```
fn get_error_html() -> String {
    String::from(
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"UTF-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"><title>404 Error - Page Not Found</title><style>body {font-family: Arial, sans-serif;background-color: #f7f7f7;color: #333;margin: 0;padding: 0;text-align: center;}.container {position: absolute;top: 50%;left: 50%;transform: translate(-50%, -50%);}h1 {font-size: 36px;margin-bottom: 20px;}p {font-size: 18px;margin-bottom: 20px;}a {color: #007bff;text-decoration: none;}a:hover {text-decoration: underline;}</style></head><body><div class=\"container\"><h1>404 Error - Page Not Found</h1><p>The page you are looking for might have been removed, had its name changed, or is temporarily unavailable.</p><p>Go back to <a href=\"/\">home page</a>.</p></div></body></html>"
    )
}

/// Starts a web server that listens on `127.0.0.1:8080` and handles incoming requests.
///
/// # Returns
/// - `Ok(())` if the server starts successfully and continues running.
/// - Returns an `MdownError` if the server encounters issues such as a failure to bind the listener.
///
/// # Functionality
/// - The server binds to the local address `127.0.0.1:8080`.
/// - Attempts to open the URL `http://127.0.0.1:8080/` in the default web browser.
/// - Listens for incoming TCP connections and handles them asynchronously using the `handle_client` function.
///
/// # Example
/// ```rust
/// if let Err(err) = web().await {
///     eprintln!("Error starting the server: {}", err);
/// }
/// ```
async fn web() -> Result<(), MdownError> {
    let listener = match TcpListener::bind("127.0.0.1:8080") {
        Ok(listener) => listener,
        Err(err) => {
            return Err(MdownError::IoError(err, String::new(), 11306));
        }
    };
    log!("Server listening on 127.0.0.1:8080");

    let url = "http://127.0.0.1:8080/";
    if let Err(err) = webbrowser::open(url) {
        eprintln!("Error opening web browser: {}", err);
    }

    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                tokio::spawn(async { handle_client(stream).await });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}

/// Initializes the server and sets up a Ctrl+C handler to gracefully exit when the user interrupts the process.
///
/// # Returns
/// - `Ok(())` if the server starts successfully and handles a Ctrl+C interruption.
/// - Returns an `MdownError` if setting up the Ctrl+C handler fails, or if any error occurs while starting the server.
///
/// # Functionality
/// - Sets a handler for the `Ctrl+C` signal to log messages and clean up resources when the process is interrupted.
/// - Calls the `web` function to start the web server.
///
/// # Example
/// ```rust
/// if let Err(err) = start().await {
///     eprintln!("Error starting the server: {}", err);
/// }
/// ```
pub(crate) async fn start() -> Result<(), MdownError> {
    let handler = ctrlc::set_handler(|| {
        log!("[user] Ctrl+C received! Exiting...");
        log!("[web] Closing server");

        match utils::remove_cache() {
            Ok(()) => (),
            Err(err) => {
                handle_error!(&err, String::from("ctrl_handler"));
            }
        }
        std::process::exit(0);
    });
    match handler {
        Ok(()) => (),
        Err(err) => {
            return Err(
                MdownError::CustomError(
                    format!("Failed setting up ctrl handler, {}", err),
                    String::from("CTRL_handler"),
                    11307
                )
            );
        }
    }
    web().await
}
