use percent_encoding::{ NON_ALPHANUMERIC, percent_decode_str, percent_encode };
use serde_json::Value;
use std::{ collections::HashMap, io::{ Read, Write }, net::TcpListener };

use crate::{
    error::{ mdown::Error, handle_error },
    getter,
    log,
    log_end,
    resolute,
    resolute::{
        CURRENT_CHAPTER,
        CURRENT_PAGE,
        CURRENT_PAGE_MAX,
        CURRENT_PERCENT,
        CURRENT_SIZE,
        CURRENT_SIZE_MAX,
        WEB_DOWNLOADED,
        MANGA_NAME,
        SCANLATION_GROUPS,
        CURRENT_CHAPTER_PARSED,
        CURRENT_CHAPTER_PARSED_MAX,
    },
    utils,
};
fn decode(url: &str) -> String {
    percent_decode_str(&url).decode_utf8_lossy().to_string()
}

pub(crate) fn encode(url: &str) -> String {
    percent_encode(url.as_bytes(), NON_ALPHANUMERIC).to_string()
}

async fn resolve_web_download(url: &str) -> Result<String, Error> {
    let handle_id = match resolute::HANDLE_ID.lock() {
        Ok(value) => value.clone(),
        Err(err) => {
            return Err(Error::PoisonError(err.to_string()));
        }
    };
    let mut manga_name = String::from("!");
    let id;
    if let Some(id_temp) = utils::resolve_regex(&url) {
        id = id_temp.as_str();
    } else if utils::is_valid_uuid(url) {
        id = url;
    } else {
        log!(&format!("@{} Didn't find any id", handle_id), handle_id);
        return Ok(String::from("!"));
    }
    *(match resolute::MANGA_ID.lock() {
        Ok(value) => value,
        Err(err) => {
            return Err(Error::PoisonError(err.to_string()));
        }
    }) = id.to_string();
    log!(&format!("@{} Found {}", handle_id, id), handle_id);
    match getter::get_manga_json(id).await {
        Ok(manga_name_json) => {
            let json_value = match utils::get_json(&manga_name_json) {
                Ok(value) => value,
                Err(err) => {
                    return Err(err);
                }
            };
            match json_value {
                Value::Object(obj) => {
                    manga_name = match resolute::resolve(obj, id).await {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(err);
                        }
                    };
                }
                _ => {
                    return Err(Error::JsonError(String::from("Could not parse manga json")));
                }
            }
        }
        Err(_) => (),
    }

    if manga_name.eq("!") {
        Ok(String::from("!"))
    } else {
        let downloaded_files = (
            match WEB_DOWNLOADED.lock() {
                Ok(value) => value,
                Err(err) => {
                    return Err(Error::PoisonError(err.to_string()));
                }
            }
        ).clone();
        let scanlation = (
            match SCANLATION_GROUPS.lock() {
                Ok(value) => value,
                Err(err) => {
                    return Err(Error::PoisonError(err.to_string()));
                }
            }
        ).clone();

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
                    scanlation.values().cloned().map(serde_json::Value::String).collect()
                ),
            ),
        ]
            .iter()
            .cloned()
            .collect();

        match serde_json::to_string(&response_map) {
            Ok(value) => Ok(value),
            Err(err) => { Err(Error::JsonError(err.to_string())) }
        }
    }
}

async fn handle_client(mut stream: std::net::TcpStream) -> Result<(), Error> {
    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(_n) => (),
        Err(err) => {
            return Err(Error::IoError(err, None));
        }
    }

    let mut end = false;

    let request = String::from_utf8_lossy(&buffer[..]);

    let response = match parse_request(&request) {
        Some((Some(url), Some(_params), handle_id)) => {
            *(match resolute::HANDLE_ID.lock() {
                Ok(id) => id,
                Err(err) => {
                    return Err(Error::PoisonError(err.to_string()));
                }
            }) = handle_id.clone();
            let json = match resolve_web_download(&url).await {
                Ok(response) =>
                    format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}", response),

                Err(err) => {
                    handle_error(&err, String::from("web_manga"));
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}",
                        r#"{"status": "error"}"#
                    )
                }
            };

            log_end(handle_id);
            *(match resolute::HANDLE_ID.lock() {
                Ok(id) => id,
                Err(err) => {
                    return Err(Error::PoisonError(err.to_string()));
                }
            }) = String::new().into_boxed_str();
            json
        }
        Some((None, _, _)) => {
            format!("HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\n\r\nInvalid Request")
        }
        Some((Some(url), None, _)) => {
            if url == String::from("main") {
                format!(
                    "{}{}",
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n",
                    "<!DOCTYPE html>\n<html lang=\"en\">\n\n<head>\n    <meta charset=\"UTF-8\">\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n    <title>mdown v0.8.2</title>\n\n    <style>\n        body {\n            font-family: Arial, sans-serif;\n            background-color: #121212;\n            color: #fff;\n            margin: 0;\n            padding: 0;\n            box-sizing: border-box;\n            transition: background-color 0.5s;\n        }\n\n        body.dark-mode {\n            background-color: #fff;\n            color: #121212;\n        }\n\n        .title {\n            text-align: center;\n            color: inherit;\n        }\n\n        .mangaForm {\n            max-width: 400px;\n            margin: 20px auto;\n            background-color: #272727;\n            padding: 20px;\n            border-radius: 8px;\n            box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);\n        }\n\n        .mangaForm.dark-mode {\n            color: #FFF;\n            background-color: #FFF;\n        }\n\n        .urlInput {\n            display: block;\n            margin-bottom: 8px;\n            color: #fff;\n        }\n\n        .urlInput.dark-mode {\n            color: #000;\n        }\n\n        input {\n            width: 100%;\n            padding: 10px;\n            margin-bottom: 16px;\n            box-sizing: border-box;\n            border: 1px solid #555;\n            border-radius: 4px;\n            background-color: #333;\n            color: #fff;\n        }\n\n        .exit-button {\n            background-color: #FFF;\n            color: #000;\n            padding: 10px 15px;\n            border: none;\n            border-radius: 50%;\n            cursor: pointer;\n            position: fixed;\n            top: 20px;\n            left: 20px;\n            font-size: 20px;\n        }\n\n        .dark-mode-toggle {\n            background-color: #FFF;\n            color: #000;\n            padding: 10px 15px;\n            border: none;\n            border-radius: 50%;\n            cursor: pointer;\n            position: fixed;\n            top: 20px;\n            right: 20px;\n            font-size: 20px;\n        }\n\n        .dark-mode-toggle:hover {\n            background-color: grey;\n        }\n\n        .download {\n            background-color: #4caf50;\n            color: #fff;\n            padding: 10px 15px;\n            border: none;\n            border-radius: 4px;\n            cursor: pointer;\n        }\n\n        .download:hover {\n            background-color: #45a049;\n        }\n\n        #resultMessage {\n            margin: 20px auto;\n            max-width: 600px;\n            background-color: #272727;\n            padding: 50px;\n            border-radius: 8px;\n            box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);\n        }\n\n        ul {\n            list-style-type: none;\n            padding: 0;\n        }\n\n        li {\n            margin-bottom: 8px;\n        }\n\n        #result {\n            color: #FFF;\n        }\n    </style>\n</head>\n\n<body>\n    <button type=\"button\" onclick=\"exitApp()\" class=\"exit-button\" id=\"exitButton\">Exit</button>\n\n    <h1 class=\"title\">mdown v0.8.2</h1>\n\n    <form class=\"mangaForm\">\n        <label class=\"urlInput\" for=\"urlInput\">Enter Manga URL:</label>\n        <input type=\"text\" id=\"urlInput\" name=\"url\" required>\n        <button type=\"button\" class=\"download\" onclick=\"downloadManga()\">Download</button>\n    </form>\n    <button type=\"button\" onclick=\"toggleDarkMode()\" class=\"dark-mode-toggle\" id=\"darkModeToggle\">&#x2600;</button>\n\n    <div id=\"resultMessage\"></div>\n\n    <script>\n        function delay(time) {\n            return new Promise(resolve => setTimeout(resolve, time));\n        }\n\n        let id = \"\";\n        let isPostRequestInProgress = false;\n        let isPostRequestInProgress_tmp = true;\n\n        function downloadManga() {\n            id = generateRandomId(10);\n            if (isPostRequestInProgress) {\n                alert(\'A download is already in progress. Please wait.\');\n                return;\n            }\n\n            isPostRequestInProgress = true;\n\n            var mangaUrl = document.getElementById(\'urlInput\').value;\n            var encodedUrl = encodeURIComponent(mangaUrl);\n            var url = \"http://127.0.0.1:8080/manga\";\n\n            fetch(url + \"?url=\" + encodedUrl + \"&id=\" + id, {\n                method: \'POST\',\n                headers: {\n                    \'Content-Type\': \'application/json\',\n                },\n            })\n                .then(response => {\n                    if (!response.ok) {\n                        throw new Error(\'Network response was not ok\');\n                    }\n                    return response.json();\n                })\n                .then(result => {\n                    const resultMessageDiv = document.getElementById(\'resultMessage\');\n                    if (result.status == \"ok\") {\n                        console.log(\'Scanlation Groups:\', result.scanlation_groups);\n                        console.log(\'Files:\', result.files);\n                        console.log(\'Manga Name:\', result.name);\n                        console.log(\'Status:\', result.status);\n\n                        resultMessageDiv.innerHTML = \"<p id=\\'result\\'>Download successful!</p>\";\n\n                        if (result.files && result.files.length > 0) {\n                            resultMessageDiv.innerHTML += \"<p id=\\'result\\'>Downloaded Files:</p>\";\n                            resultMessageDiv.innerHTML += \"<ul id=\\'result\\'>\";\n                            result.files.forEach(file => {\n                                resultMessageDiv.innerHTML += \"<li id=\\'result\\'>\" + file + \"</li>\";\n                            });\n                            resultMessageDiv.innerHTML += \"</ul>\";\n                        }\n\n                        if (result.scanlation_groups && result.scanlation_groups.length > 0) {\n                            resultMessageDiv.innerHTML += \"<p id=\\'result\\'>Scanlation Groups:</p>\";\n                            resultMessageDiv.innerHTML += \"<ul id=\\'result\\'>\";\n                            result.scanlation_groups.forEach(group => {\n                                resultMessageDiv.innerHTML += \"<li id=\\'result\\'>\" + group + \"</li>\";\n                            });\n                            resultMessageDiv.innerHTML += \"</ul>\";\n\n                        }\n                        isPostRequestInProgress = false;\n                        isPostRequestInProgress_tmp = true;\n                    }\n                })\n                .catch(error => {\n                    console.error(\'Error during POST request:\', error);\n                    document.getElementById(\'resultMessage\').innerHTML = \"<p id=\'result\'>Error during download. Please try again.<p>\";\n\n                    isPostRequestInProgress = false;\n                    isPostRequestInProgress_tmp = true;\n                });\n        }\n\n        function fetchWhilePostInProgress() {\n            setInterval(() => {\n                if (!isPostRequestInProgress) {\n                    return;\n                }\n                if (isPostRequestInProgress_tmp) {\n                    delay(1000);\n                    isPostRequestInProgress_tmp = false\n                }\n\n                fetch(\"http://127.0.0.1:8080/manga-result?id=\" + id)\n                    .then(response => response.json())\n                    .then(result => {\n                        if (result.status == \"ok\") {\n                            const resultMessageDiv = document.getElementById(\'resultMessage\');\n                            console.log(\'Scanlation Groups:\', result.scanlation_groups);\n                            console.log(\'Files:\', result.files);\n                            console.log(\'Current chapter:\', result.current);\n                            console.log(\'Manga Name:\', result.name);\n                            console.log(\'Status:\', result.status);\n                            console.log(\"current_chapter_parsed\", result.current_chapter_parsed);\n                            console.log(\"current_chapter_parsed_max\", result.current_chapter_parsed_max);\n\n                            resultMessageDiv.innerHTML = \"<p id=\\'result\\'>In Progress!</p>\";\n                            resultMessageDiv.innerHTML += \"<p id=\\'result\\'>Parsed chapters: \" + result.current_chapter_parsed + \"/\" + result.current_chapter_parsed_max + \"</p>\";\n                            if (result.current && result.current.length > 0) {\n                                resultMessageDiv.innerHTML += \"<p id=\\'result\\'>Current chapter: \" + result.current + \"</p>\";\n                            }\n                            if (result.current_page && result.current_page.length > 0 && result.current_page_max && result.current_page_max.length > 0) {\n                                resultMessageDiv.innerHTML += \"<p id=\\'result\\'>\" + \"#\".repeat(parseInt(result.current_page)) + \"  \" + result.current_page + \"|\" + result.current_page_max + \"</p>\";\n                            }\n                            if (result.current_percent && result.current_percent.length > 0) {\n                                resultMessageDiv.innerHTML += \"<p id=\\'result\\'>\" + result.current_percent + \" | \" + result.current_size + \"mb/\" + result.current_size_max + \"mb</p>\";\n                            }\n\n                            if (result.files && result.files.length > 0) {\n                                resultMessageDiv.innerHTML += \"<p id=\\'result\\'>Downloaded Files:</p>\";\n                                resultMessageDiv.innerHTML += \"<ul id=\\'result\\'>\";\n                                result.files.forEach(file => {\n                                    resultMessageDiv.innerHTML += \"<li id=\\'result\\'>\" + file + \"</li>\";\n                                });\n                                resultMessageDiv.innerHTML += \"</ul>\";\n                            }\n\n                            if (result.scanlation_groups && result.scanlation_groups.length > 0) {\n                                resultMessageDiv.innerHTML += \"<p id=\\'result\\'>Scanlation Groups:</p>\";\n                                resultMessageDiv.innerHTML += \"<ul id=\\'result\\'>\";\n                                result.scanlation_groups.forEach(group => {\n                                    resultMessageDiv.innerHTML += \"<li id=\\'result\\'>\" + group + \"</li>\";\n                                });\n                                resultMessageDiv.innerHTML += \"</ul>\";\n\n                            }\n                        }\n                    })\n                    .catch(error => {\n                        console.error(\'Error during GET request:\', error);\n                    });\n            }, 500);\n        }\n        fetchWhilePostInProgress();\n\n        function generateRandomId(length) {\n            const CHARSET = \'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789\';\n            let id = \'\';\n\n            for (let i = 0; i < length; i++) {\n                const randomIndex = Math.floor(Math.random() * CHARSET.length);\n                id += CHARSET.charAt(randomIndex);\n            }\n\n            return id;\n        }\n\n        function toggleDarkMode() {\n            const body = document.body;\n            body.classList.toggle(\'dark-mode\');\n            const button = document.getElementById(\'darkModeToggle\');\n            const exit_button = document.getElementById(\'exitButton\');\n\n            if (body.classList.contains(\'dark-mode\')) {\n                button.innerHTML = \'\u{1F319}\';\n                button.style.backgroundColor = \"#000\";\n                button.style.color = \"#FFF\";\n                exit_button.style.backgroundColor = \"#000\";\n                exit_button.style.color = \"#FFF\";\n            } else {\n                button.innerHTML = \'\u{2600}\';\n                button.style.backgroundColor = \"#FFF\";\n                button.style.color = \"#000\";\n                exit_button.style.backgroundColor = \"#FFF\";\n                exit_button.style.color = \"#000\";\n            }\n        }\n        function exitApp() {\n            fetch(\"http://127.0.0.1:8080/end\", {\n                method: \'GET\'\n            })\n                .then(response => {\n                    if (response.ok) {\n                        window.close();\n                    } else {\n                        console.error(\'Failed to send exit request\');\n                    }\n                })\n                .catch(error => {\n                    console.error(\'Error while sending exit request:\', error);\n                });\n        }\n\n    </script>\n\n</body>\n\n</html>"
                )
            } else if url == String::from("progress") {
                let downloaded_files = (
                    match WEB_DOWNLOADED.lock() {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(Error::PoisonError(err.to_string()));
                        }
                    }
                ).clone();
                let scanlation = (
                    match SCANLATION_GROUPS.lock() {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(Error::PoisonError(err.to_string()));
                        }
                    }
                ).clone();
                let response_map: HashMap<&str, serde_json::Value> = [
                    ("status", serde_json::Value::String("ok".to_string())),
                    (
                        "name",
                        serde_json::Value::String(
                            (
                                match MANGA_NAME.lock() {
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(Error::PoisonError(err.to_string()));
                                    }
                                }
                            ).to_string()
                        ),
                    ),
                    (
                        "current",
                        serde_json::Value::String(
                            (
                                match CURRENT_CHAPTER.lock() {
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(Error::PoisonError(err.to_string()));
                                    }
                                }
                            ).to_string()
                        ),
                    ),
                    (
                        "current_page",
                        serde_json::Value::String(
                            (
                                match CURRENT_PAGE.lock() {
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(Error::PoisonError(err.to_string()));
                                    }
                                }
                            ).to_string()
                        ),
                    ),
                    (
                        "current_page_max",
                        serde_json::Value::String(
                            (
                                match CURRENT_PAGE_MAX.lock() {
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(Error::PoisonError(err.to_string()));
                                    }
                                }
                            ).to_string()
                        ),
                    ),
                    (
                        "current_percent",
                        serde_json::Value::String(
                            format!("{:.2}", match CURRENT_PERCENT.lock() {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(Error::PoisonError(err.to_string()));
                                }
                            })
                        ),
                    ),
                    (
                        "current_size",
                        serde_json::Value::String(
                            format!("{:.2}", match CURRENT_SIZE.lock() {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(Error::PoisonError(err.to_string()));
                                }
                            })
                        ),
                    ),
                    (
                        "current_size_max",
                        serde_json::Value::String(
                            format!("{:.2}", match CURRENT_SIZE_MAX.lock() {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(Error::PoisonError(err.to_string()));
                                }
                            })
                        ),
                    ),
                    (
                        "current_chapter_parsed",
                        serde_json::Value::String(
                            (
                                match CURRENT_CHAPTER_PARSED.lock() {
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(Error::PoisonError(err.to_string()));
                                    }
                                }
                            ).to_string()
                        ),
                    ),
                    (
                        "current_chapter_parsed_max",
                        serde_json::Value::String(
                            (
                                match CURRENT_CHAPTER_PARSED_MAX.lock() {
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(Error::PoisonError(err.to_string()));
                                    }
                                }
                            ).to_string()
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
                            scanlation.values().cloned().map(serde_json::Value::String).collect()
                        ),
                    ),
                ]
                    .iter()
                    .cloned()
                    .collect();
                let json = match serde_json::to_string(&response_map) {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(Error::JsonError(err.to_string()));
                    }
                };
                format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}", json)
            } else if url == String::from("exit") {
                end = true;
                String::from(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"ok\"}"
                )
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

    match stream.write_all(response.as_bytes()) {
        Ok(()) => (),
        Err(_err) => (),
    }
    match stream.flush() {
        Ok(()) => (),
        Err(_err) => (),
    }

    if end {
        log!("[user] Ctrl+C received! Exiting...");
        log!("[web] Closing server");

        match utils::remove_cache() {
            Ok(()) => (),
            Err(err) => {
                handle_error(&err, String::from("ctrl_handler"));
            }
        }
        std::process::exit(0);
    }
    Ok(())
}

fn parse_request(
    request: &str
) -> Option<(Option<String>, Option<HashMap<String, String>>, Box<str>)> {
    let url_param = "url=";

    let parts: Vec<&str> = request.split_whitespace().collect();

    if parts.len() >= 2 {
        if parts[1].starts_with("/manga?") && parts[1].contains(&url_param) {
            log!("REQUEST RECEIVED");
            log!("REQUEST Type: download");
            let query_params: HashMap<_, _> = (
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
                .collect();
            if let Some(manga_url) = query_params.get("url").cloned() {
                let id = match query_params.get("id").cloned() {
                    Some(id) => id.into_boxed_str(),
                    None => String::from("0").into_boxed_str(),
                };
                let decoded_url = decode(&manga_url);
                return Some((Some(decoded_url), Some(query_params), id));
            }
        } else if parts[1].starts_with("/manga-result") {
            let query_params: HashMap<_, _> = (
                match parts[1].split('?').nth(1) {
                    Some(value) => value,
                    None => "",
                }
            )
                .split('&')
                .filter_map(|param| {
                    let mut iter = param.split('=');
                    let key = iter.next()?.to_owned();
                    let value = iter.next()?.to_owned();
                    Some((key, value))
                })
                .collect();
            if let Some(id) = query_params.get("id").cloned() {
                log!("REQUEST RECEIVED", id.clone().into_boxed_str());
                log!("REQUEST Type: progress", id.clone().into_boxed_str());

                return Some((Some(String::from("progress")), None, id.into_boxed_str()));
            }
        } else if parts[1].starts_with("/end") {
            log!("REQUEST Type: end");
            return Some((Some(String::from("exit")), None, String::new().into_boxed_str()));
        } else if parts[1].eq("/") {
            log!("REQUEST Type: main");
            return Some((Some(String::from("main")), None, String::new().into_boxed_str()));
        }
    }
    None
}

async fn web() -> Result<(), Error> {
    let listener = match TcpListener::bind("127.0.0.1:8080") {
        Ok(listener) => listener,
        Err(err) => {
            return Err(Error::IoError(err, None));
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

pub(crate) async fn start() -> Result<(), Error> {
    match
        ctrlc::set_handler(|| {
            log!("[user] Ctrl+C received! Exiting...");
            log!("[web] Closing server");

            match utils::remove_cache() {
                Ok(()) => (),
                Err(err) => {
                    handle_error(&err, String::from("ctrl_handler"));
                }
            }
            std::process::exit(0);
        })
    {
        Ok(()) => (),
        Err(err) => {
            return Err(
                Error::CustomError(
                    format!("Failed setting up ctrl handler, {}", err.to_string()),
                    String::from("CTRL handler")
                )
            );
        }
    }
    match web().await {
        Ok(()) => Ok(()),
        Err(err) => Err(err),
    }
}
