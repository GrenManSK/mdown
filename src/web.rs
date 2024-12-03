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

fn decode(url: &str) -> String {
    percent_decode_str(url).decode_utf8_lossy().to_string()
}

pub(crate) fn encode(url: &str) -> String {
    percent_encode(url.as_bytes(), NON_ALPHANUMERIC).to_string()
}

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

    let path = parts[1];

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
            let query_params = getter::get_query(parts);
            let file_path = match query_params.get("path").cloned() {
                Some(value) => value,
                None => {
                    return Ok(());
                }
            };

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
                    return Err(err);
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
            response = String::from(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"error\"}"
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

fn get_html() -> String {
    if *args::ARGS_DEV {
        let err_404 = String::from(
            "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n    <meta charset=\"UTF-8\">\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n    <title>404 Error - Page Not Found</title>\n    <style>\n        body {\n            font-family: Arial, sans-serif;\n            background-color: #f7f7f7;\n            color: #333;\n            margin: 0;\n            padding: 0;\n            text-align: center;\n        }\n        .container {\n            position: absolute;\n            top: 50%;\n            left: 50%;\n            transform: translate(-50%, -50%);\n        }\n        h1 {\n            font-size: 36px;\n            margin-bottom: 20px;\n        }\n        p {\n            font-size: 18px;\n            margin-bottom: 20px;\n        }\n        a {\n            color: #007bff;\n            text-decoration: none;\n        }\n        a:hover {\n            text-decoration: underline;\n        }\n    </style>\n</head>\n<body>\n    <div class=\"container\">\n        <h1>404 Error - Page Not Found</h1>\n        <p>The page you are looking for might have been removed, had its name changed, or is temporarily unavailable.</p>\n        <p>Go back to <a href=\"/\">home page</a>.</p>\n    </div>\n</body>\n</html>\n"
        );
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
            "<!DOCTYPE html>\n<html lang=\"en\">\n\n<head>\n    <meta charset=\"UTF-8\">\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n    <title>Mdown</title>\n\n    <style>\n        body {\n            font-family: Arial, sans-serif;\n            background-color: #121212;\n            color: #fff;\n            margin: 0;\n            padding: 0;\n            box-sizing: border-box;\n            transition: background-color 0.5s;\n        }\n\n        body.dark-mode {\n            background-color: #fff;\n            color: #121212;\n        }\n\n        .title {\n            margin-left: 44vw;\n            color: inherit;\n            display: flex;\n            align-items: center;\n        }\n\n        .mangaForm {\n            max-width: 400px;\n            margin: 20px auto;\n            background-color: #272727;\n            padding: 20px;\n            border-radius: 8px;\n            box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);\n        }\n\n        .mangaForm.dark-mode {\n            color: #FFF;\n            background-color: #FFF;\n        }\n\n        .urlInput {\n            display: block;\n            margin-bottom: 8px;\n            color: #fff;\n        }\n\n        .urlInput.dark-mode {\n            color: #000;\n        }\n\n        input {\n            width: 100%;\n            padding: 10px;\n            margin-bottom: 16px;\n            box-sizing: border-box;\n            border: 1px solid #555;\n            border-radius: 4px;\n            background-color: #333;\n            color: #fff;\n        }\n\n        .exit-button {\n            background-color: #FFF;\n            color: #000;\n            padding: 10px 15px;\n            border: none;\n            border-radius: 50%;\n            cursor: pointer;\n            position: fixed;\n            top: 20px;\n            left: 20px;\n            font-size: 20px;\n        }\n\n        .dark-mode-toggle {\n            background-color: #FFF;\n            color: #000;\n            padding: 10px 15px;\n            border: none;\n            border-radius: 50%;\n            cursor: pointer;\n            position: fixed;\n            top: 20px;\n            right: 20px;\n            font-size: 20px;\n        }\n\n        .dark-mode-toggle:hover {\n            background-color: grey;\n        }\n\n        .download {\n            background-color: #4caf50;\n            color: #fff;\n            padding: 10px 15px;\n            border: none;\n            border-radius: 4px;\n            cursor: pointer;\n        }\n\n        .download:hover {\n            background-color: #45a049;\n        }\n\n        #resultMessage {\n            margin: 20px auto;\n            max-width: 600px;\n            background-color: #272727;\n            padding: 50px;\n            border-radius: 8px;\n            box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);\n        }\n\n        ul {\n            list-style-type: none;\n            padding: 0;\n        }\n\n        li {\n            margin-bottom: 8px;\n        }\n\n        #result {\n            color: #FFF;\n        }\n\n        #resultEnd {\n            margin: 20px auto;\n            max-width: 600px;\n            background-color: #272727;\n            padding: 50px;\n            border-radius: 8px;\n            box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);\n            animation: popUp 1s ease-out;\n            display: none;\n            transform: scale(0);\n            opacity: 0;\n        }\n\n        #resultEnd.dark-mode {\n            color: #000\n        }\n\n        #resultEnd.visible {\n            display: block;\n            position: absolute;\n            z-index: 10;\n            top: 30%;\n            left: 40vw;\n            color: #FFF;\n            animation: popUp 1s ease-out forwards;\n        }\n\n        @keyframes popUp {\n            0% {\n                transform: scale(0);\n                opacity: 0;\n            }\n\n            95% {\n                transform: scale(4);\n                opacity: 1;\n            }\n\n            100% {\n                transform: scale(2);\n                opacity: 1;\n            }\n        }\n\n        #imageContainer {\n            position: fixed;\n            top: 0;\n            left: 0;\n            width: 100%;\n            height: 100%;\n            pointer-events: none;\n            overflow: hidden;\n        }\n\n        .flying-image {\n            position: absolute;\n            animation: fly 200s linear infinite;\n            max-width: 20vw;\n            animation-direction: alternate;\n            animation-timing-function: ease-in-out;\n        }\n\n        @keyframes fly {\n            0% {\n                transform: translateX(-100vw) rotate(-20deg);\n            }\n\n            100% {\n                transform: translateX(200vw) rotate(20deg);\n            }\n        }\n\n        #version {\n            margin-left: 5px;\n        }\n    </style>\n</head>\n\n<body>\n    <button type=\"button\" onclick=\"exitApp()\" class=\"exit-button\" id=\"exitButton\">Exit</button>\n    <button type=\"button\" onclick=\"toggleDarkMode()\" class=\"dark-mode-toggle\" id=\"darkModeToggle\">&#x2600;</button>\n\n    <h1 class=\"title\">mdown <p id=\"version\"></p>\n    </h1>\n\n    <form class=\"mangaForm\">\n        <label class=\"urlInput\" for=\"urlInput\">Enter Manga URL:</label>\n        <input type=\"text\" id=\"urlInput\" name=\"url\" required>\n        <button type=\"button\" class=\"download\" onclick=\"downloadManga()\">Download</button>\n    </form>\n\n    <div id=\"resultMessage\"></div>\n\n    <div id=\"resultEnd\"></div>\n\n    <div id=\"imageContainer\"></div>\n\n    <audio id=\"downloadedMusic\" src=\"__get__?path=rambling_pleat\" loop></audio>\n    <audio id=\"downloadMusic\" src=\"__get__?path=system_haven\" loop></audio>\n\n    <script>\n        fetch(\'__version__\')\n            .then(response => {\n                if (!response.ok) {\n                    throw new Error(\'Network response was not ok\');\n                }\n                return response.text();\n            })\n            .then(text => {\n                document.getElementById(\'version\').textContent = `v${text}`;\n            })\n            .catch(error => {\n                console.error(\'There was a problem fetching the text:\', error);\n            });\n\n        function delay(time) {\n            return new Promise(resolve => setTimeout(resolve, time));\n        }\n\n        let id = \"\";\n        let isPostRequestInProgress = false;\n        let isPostRequestInProgress_tmp = true;\n        let images = [];\n        let times = 0;\n        let end = false;\n\n        function sleep(ms) {\n            return new Promise(resolve => setTimeout(resolve, ms));\n        }\n\n        function clickHandler(event) {\n            end = true;\n            const resultEndDiv = document.getElementById(\'resultEnd\');\n            resultEndDiv.classList.remove(\'visible\');\n            const downloadedMusic = document.getElementById(\'downloadedMusic\');\n            downloadedMusic.pause();\n            downloadedMusic.currentTime = 0;\n            const imageContainer = document.getElementById(\'imageContainer\');\n            imageContainer.innerHTML = \'\';\n        }\n\n        function createFlyingImage() {\n            const imageContainer = document.getElementById(\'imageContainer\');\n            const img = document.createElement(\'img\');\n            console.log(images.length);\n\n            var randomIndex = Math.floor(Math.random() * images.length);\n\n            var randomImage = images[randomIndex];\n            img.src = \"data:image/png;base64,\" + images[randomIndex];\n            img.classList.add(\'flying-image\');\n            img.style.zIndex = Math.random() >= 0.5 ? \"1\" : \"20\";\n\n            const initialPosition = \"0vw\";\n            img.style.left = initialPosition;\n            img.style.top = `${(Math.random() * 100) - 25}vh`;\n            img.style.animationDuration = `${5 + Math.random() * 20}s`;\n\n            imageContainer.appendChild(img);\n\n            img.addEventListener(\'animationiteration\', () => {\n                const newInitialPosition = initialPosition === \'-100vw\' ? \'200vw\' : \'-100vw\';\n                img.style.left = newInitialPosition;\n            });\n        }\n\n        async function get_confetti() {\n            try {\n                const response = await fetch(\'__confetti__\');\n                if (!response.ok) {\n                    throw new Error(\'Network response was not ok\');\n                }\n                const data = await response.json();\n                images = data.images;\n            } catch (error) {\n                console.error(\'Error:\', error);\n                throw error;\n            }\n        }\n\n        function start_confetti_event() {\n            if (end) {\n                return;\n            }\n            times += 1;\n            const randomInterval = Math.random() * (2000 - 500) + 500;\n            setTimeout(() => {\n                if (times % 10 === 0) {\n                    start_confetti_big();\n                } else {\n                    start_confetti();\n                }\n                start_confetti_event();\n            }, randomInterval);\n        }\n\n        function start_confetti() {\n            confetti({\n                particleCount: 250,\n                spread: 100,\n                origin: { y: Math.random(), x: Math.random() }\n            });\n        }\n\n        function start_confetti_big() {\n            confetti({\n                particleCount: 250,\n                spread: 100,\n                origin: { y: Math.random(), x: Math.random() }\n            });\n            confetti({\n                particleCount: 250,\n                spread: 100,\n                origin: { y: Math.random(), x: Math.random() }\n            });\n            confetti({\n                particleCount: 250,\n                spread: 100,\n                origin: { y: Math.random(), x: Math.random() }\n            });\n        }\n\n        function downloadManga() {\n            id = generateRandomId(10);\n            if (isPostRequestInProgress) {\n                alert(\'A download is already in progress. Please wait.\');\n                return;\n            }\n\n            isPostRequestInProgress = true;\n\n\n            const downloadMusic = document.getElementById(\'downloadMusic\');\n            downloadMusic.play().catch(error => console.log(\'Error playing sound:\', error));\n\n            var mangaUrl = document.getElementById(\'urlInput\').value;\n            var encodedUrl = encodeURIComponent(mangaUrl);\n            var url = \"http://127.0.0.1:8080/manga\";\n\n            fetch(url + \"?url=\" + encodedUrl + \"&id=\" + id, {\n                method: \'POST\',\n                headers: {\n                    \'Content-Type\': \'application/json\',\n                },\n            })\n                .then(response => {\n                    if (!response.ok) {\n                        throw new Error(\'Network response was not ok\');\n                    }\n                    return response.json();\n                })\n                .then(async result => {\n                    const resultMessageDiv = document.getElementById(\'resultMessage\');\n                    if (result.status == \"ok\") {\n                        end = false;\n\n                        console.log(\'Scanlation Groups:\', result.scanlation_groups);\n                        console.log(\'Files:\', result.files);\n                        console.log(\'Manga Name:\', result.name);\n                        console.log(\'Status:\', result.status);\n\n                        resultMessageDiv.innerHTML = \"<p id=\\\'result\\\'>Download successful!</p>\";\n\n                        if (result.files && result.files.length > 0) {\n                            resultMessageDiv.innerHTML += \"<p id=\\\'result\\\'>Downloaded Files:</p>\";\n                            resultMessageDiv.innerHTML += \"<ul id=\\\'result\\\'>\";\n                            result.files.forEach(file => {\n                                resultMessageDiv.innerHTML += \"<li id=\\\'result\\\'>\" + file + \"</li>\";\n                            });\n                            resultMessageDiv.innerHTML += \"</ul>\";\n                        }\n\n                        if (result.scanlation_groups && result.scanlation_groups.length > 0) {\n                            resultMessageDiv.innerHTML += \"<p id=\\\'result\\\'>Scanlation Groups:</p>\";\n                            resultMessageDiv.innerHTML += \"<ul id=\\\'result\\\'>\";\n                            result.scanlation_groups.forEach(group => {\n                                resultMessageDiv.innerHTML += \"<li id=\\\'result\\\'>\" + group + \"</li>\";\n                            });\n                            resultMessageDiv.innerHTML += \"</ul>\";\n\n                        }\n                        isPostRequestInProgress = false;\n                        isPostRequestInProgress_tmp = true;\n\n                        await get_confetti();\n                        const resultEnd = document.getElementById(\'resultEnd\');\n                        resultEnd.innerHTML = `<p>${result.name} has been downloaded</p>`;\n\n                        const downloadMusic = document.getElementById(\'downloadMusic\');\n                        downloadMusic.pause();\n                        downloadMusic.currentTime = 0;\n                        const downloadedMusic = document.getElementById(\'downloadedMusic\');\n                        downloadedMusic.play().catch(error => console.log(\'Error playing sound:\', error));\n\n                        const body = document.body;\n\n                        setTimeout(() => {\n                            body.style.transition = \"0s\";\n                            body.style.backgroundColor = \"#FFF\";\n                        }, 100);\n                        setTimeout(() => {\n                            body.style.backgroundColor = \"#cfff01\";\n\n                        }, 200);\n                        setTimeout(() => {\n                            body.style.backgroundColor = \"#2da657\";\n\n                        }, 300);\n                        setTimeout(() => {\n                            body.style.backgroundColor = \"#0763cc\";\n\n                        }, 400);\n                        setTimeout(() => {\n                            body.style.backgroundColor = \"#cc074c\";\n                        }, 500);\n                        setTimeout(() => {\n                            body.style.backgroundColor = \"#121212\";\n                            body.style.transition = \"background-color 0.5s\";\n                            confetti({\n                                particleCount: 250,\n                                spread: 100,\n                                origin: { y: 0.6 }\n                            });\n                            confetti({\n                                particleCount: 250,\n                                spread: 100,\n                                origin: { y: 0.8, x: 0.25 }\n                            });\n                            confetti({\n                                particleCount: 250,\n                                spread: 100,\n                                origin: { y: 0.8, x: 0.75 }\n                            });\n\n                            start_confetti_event();\n                        }, 900);\n\n                        showResultEnd();\n\n                        for (let i = 0; i < 10; i++) {\n                            createFlyingImage();\n                        }\n                        document.addEventListener(\'click\', clickHandler);\n                    }\n                })\n                .catch(error => {\n                    console.error(\'Error during POST request:\', error);\n                    document.getElementById(\'resultMessage\').innerHTML = \"<p id=\'result\'>Error during download. Please try again.<p>\";\n\n                    isPostRequestInProgress = false;\n                    isPostRequestInProgress_tmp = true;\n                });\n        }\n\n        function fetchWhilePostInProgress() {\n            var parsed = 0;\n            var total = 0;\n            var current = 0;\n            setInterval(async () => {\n                if (!isPostRequestInProgress) {\n                    return;\n                }\n\n                if (isPostRequestInProgress_tmp) {\n                    await delay(1000);\n                    isPostRequestInProgress_tmp = false;\n                }\n\n                fetch(\"http://127.0.0.1:8080/manga-result?id=\" + id)\n                    .then(response => response.json())\n                    .then(async result => {\n                        if (result.status === \"ok\") {\n                            const resultMessageDiv = document.getElementById(\'resultMessage\');\n                            resultMessageDiv.innerHTML = `\n                        <p id=\'result\'>In Progress!</p>\n                        <p id=\'result\'>Parsed chapters: ${result.current_chapter_parsed}/${result.current_chapter_parsed_max}</p>\n                        ${result.current ? `<p id=\'result\'>Current chapter: ${result.current}</p>` : \'\'}\n                    `;\n\n                            let progressElement = document.getElementById(\'progress\');\n                            if (!progressElement) {\n                                progressElement = document.createElement(\'div\');\n                                progressElement.id = \'progress\';\n                                resultMessageDiv.appendChild(progressElement);\n                            }\n\n                            if (result.current_page && result.current_page_max) {\n                                for (let i = current; i <= result.current_page; i++) {\n                                    let progressHTML = `<p id=\'result\'>${\"#\".repeat(i)}  ${i}|${result.current_page_max}</p>`;\n                                    progressElement.innerHTML = progressHTML;\n                                    await delay(10); // Small delay for visual effect\n                                }\n                                current = result.current_page;\n                            }\n\n                            if (result.current_percent && result.current_size && result.current_size_max) {\n                                resultMessageDiv.innerHTML += `\n                            <p id=\'result\'>${result.current_percent} | ${result.current_size}mb/${result.current_size_max}mb</p>\n                        `;\n                            }\n\n                            if (result.files && result.files.length > 0) {\n                                let filesHTML = `\n                            <p id=\'result\'>Downloaded Files:</p>\n                            <ul id=\'result\'>\n                                ${result.files.map(file => `<li id=\'result\'>${file}</li>`).join(\'\')}\n                            </ul>\n                        `;\n                                resultMessageDiv.innerHTML += filesHTML;\n                            }\n\n                            if (result.scanlation_groups && result.scanlation_groups.length > 0) {\n                                let groupsHTML = `\n                            <p id=\'result\'>Scanlation Groups:</p>\n                            <ul id=\'result\'>\n                                ${result.scanlation_groups.map(group => `<li id=\'result\'>${group}</li>`).join(\'\')}\n                            </ul>\n                        `;\n                                resultMessageDiv.innerHTML += groupsHTML;\n                            }\n\n                            parsed = result.current_chapter_parsed;\n                            total = result.current_page_max;\n                        }\n                    })\n                    .catch(error => {\n                        console.error(\'Error during GET request:\', error);\n                    });\n            }, 500);\n        }\n        fetchWhilePostInProgress();\n\n        function showResultEnd() {\n            const resultEndDiv = document.getElementById(\'resultEnd\');\n            resultEndDiv.classList.add(\'visible\');\n        }\n\n        function generateRandomId(length) {\n            const CHARSET = \'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789\';\n            let id = \'\';\n\n            for (let i = 0; i < length; i++) {\n                const randomIndex = Math.floor(Math.random() * CHARSET.length);\n                id += CHARSET.charAt(randomIndex);\n            }\n\n            return id;\n        }\n\n        function toggleDarkMode() {\n            const body = document.body;\n            body.classList.toggle(\'dark-mode\');\n            const button = document.getElementById(\'darkModeToggle\');\n            const exit_button = document.getElementById(\'exitButton\');\n\n            if (body.classList.contains(\'dark-mode\')) {\n                button.innerHTML = \'\\u{1F319}\';\n                button.style.backgroundColor = \"#000\";\n                button.style.color = \"#FFF\";\n                exit_button.style.backgroundColor = \"#000\";\n                exit_button.style.color = \"#FFF\";\n            } else {\n                button.innerHTML = \'\\u{2600}\';\n                button.style.backgroundColor = \"#FFF\";\n                button.style.color = \"#000\";\n                exit_button.style.backgroundColor = \"#FFF\";\n                exit_button.style.color = \"#000\";\n            }\n        }\n        function exitApp() {\n            fetch(\"http://127.0.0.1:8080/end\", {\n                method: \'GET\'\n            })\n                .then(response => {\n                    if (response.ok) {\n                        window.close();\n                    } else {\n                        console.error(\'Failed to send exit request\');\n                    }\n                })\n                .catch(error => {\n                    console.error(\'Error while sending exit request:\', error);\n                });\n        }\n        !function (t, e) { !function t(e, n, a, i) { var o = !!(e.Worker && e.Blob && e.Promise && e.OffscreenCanvas && e.OffscreenCanvasRenderingContext2D && e.HTMLCanvasElement && e.HTMLCanvasElement.prototype.transferControlToOffscreen && e.URL && e.URL.createObjectURL); function r() { } function l(t) { var a = n.exports.Promise, i = void 0 !== a ? a : e.Promise; return \"function\" == typeof i ? new i(t) : (t(r, r), null) } var c, s, u, d, f, h, m, g, b, v = (u = Math.floor(1e3 / 60), d = {}, f = 0, \"function\" == typeof requestAnimationFrame && \"function\" == typeof cancelAnimationFrame ? (c = function (t) { var e = Math.random(); return d[e] = requestAnimationFrame((function n(a) { f === a || f + u - 1 < a ? (f = a, delete d[e], t()) : d[e] = requestAnimationFrame(n) })), e }, s = function (t) { d[t] && cancelAnimationFrame(d[t]) }) : (c = function (t) { return setTimeout(t, u) }, s = function (t) { return clearTimeout(t) }), { frame: c, cancel: s }), p = (g = {}, function () { if (h) return h; if (!a && o) { var e = [\"var CONFETTI, SIZE = {}, module = {};\", \"(\" + t.toString() + \")(this, module, true, SIZE);\", \"onmessage = function(msg) {\", \"  if (msg.data.options) {\", \"    CONFETTI(msg.data.options).then(function () {\", \"      if (msg.data.callback) {\", \"        postMessage({ callback: msg.data.callback });\", \"      }\", \"    });\", \"  } else if (msg.data.reset) {\", \"    CONFETTI.reset();\", \"  } else if (msg.data.resize) {\", \"    SIZE.width = msg.data.resize.width;\", \"    SIZE.height = msg.data.resize.height;\", \"  } else if (msg.data.canvas) {\", \"    SIZE.width = msg.data.canvas.width;\", \"    SIZE.height = msg.data.canvas.height;\", \"    CONFETTI = module.exports.create(msg.data.canvas);\", \"  }\", \"}\"].join(\"\\n\"); try { h = new Worker(URL.createObjectURL(new Blob([e]))) } catch (t) { return void 0 !== typeof console && \"function\" == typeof console.warn && console.warn(\"🎊 Could not load worker\", t), null } !function (t) { function e(e, n) { t.postMessage({ options: e || {}, callback: n }) } t.init = function (e) { var n = e.transferControlToOffscreen(); t.postMessage({ canvas: n }, [n]) }, t.fire = function (n, a, i) { if (m) return e(n, null), m; var o = Math.random().toString(36).slice(2); return m = l((function (a) { function r(e) { e.data.callback === o && (delete g[o], t.removeEventListener(\"message\", r), m = null, i(), a()) } t.addEventListener(\"message\", r), e(n, o), g[o] = r.bind(null, { data: { callback: o } }) })) }, t.reset = function () { for (var e in t.postMessage({ reset: !0 }), g) g[e](), delete g[e] } }(h) } return h }), y = { particleCount: 50, angle: 90, spread: 45, startVelocity: 45, decay: .9, gravity: 1, drift: 0, ticks: 200, x: .5, y: .5, shapes: [\"square\", \"circle\"], zIndex: 100, colors: [\"#26ccff\", \"#a25afd\", \"#ff5e7e\", \"#88ff5a\", \"#fcff42\", \"#ffa62d\", \"#ff36ff\"], disableForReducedMotion: !1, scalar: 1 }; function M(t, e, n) { return function (t, e) { return e ? e(t) : t }(t && null != t[e] ? t[e] : y[e], n) } function w(t) { return t < 0 ? 0 : Math.floor(t) } function x(t) { return parseInt(t, 16) } function C(t) { return t.map(k) } function k(t) { var e = String(t).replace(/[^0-9a-f]/gi, \"\"); return e.length < 6 && (e = e[0] + e[0] + e[1] + e[1] + e[2] + e[2]), { r: x(e.substring(0, 2)), g: x(e.substring(2, 4)), b: x(e.substring(4, 6)) } } function I(t) { t.width = document.documentElement.clientWidth, t.height = document.documentElement.clientHeight } function S(t) { var e = t.getBoundingClientRect(); t.width = e.width, t.height = e.height } function T(t, e, n, o, r) { var c, s, u = e.slice(), d = t.getContext(\"2d\"), f = l((function (e) { function l() { c = s = null, d.clearRect(0, 0, o.width, o.height), r(), e() } c = v.frame((function e() { !a || o.width === i.width && o.height === i.height || (o.width = t.width = i.width, o.height = t.height = i.height), o.width || o.height || (n(t), o.width = t.width, o.height = t.height), d.clearRect(0, 0, o.width, o.height), u = u.filter((function (t) { return function (t, e) { e.x += Math.cos(e.angle2D) * e.velocity + e.drift, e.y += Math.sin(e.angle2D) * e.velocity + e.gravity, e.wobble += e.wobbleSpeed, e.velocity *= e.decay, e.tiltAngle += .1, e.tiltSin = Math.sin(e.tiltAngle), e.tiltCos = Math.cos(e.tiltAngle), e.random = Math.random() + 2, e.wobbleX = e.x + 10 * e.scalar * Math.cos(e.wobble), e.wobbleY = e.y + 10 * e.scalar * Math.sin(e.wobble); var n = e.tick++ / e.totalTicks, a = e.x + e.random * e.tiltCos, i = e.y + e.random * e.tiltSin, o = e.wobbleX + e.random * e.tiltCos, r = e.wobbleY + e.random * e.tiltSin; return t.fillStyle = \"rgba(\" + e.color.r + \", \" + e.color.g + \", \" + e.color.b + \", \" + (1 - n) + \")\", t.beginPath(), \"circle\" === e.shape ? t.ellipse ? t.ellipse(e.x, e.y, Math.abs(o - a) * e.ovalScalar, Math.abs(r - i) * e.ovalScalar, Math.PI / 10 * e.wobble, 0, 2 * Math.PI) : function (t, e, n, a, i, o, r, l, c) { t.save(), t.translate(e, n), t.rotate(o), t.scale(a, i), t.arc(0, 0, 1, r, l, c), t.restore() }(t, e.x, e.y, Math.abs(o - a) * e.ovalScalar, Math.abs(r - i) * e.ovalScalar, Math.PI / 10 * e.wobble, 0, 2 * Math.PI) : (t.moveTo(Math.floor(e.x), Math.floor(e.y)), t.lineTo(Math.floor(e.wobbleX), Math.floor(i)), t.lineTo(Math.floor(o), Math.floor(r)), t.lineTo(Math.floor(a), Math.floor(e.wobbleY))), t.closePath(), t.fill(), e.tick < e.totalTicks }(d, t) })), u.length ? c = v.frame(e) : l() })), s = l })); return { addFettis: function (t) { return u = u.concat(t), f }, canvas: t, promise: f, reset: function () { c && v.cancel(c), s && s() } } } function E(t, n) { var a, i = !t, r = !!M(n || {}, \"resize\"), c = M(n, \"disableForReducedMotion\", Boolean), s = o && !!M(n || {}, \"useWorker\") ? p() : null, u = i ? I : S, d = !(!t || !s) && !!t.__confetti_initialized, f = \"function\" == typeof matchMedia && matchMedia(\"(prefers-reduced-motion)\").matches; function h(e, n, i) { for (var o, r, l, c, s, d = M(e, \"particleCount\", w), f = M(e, \"angle\", Number), h = M(e, \"spread\", Number), m = M(e, \"startVelocity\", Number), g = M(e, \"decay\", Number), b = M(e, \"gravity\", Number), v = M(e, \"drift\", Number), p = M(e, \"colors\", C), y = M(e, \"ticks\", Number), x = M(e, \"shapes\"), k = M(e, \"scalar\"), I = function (t) { var e = M(t, \"origin\", Object); return e.x = M(e, \"x\", Number), e.y = M(e, \"y\", Number), e }(e), S = d, E = [], F = t.width * I.x, N = t.height * I.y; S--;)E.push((o = { x: F, y: N, angle: f, spread: h, startVelocity: m, color: p[S % p.length], shape: x[(c = 0, s = x.length, Math.floor(Math.random() * (s - c)) + c)], ticks: y, decay: g, gravity: b, drift: v, scalar: k }, r = void 0, l = void 0, r = o.angle * (Math.PI / 180), l = o.spread * (Math.PI / 180), { x: o.x, y: o.y, wobble: 10 * Math.random(), wobbleSpeed: Math.min(.11, .1 * Math.random() + .05), velocity: .5 * o.startVelocity + Math.random() * o.startVelocity, angle2D: -r + (.5 * l - Math.random() * l), tiltAngle: (.5 * Math.random() + .25) * Math.PI, color: o.color, shape: o.shape, tick: 0, totalTicks: o.ticks, decay: o.decay, drift: o.drift, random: Math.random() + 2, tiltSin: 0, tiltCos: 0, wobbleX: 0, wobbleY: 0, gravity: 3 * o.gravity, ovalScalar: .6, scalar: o.scalar })); return a ? a.addFettis(E) : (a = T(t, E, u, n, i)).promise } function m(n) { var o = c || M(n, \"disableForReducedMotion\", Boolean), m = M(n, \"zIndex\", Number); if (o && f) return l((function (t) { t() })); i && a ? t = a.canvas : i && !t && (t = function (t) { var e = document.createElement(\"canvas\"); return e.style.position = \"fixed\", e.style.top = \"0px\", e.style.left = \"0px\", e.style.pointerEvents = \"none\", e.style.zIndex = t, e }(m), document.body.appendChild(t)), r && !d && u(t); var g = { width: t.width, height: t.height }; function b() { if (s) { var e = { getBoundingClientRect: function () { if (!i) return t.getBoundingClientRect() } }; return u(e), void s.postMessage({ resize: { width: e.width, height: e.height } }) } g.width = g.height = null } function v() { a = null, r && e.removeEventListener(\"resize\", b), i && t && (document.body.removeChild(t), t = null, d = !1) } return s && !d && s.init(t), d = !0, s && (t.__confetti_initialized = !0), r && e.addEventListener(\"resize\", b, !1), s ? s.fire(n, g, v) : h(n, g, v) } return m.reset = function () { s && s.reset(), a && a.reset() }, m } function F() { return b || (b = E(null, { useWorker: !0, resize: !0 })), b } n.exports = function () { return F().apply(this, arguments) }, n.exports.reset = function () { F().reset() }, n.exports.create = E }(function () { return void 0 !== t ? t : \"undefined\" != typeof self ? self : this || {} }(), e, !1), t.confetti = e.exports }(window, {});\n        //# sourceMappingURL=/sm/ab60d7fb9bf5b5ded42c77782b65b071de85f56c21da42948364d6b2b1961762.map\n\n    </script>\n\n</body>\n\n</html>"
        )
    }
}

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
