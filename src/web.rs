use std::{ collections::HashMap, io::{ Read, Write }, net::TcpListener };
use percent_encoding::{ percent_decode_str, percent_encode, NON_ALPHANUMERIC };
use serde_json::{ self, Value };
use webbrowser;
use tracing::info;

use crate::{
    utils,
    resolute,
    getter,
    resolute::{
        DOWNLOADED,
        SCANLATION_GROUPS,
        MANGA_NAME,
        CURRENT_CHAPTER,
        CURRENT_PAGE,
        CURRENT_PAGE_MAX,
        CURRENT_SIZE,
        CURRENT_SIZE_MAX,
        CURRENT_PERCENT,
    },
    ARGS,
    MANGA_ID,
};

fn decode(url: &str) -> String {
    percent_decode_str(&url).decode_utf8_lossy().to_string()
}

pub(crate) fn encode(url: &str) -> String {
    percent_encode(url.as_bytes(), NON_ALPHANUMERIC).to_string()
}

async fn resolve_web_download(url: &str, handle_id: String) -> String {
    let mut manga_name = String::from("!");
    if let Some(id) = utils::resolve_regex(&url) {
        let id: &str = id.as_str();
        *MANGA_ID.lock().unwrap() = id.to_string();
        info!("@{} Found {}", handle_id, id);
        match getter::get_manga_json(id).await {
            Ok(manga_name_json) => {
                let json_value = serde_json::from_str(&manga_name_json).unwrap();
                if let Value::Object(obj) = json_value {
                    manga_name = resolute::resolve(obj, id, Some(handle_id)).await;
                } else {
                    eprintln!("Unexpected JSON value");
                    return String::from("!");
                }
            }
            Err(_) => {}
        }
    } else {
        info!("@{} Didn't find any id", handle_id);
    }
    if manga_name.eq("!") {
        String::from("!")
    } else {
        let downloaded_files = DOWNLOADED.lock().unwrap().clone();
        let scanlation = SCANLATION_GROUPS.lock().unwrap().clone();

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
                    scanlation.into_iter().map(serde_json::Value::String).collect()
                ),
            ),
        ]
            .iter()
            .cloned()
            .collect();

        serde_json::to_string(&response_map).expect("Failed to serialize JSON")
    }
}

async fn handle_client(mut stream: std::net::TcpStream) {
    let addr = stream.peer_addr().expect("Failed to get peer address");
    info!("Connection from: {}", addr);
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let request = String::from_utf8_lossy(&buffer[..]);

    let response = match parse_request(&request) {
        Some((Some(url), Some(_params), handle_id)) => {
            let json = resolve_web_download(&url, handle_id.clone()).await;
            format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}", json)
        }
        Some((None, _, _)) => {
            format!("HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\n\r\nInvalid Request")
        }
        Some((Some(url), None, _)) => {
            if url == String::from("main") {
                format!(
                    "{}{}",
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n",
                    "<!DOCTYPE html>\n<html lang=\"en\">\n\n<head>\n    <meta charset=\"UTF-8\">\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n    <title>mdown v0.7.1</title>\n\n    <style>\n        body {\n            font-family: Arial, sans-serif;\n            background-color: #121212;\n            color: #fff;\n            margin: 0;\n            padding: 0;\n            box-sizing: border-box;\n            transition: background-color 0.5s;\n        }\n\n        body.dark-mode {\n            background-color: #fff;\n            color: #121212;\n        }\n\n        .title {\n            text-align: center;\n            color: inherit;\n        }\n\n        .mangaForm {\n            max-width: 400px;\n            margin: 20px auto;\n            background-color: #272727;\n            padding: 20px;\n            border-radius: 8px;\n            box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);\n        }\n\n        .mangaForm.dark-mode {\n            color: #FFF;\n            background-color: #FFF;\n        }\n\n        .urlInput {\n            display: block;\n            margin-bottom: 8px;\n            color: #fff;\n        }\n\n        .urlInput.dark-mode {\n            color: #000;\n        }\n\n        input {\n            width: 100%;\n            padding: 10px;\n            margin-bottom: 16px;\n            box-sizing: border-box;\n            border: 1px solid #555;\n            border-radius: 4px;\n            background-color: #333;\n            color: #fff;\n        }\n\n        .dark-mode-toggle {\n            background-color: #FFF;\n            color: #000;\n            padding: 10px 15px;\n            border: none;\n            border-radius: 50%;\n            cursor: pointer;\n            position: fixed;\n            top: 20px;\n            right: 20px;\n            font-size: 20px;\n        }\n\n        .dark-mode-toggle:hover {\n            background-color: grey;\n        }\n\n        .download {\n            background-color: #4caf50;\n            color: #fff;\n            padding: 10px 15px;\n            border: none;\n            border-radius: 4px;\n            cursor: pointer;\n        }\n\n        .download:hover {\n            background-color: #45a049;\n        }\n\n        #resultMessage {\n            margin: 20px auto;\n            max-width: 600px;\n            background-color: #272727;\n            padding: 50px;\n            border-radius: 8px;\n            box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);\n        }\n\n        ul {\n            list-style-type: none;\n            padding: 0;\n        }\n\n        li {\n            margin-bottom: 8px;\n        }\n\n        #result {\n            color: #FFF;\n        }\n    </style>\n</head>\n\n<body>\n\n    <h1 class=\"title\">mdown v0.7.1</h1>\n\n    <form class=\"mangaForm\">\n        <label class=\"urlInput\" for=\"urlInput\">Enter Manga URL:</label>\n        <input type=\"text\" id=\"urlInput\" name=\"url\" required>\n        <button type=\"button\" class=\"download\" onclick=\"downloadManga()\">Download</button>\n    </form>\n    <button type=\"button\" onclick=\"toggleDarkMode()\" class=\"dark-mode-toggle\" id=\"darkModeToggle\">&#x2600;</button>\n\n    <div id=\"resultMessage\"></div>\n\n    <script>\n        function delay(time) {\n            return new Promise(resolve => setTimeout(resolve, time));\n        }\n\n        let id = \"\";\n        let isPostRequestInProgress = false;\n        let isPostRequestInProgress_tmp = true;\n\n        function downloadManga() {\n            id = generateRandomId(10);\n            if (isPostRequestInProgress) {\n                alert(\'A download is already in progress. Please wait.\');\n                return;\n            }\n\n            isPostRequestInProgress = true;\n\n            var mangaUrl = document.getElementById(\'urlInput\').value;\n            var encodedUrl = encodeURIComponent(mangaUrl);\n            var url = \"http://127.0.0.1:8080/manga\";\n\n            fetch(url + \"?url=\" + encodedUrl + \"&id=\" + id, {\n                method: \'POST\',\n                headers: {\n                    \'Content-Type\': \'application/json\',\n                },\n            })\n                .then(response => {\n                    if (!response.ok) {\n                        throw new Error(\'Network response was not ok\');\n                    }\n                    return response.json();\n                })\n                .then(result => {\n                    const resultMessageDiv = document.getElementById(\'resultMessage\');\n                    console.log(\'Scanlation Groups:\', result.scanlation_groups);\n                    console.log(\'Files:\', result.files);\n                    console.log(\'Manga Name:\', result.name);\n                    console.log(\'Status:\', result.status);\n\n                    resultMessageDiv.innerHTML = \"<p id=\\'result\\'>Download successful!</p>\";\n\n                    if (result.files && result.files.length > 0) {\n                        resultMessageDiv.innerHTML += \"<p id=\\'result\\'>Downloaded Files:</p>\";\n                        resultMessageDiv.innerHTML += \"<ul id=\\'result\\'>\";\n                        result.files.forEach(file => {\n                            resultMessageDiv.innerHTML += \"<li id=\\'result\\'>\" + file + \"</li>\";\n                        });\n                        resultMessageDiv.innerHTML += \"</ul>\";\n                    }\n\n                    if (result.scanlation_groups && result.scanlation_groups.length > 0) {\n                        resultMessageDiv.innerHTML += \"<p id=\\'result\\'>Scanlation Groups:</p>\";\n                        resultMessageDiv.innerHTML += \"<ul id=\\'result\\'>\";\n                        result.scanlation_groups.forEach(group => {\n                            resultMessageDiv.innerHTML += \"<li id=\\'result\\'>\" + group + \"</li>\";\n                        });\n                        resultMessageDiv.innerHTML += \"</ul>\";\n\n                    }\n                    isPostRequestInProgress = false;\n                    isPostRequestInProgress_tmp = true;\n                })\n                .catch(error => {\n                    console.error(\'Error during POST request:\', error);\n                    document.getElementById(\'resultMessage\').innerHTML = \"<p id=\'result\'>Error during download. Please try again.<p>\";\n\n                    isPostRequestInProgress = false;\n                    isPostRequestInProgress_tmp = true;\n                });\n        }\n\n        function fetchWhilePostInProgress() {\n            setInterval(() => {\n                if (!isPostRequestInProgress) {\n                    return;\n                }\n                if (isPostRequestInProgress_tmp) {\n                    delay(1000);\n                    isPostRequestInProgress_tmp = false\n                }\n\n                fetch(\"http://127.0.0.1:8080/manga-result?id=\" + id)\n                    .then(response => response.json())\n                    .then(result => {\n                        const resultMessageDiv = document.getElementById(\'resultMessage\');\n                        console.log(\'Scanlation Groups:\', result.scanlation_groups);\n                        console.log(\'Files:\', result.files);\n                        console.log(\'Current chapter:\', result.current);\n                        console.log(\'Manga Name:\', result.name);\n                        console.log(\'Status:\', result.status);\n\n                        resultMessageDiv.innerHTML = \"<p id=\\'result\\'>In Progress!</p>\";\n                        if (result.current && result.current.length > 0) {\n                            resultMessageDiv.innerHTML += \"<p id=\\'result\\'>Current chapter: \" + result.current + \"</p>\";\n                        }\n                        if (result.current_page && result.current_page.length > 0 && result.current_page_max && result.current_page_max.length > 0) {\n                            resultMessageDiv.innerHTML += \"<p id=\\'result\\'>\" + \"#\".repeat(parseInt(result.current_page)) + \"  \" + result.current_page + \"|\" + result.current_page_max + \"</p>\";\n                        }\n                        if (result.current_percent && result.current_percent.length > 0) {\n                            resultMessageDiv.innerHTML += \"<p id=\\'result\\'>\" + result.current_percent + \" | \" + result.current_size + \"mb/\" + result.current_size_max + \"mb</p>\";\n                        }\n\n                        if (result.files && result.files.length > 0) {\n                            resultMessageDiv.innerHTML += \"<p id=\\'result\\'>Downloaded Files:</p>\";\n                            resultMessageDiv.innerHTML += \"<ul id=\\'result\\'>\";\n                            result.files.forEach(file => {\n                                resultMessageDiv.innerHTML += \"<li id=\\'result\\'>\" + file + \"</li>\";\n                            });\n                            resultMessageDiv.innerHTML += \"</ul>\";\n                        }\n\n                        if (result.scanlation_groups && result.scanlation_groups.length > 0) {\n                            resultMessageDiv.innerHTML += \"<p id=\\'result\\'>Scanlation Groups:</p>\";\n                            resultMessageDiv.innerHTML += \"<ul id=\\'result\\'>\";\n                            result.scanlation_groups.forEach(group => {\n                                resultMessageDiv.innerHTML += \"<li id=\\'result\\'>\" + group + \"</li>\";\n                            });\n                            resultMessageDiv.innerHTML += \"</ul>\";\n\n                        }\n                    })\n                    .catch(error => {\n                        console.error(\'Error during GET request:\', error);\n                    });\n            }, 500);\n        }\n        fetchWhilePostInProgress();\n\n        function generateRandomId(length) {\n            const CHARSET = \'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789\';\n            let id = \'\';\n\n            for (let i = 0; i < length; i++) {\n                const randomIndex = Math.floor(Math.random() * CHARSET.length);\n                id += CHARSET.charAt(randomIndex);\n            }\n\n            return id;\n        }\n\n        function toggleDarkMode() {\n            const body = document.body;\n            body.classList.toggle(\'dark-mode\');\n            const button = document.getElementById(\'darkModeToggle\');\n\n            if (body.classList.contains(\'dark-mode\')) {\n                button.innerHTML = \'\u{1F319}\';\n                button.style.backgroundColor = \"#000\";\n                button.style.color = \"#FFF\";\n            } else {\n                button.innerHTML = \'\u{2600}\';\n                button.style.backgroundColor = \"#FFF\";\n                button.style.color = \"#000\";\n            }\n        }\n\n    </script>\n\n</body>\n\n</html>"
                )
            } else if url == String::from("progress") {
                let downloaded_files = DOWNLOADED.lock().unwrap().clone();
                let scanlation = SCANLATION_GROUPS.lock().unwrap().clone();
                let response_map: HashMap<&str, serde_json::Value> = [
                    ("status", serde_json::Value::String("ok".to_string())),
                    (
                        "name",
                        serde_json::Value::String(unsafe {
                            MANGA_NAME.lock().unwrap_unchecked().to_string()
                        }),
                    ),
                    (
                        "current",
                        serde_json::Value::String(CURRENT_CHAPTER.lock().unwrap().to_string()),
                    ),
                    (
                        "current_page",
                        serde_json::Value::String(CURRENT_PAGE.lock().unwrap().to_string()),
                    ),
                    (
                        "current_page_max",
                        serde_json::Value::String(CURRENT_PAGE_MAX.lock().unwrap().to_string()),
                    ),
                    (
                        "current_percent",
                        serde_json::Value::String(
                            format!("{:.2}", CURRENT_PERCENT.lock().unwrap())
                        ),
                    ),
                    (
                        "current_size",
                        serde_json::Value::String(format!("{:.2}", CURRENT_SIZE.lock().unwrap())),
                    ),
                    (
                        "current_size_max",
                        serde_json::Value::String(
                            format!("{:.2}", CURRENT_SIZE_MAX.lock().unwrap())
                        ),
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
                            scanlation.into_iter().map(serde_json::Value::String).collect()
                        ),
                    ),
                ]
                    .iter()
                    .cloned()
                    .collect();
                let json = serde_json::to_string(&response_map).expect("Failed to serialize JSON");
                format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}", json)
            } else {
                format!(
                    "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\n\r\nInvalid Request"
                )
            }
        }
        None => {
            format!("HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\n\r\nInvalid Request")
        }
    };

    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn parse_request(
    request: &str
) -> Option<(Option<String>, Option<HashMap<String, String>>, String)> {
    let url_param = "url=";

    let parts: Vec<&str> = request.split_whitespace().collect();

    if parts.len() >= 2 {
        if parts[1].starts_with("/manga?") && parts[1].contains(&url_param) {
            info!("REQUEST RECEIVED");
            info!("REQUEST Type: download");
            let query_params: HashMap<_, _> = parts[1]
                .split('?')
                .nth(1)
                .unwrap_or("")
                .split('&')
                .filter_map(|param| {
                    let mut iter = param.split('=');
                    let key = iter.next()?.to_owned();
                    let value = iter.next()?.to_owned();
                    Some((key, value))
                })
                .collect();
            if let Some(manga_url) = query_params.get("url").cloned() {
                let id = query_params.get("id").cloned().unwrap_or_default();
                let decoded_url = decode(&manga_url);
                return Some((Some(decoded_url), Some(query_params), id));
            }
        } else if parts[1].starts_with("/manga-result") {
            if ARGS.log {
                info!("REQUEST RECEIVED");
                info!("REQUEST Type: progress");
            }
            let query_params: HashMap<_, _> = parts[1]
                .split('?')
                .nth(1)
                .unwrap_or("")
                .split('&')
                .filter_map(|param| {
                    let mut iter = param.split('=');
                    let key = iter.next()?.to_owned();
                    let value = iter.next()?.to_owned();
                    Some((key, value))
                })
                .collect();
            if let Some(id) = query_params.get("id").cloned() {
                return Some((Some(String::from("progress")), None, id));
            }
        } else if parts[1].eq("/") {
            info!("REQUEST Type: main");
            return Some((Some(String::from("main")), None, String::new()));
        }
    }
    None
}

pub(crate) async fn web() {
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Failed to bind address");
    info!("Server listening on 127.0.0.1:8080");

    let url = "http://127.0.0.1:8080/";
    if let Err(err) = webbrowser::open(url) {
        eprintln!("Error opening web browser: {}", err);
    }

    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                tokio::spawn(handle_client(stream));
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}
