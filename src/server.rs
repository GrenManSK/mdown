use if_addrs::get_if_addrs;
use serde_json::{ Value, json };
use std::{
    fs::{ self, File },
    io::{ self, BufRead, BufReader, Write, Read },
    net::{ TcpListener, TcpStream },
    path::Path,
    thread,
};
include!(concat!(env!("OUT_DIR"), "/error_404_jpg.rs"));

use crate::{ args, error::MdownError, getter::get_query, handle_error, log, utils, zip_func };

fn get_directory_content(path: &str) -> std::result::Result<Value, MdownError> {
    let mut result = serde_json::Map::new();
    let decoded_str = match percent_encoding::percent_decode_str(path).decode_utf8() {
        Ok(decoded_str) => decoded_str.to_string(),
        Err(err) => {
            return Err(MdownError::ConversionError(err.to_string()));
        }
    };

    let dir = match fs::read_dir(&decoded_str) {
        Ok(dir) => dir,
        Err(err) => {
            return Err(MdownError::IoError(err, decoded_str));
        }
    };
    for entry in dir {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                return Err(MdownError::IoError(err, decoded_str));
            }
        };
        let file_name = match entry.file_name().into_string() {
            Ok(file_name) => file_name,
            Err(_err) => {
                return Err(
                    MdownError::ConversionError(
                        String::from("Failed to convert file name to string")
                    )
                );
            }
        };
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(err) => {
                return Err(MdownError::IoError(err, file_name));
            }
        };
        let mut file_info =
            json!({
            "size": metadata.len(),
            "modified": match metadata.modified() {
                Ok(value) => value,
                Err(err) => {
                    return Err(MdownError::IoError(err, file_name));
                }
            },
            "path": file_name,
            "type": if metadata.is_dir() { "directory" } else { "file" }
        });

        if metadata.is_dir() {
            if let Ok(sub_dir_content) = get_directory_content(&entry.path().to_string_lossy()) {
                match file_info.as_object_mut() {
                    Some(value) => value.insert("content".to_string(), sub_dir_content),
                    None => {
                        return Err(
                            MdownError::NotFoundError(
                                String::from("Could not get file_info as mutable object")
                            )
                        );
                    }
                };
            }
        }

        result.insert(file_name, file_info);
    }

    Ok(Value::Object(result))
}

fn handle_client(stream: TcpStream) -> std::result::Result<(), MdownError> {
    let mut stream = BufReader::new(stream);
    let mut request_line = String::new();
    match stream.read_line(&mut request_line) {
        Ok(_n) => (),
        Err(err) => {
            return Err(MdownError::IoError(err, String::new()));
        }
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    let path = match request_line.split_whitespace().nth(1) {
        Some(value) => value,
        None => "/",
    };
    if parts.len() >= 2 {
        if path.starts_with("/__search__") {
            let file_path: String;
            if path.starts_with("/__search__?") {
                let query_params = get_query(parts);
                file_path = match query_params.get("path").cloned() {
                    Some(value) => value,
                    None => String::from("."),
                };
            } else {
                file_path = String::from(".");
            }
            let json_response = match get_directory_content(&file_path) {
                Ok(value) => value,
                Err(err) => {
                    return Err(MdownError::JsonError(err.to_string()));
                }
            };
            let response_body = match serde_json::to_string(&json_response) {
                Ok(value) => value,
                Err(err) => {
                    return Err(MdownError::JsonError(err.to_string()));
                }
            };
            let mut response = String::new();
            response.push_str("HTTP/1.1 200 OK\r\n");
            response.push_str("Content-Type: application/json\r\n");
            response.push_str("Access-Control-Allow-Origin: *\r\n");
            response.push_str(&format!("Content-Length: {}\r\n\r\n", response_body.len()));
            response.push_str(&response_body);
            match stream.get_mut().write_all(response.as_bytes()) {
                Ok(_n) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, String::new()));
                }
            };
        } else if path.starts_with("/__preview__?") {
            let query_params = get_query(parts);
            let file_path = match query_params.get("path").cloned() {
                Some(value) => format!(".\\{}", value),
                None => {
                    return Ok(());
                }
            };

            let decoded_str = match percent_encoding::percent_decode_str(&file_path).decode_utf8() {
                Ok(decoded_str) => decoded_str.to_string().replace("./", "").replace("/", ""),
                Err(err) => {
                    return Err(MdownError::ConversionError(err.to_string()));
                }
            };

            let contents;
            if decoded_str.ends_with(".cbz") {
                contents = match zip_func::extract_image_from_zip(&decoded_str) {
                    Ok(contents) => contents,
                    Err(err) => {
                        return Err(err);
                    }
                };
            } else {
                contents = match fs::read(&decoded_str) {
                    Ok(contents) => contents,
                    Err(err) => {
                        return Err(MdownError::IoError(err, decoded_str));
                    }
                };
            }

            let mut response = String::new();
            response.push_str("HTTP/1.1 200 OK\r\n");
            response.push_str("Content-Type: image/png\r\n");
            response.push_str("Content-Length: ");
            response.push_str(&contents.len().to_string());
            response.push_str("\r\n\r\n");

            match stream.get_mut().write_all(response.as_bytes()) {
                Ok(_n) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, String::new()));
                }
            }
            match stream.get_mut().write_all(&contents) {
                Ok(_n) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, String::new()));
                }
            }
        } else if path.starts_with("/__download__?") {
            let query_params = get_query(parts);
            let file_path = match query_params.get("path").cloned() {
                Some(value) => value,
                None => {
                    return Ok(());
                }
            };

            let decoded_str = match percent_encoding::percent_decode_str(&file_path).decode_utf8() {
                Ok(decoded_str) => decoded_str.to_string().replace("./", "").replace("/", ""),
                Err(err) => {
                    return Err(MdownError::ConversionError(err.to_string()));
                }
            };

            let dst_file = match decoded_str.split('/').last() {
                Some(value) => format!("{}.zip", value),
                None => {
                    return Ok(());
                }
            };

            zip_func::to_zip(&decoded_str, &dst_file);

            let contents = match fs::read(&dst_file) {
                Ok(contents) => contents,
                Err(err) => {
                    return Err(MdownError::IoError(err, dst_file));
                }
            };
            let mut response = String::new();
            response.push_str("HTTP/1.1 200 OK\r\n");
            response.push_str("Content-Disposition: attachment; filename=\"");
            response.push_str(&dst_file);
            response.push_str("\"\r\n");
            response.push_str("Content-Type: application/octet-stream\r\n");
            response.push_str("Content-Length: ");
            response.push_str(&contents.len().to_string());
            response.push_str("\r\n\r\n");
            match stream.get_mut().write_all(response.as_bytes()) {
                Ok(_n) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, String::new()));
                }
            }
            match stream.get_mut().write_all(&contents) {
                Ok(_n) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, String::new()));
                }
            }

            match fs::remove_file(&dst_file) {
                Ok(_) => {}
                Err(err) => {
                    return Err(MdownError::IoError(err, dst_file));
                }
            };
        } else if path.starts_with("/__version__") {
            let response = format!(
                "{}{}",
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n",
                env!("CARGO_PKG_VERSION")
            );
            match stream.get_mut().write_all(response.as_bytes()) {
                Ok(_n) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, String::new()));
                }
            };
        } else if path.starts_with("/__get__?") {
            let query_params = get_query(parts);
            let file_path = match query_params.get("path").cloned() {
                Some(value) => value,
                None => {
                    return Ok(());
                }
            };

            let content = match file_path.as_str() {
                "error_404" => ERROR_404_JPG,
                _ => {
                    return Err(
                        MdownError::CustomError(
                            String::from("Didn't find resource"),
                            String::from("Resource")
                        )
                    );
                }
            };
            match stream.get_mut().write_all(&content) {
                Ok(_n) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, String::new()));
                }
            }
        } else if path == "/" {
            let html = get_html();
            let response = format!(
                "{}{}",
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n",
                html
            );
            match stream.get_mut().write_all(response.as_bytes()) {
                Ok(_n) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, String::new()));
                }
            };
        } else {
            let decoded_str = match percent_encoding::percent_decode_str(path).decode_utf8() {
                Ok(decoded_str) => decoded_str.to_string(),
                Err(err) => {
                    return Err(MdownError::ConversionError(err.to_string()));
                }
            };
            let file_path = format!(".{}", decoded_str);
            if Path::new(&file_path).is_file() {
                let contents = match fs::read(&file_path) {
                    Ok(contents) => contents,
                    Err(err) => {
                        return Err(MdownError::IoError(err, String::new()));
                    }
                };
                let mut response = String::new();
                let filename = match file_path.split("/").last() {
                    Some(value) => value.to_owned(),
                    None => format!("{}.cbz", utils::generate_random_id(16)),
                };
                response.push_str("HTTP/1.1 200 OK\r\n");
                response.push_str("Content-Disposition: attachment; filename=");
                response.push_str(&filename);
                response.push_str("\r\n");
                response.push_str("Content-Type: application/octet-stream\r\n");
                response.push_str("Content-Length: ");
                response.push_str(&contents.len().to_string());
                response.push_str("\r\n\r\n");
                match stream.get_mut().write_all(response.as_bytes()) {
                    Ok(_n) => (),
                    Err(err) => {
                        return Err(MdownError::IoError(err, String::new()));
                    }
                }
                match stream.get_mut().write_all(&contents) {
                    Ok(_n) => (),
                    Err(err) => {
                        return Err(MdownError::IoError(err, String::new()));
                    }
                };
            } else {
                let response = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
                match stream.get_mut().write_all(response.as_bytes()) {
                    Ok(_n) => (),
                    Err(err) => {
                        return Err(MdownError::IoError(err, String::new()));
                    }
                };
            }
        }
    }

    Ok(())
}

fn get_html() -> String {
    if *args::ARGS_DEV {
        let err_404 = String::from(
            "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n    <meta charset=\"UTF-8\">\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n    <title>404 Error - Page Not Found</title>\n    <style>\n        body {\n            font-family: Arial, sans-serif;\n            background-color: #f7f7f7;\n            color: #333;\n            margin: 0;\n            padding: 0;\n            text-align: center;\n        }\n        .container {\n            position: absolute;\n            top: 50%;\n            left: 50%;\n            transform: translate(-50%, -50%);\n        }\n        h1 {\n            font-size: 36px;\n            margin-bottom: 20px;\n        }\n        p {\n            font-size: 18px;\n            margin-bottom: 20px;\n        }\n        a {\n            color: #007bff;\n            text-decoration: none;\n        }\n        a:hover {\n            text-decoration: underline;\n        }\n    </style>\n</head>\n<body>\n    <div class=\"container\">\n        <h1>404 Error - Page Not Found</h1>\n        <p>The page you are looking for might have been removed, had its name changed, or is temporarily unavailable.</p>\n        <p>Go back to <a href=\"/\">home page</a>.</p>\n    </div>\n</body>\n</html>\n"
        );
        let mut file = match File::open("server.html") {
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
            "<!DOCTYPE html>\n<html lang=\"en\">\n\n<head>\n    <meta charset=\"UTF-8\">\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n    <title>File Manager</title>\n    <style>\n        body {\n            font-family: Arial, sans-serif;\n            background-color: #121212;\n            color: #fff;\n            margin: 0;\n            padding: 0;\n            display: grid;\n            justify-content: center;\n            align-items: center;\n            height: 100vh;\n        }\n\n        h2 {\n            font-size: 40px;\n            margin-left: 20px;\n        }\n\n        .container {\n            width: 80%;\n            max-width: 800px;\n            background-color: #272727;\n            padding: 20px;\n            box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);\n            border-radius: 8px;\n            display: flex;\n            flex-direction: column;\n            align-items: center;\n        }\n\n        .file-list {\n            list-style-type: none;\n            padding: 0;\n            width: 100%;\n        }\n\n        .file-list li {\n            margin-bottom: 5px;\n        }\n\n        .file-list li a {\n            text-decoration: none;\n            color: lightblue;\n            cursor: pointer;\n        }\n\n        .file-info {\n            border: 1px solid #555;\n            padding: 10px;\n            margin-top: 20px;\n            width: 100%;\n            background-color: #333;\n        }\n\n        #header {\n            display: flex;\n            align-items: center;\n        }\n\n        #version {\n            margin-left: 5px;\n        }\n\n        .controls {\n            display: flex;\n            gap: 10px;\n            margin-top: 10px;\n            width: 100%;\n        }\n\n        .controls input,\n        .controls button {\n            flex: 1;\n        }\n\n        input {\n            width: 100%;\n            padding: 10px;\n            margin-bottom: 16px;\n            box-sizing: border-box;\n            border: 1px solid #555;\n            border-radius: 4px;\n            background-color: #333;\n            color: #fff;\n        }\n\n        .download {\n            background-color: #4caf50;\n            color: #fff;\n            padding: 10px 15px;\n            border: none;\n            border-radius: 4px;\n            cursor: pointer;\n            transition: background-color 0.5s;\n        }\n\n        .download:hover {\n            background-color: #45a049;\n\n        }\n\n        .button {\n            background-color: white;\n            transition: background-color 0.5s;\n            padding: 10px 15px;\n            border: none;\n            border-radius: 4px;\n            cursor: pointer;\n        }\n\n        .button:hover {\n            background-color: lightgrey;\n\n        }\n    </style>\n</head>\n\n<body>\n\n    <div class=\"container\">\n        <h2 id=\"header\">File Manager Mdown <p id=\"version\"></p>\n        </h2>\n        <div>\n            <label for=\"ipAddress\">Enter IP Address:</label>\n            <input type=\"text\" id=\"ipAddress\">\n            <button class=\"button\" onclick=\"fetchFiles()\">Connect</button>\n            <button class=\"button\" onclick=\"goToParentDirectory()\">Parent Directory</button>\n            <button class=\"download\" onclick=\"downloadAsZip()\">Download As ZIP</button>\n        </div>\n        <ul class=\"file-list\" id=\"fileList\"></ul>\n        <div class=\"file-info\" id=\"fileInfo\"></div>\n    </div>\n    <script>\n        fetch(\'__version__\')\n            .then(response => {\n                if (!response.ok) {\n                    throw new Error(\'Network response was not ok\');\n                }\n                return response.text();\n            })\n            .then(text => {\n                document.getElementById(\'version\').textContent = `v${text}`;\n            })\n            .catch(error => {\n                console.error(\'There was a problem fetching the text:\', error);\n            });\n\n        var path_hist = \"\";\n\n        function displayFiles(files) {\n            const fileList = document.getElementById(\'fileList\');\n            fileList.innerHTML = \'\';\n\n            const directories = [];\n            const regularFiles = [];\n\n            for (const key in files) {\n                const file = files[key];\n                if (file.type === \'directory\') {\n                    directories.push(file);\n                } else {\n                    regularFiles.push(file);\n                }\n            }\n\n            directories.sort((a, b) => a.path.localeCompare(b.path));\n            regularFiles.sort((a, b) => a.path.localeCompare(b.path));\n\n            const sortedFiles = [...directories, ...regularFiles];\n\n            sortedFiles.forEach(file => {\n                const listItem = document.createElement(\'li\');\n                const link = document.createElement(\'a\');\n                link.setAttribute(\'data-isDir\', file.type === \'directory\');\n                link.setAttribute(\'data-path\', file.path);\n                link.textContent = file.path;\n                link.addEventListener(\'click\', () => {\n                    const fileInfo = document.getElementById(\'fileInfo\');\n                    fileInfo.innerHTML = \"\";\n                    if (file.type === \'directory\') {\n                        fetchFiles(path_hist + file.path);\n                    } else {\n                        displayFileInfo(file);\n                    }\n                });\n                listItem.appendChild(link);\n                fileList.appendChild(listItem);\n            });\n        }\n\n        function displayFileInfo(file) {\n\n            const encoded_path = encodeURIComponent(path_hist + \"\\\\\" + file.path);\n            const fileInfo = document.getElementById(\'fileInfo\');\n\n            const milliseconds = file.modified.secs_since_epoch * 1000 + Math.round(file.modified.nanos_since_epoch / 1000000);\n            let content = `\n                <h3>File Details</h3>\n                <p>Name: ${file.path}</p>\n                <p>Size: ${file.size} bytes</p>\n                <p>Last Modified: ${new Date(milliseconds).toLocaleString()}</p>\n                <img src=\"__preview__?path=${encoded_path}\" alt=\"\">\n            `;\n            if (file.type !== \'directory\') {\n                content += `<a href=\"http://${document.getElementById(\'ipAddress\').value}:3000/${path_hist + file.path}\" download style=\"color: #fff;>Download</a>`;\n            }\n            fileInfo.innerHTML = content;\n        }\n\n        function fetchFiles(path = \'.\') {\n            const encoded_path = encodeURIComponent(path);\n            const ipAddress = document.getElementById(\'ipAddress\').value;\n            if (!ipAddress) {\n                alert(\'Please enter an IP address.\');\n                return;\n            }\n\n            fetch(`http://${ipAddress}:3000/__search__?path=${encoded_path}`)\n                .then(response => response.json())\n                .then(data => {\n                    displayFiles(data);\n                })\n                .catch(error => {\n                    alert(\'Failed to fetch files. Please try again later.\');\n                    console.error(\'Error:\', error);\n                });\n            path_hist = path + \'/\';\n        }\n\n        function goToParentDirectory() {\n            const ipAddress = document.getElementById(\'ipAddress\').value;\n            var currentPath = path_hist.split(\'/\').slice(0, -2).join(\'/\') + \"/\";\n            if (currentPath == \'/\') {\n                currentPath = \"./\";\n            }\n            path_hist = currentPath;\n\n            const encoded_path = encodeURIComponent(currentPath);\n            fetch(`http://${ipAddress}:3000/__search__?path=${encoded_path}`)\n                .then(response => response.json())\n                .then(data => {\n                    displayFiles(data);\n                })\n                .catch(error => {\n                    alert(\'Failed to fetch files. Please try again later.\');\n                    console.error(\'Error:\', error);\n                });\n        }\n\n        function downloadAsZip() {\n            const ipAddress = document.getElementById(\'ipAddress\').value;\n            const currentPath = path_hist;\n            if (!ipAddress) {\n                alert(\'Please enter an IP address.\');\n                return;\n            }\n\n            fetch(`http://${ipAddress}:3000/__download__?path=${encodeURIComponent(currentPath)}`, {\n                method: \'GET\'\n            })\n                .then(response => {\n                    const headers = response.headers.get(\"content-disposition\");\n                    const filenameRegex = /filename=[\"\']?([^\"\']+)/;\n                    const matches = headers.match(filenameRegex);\n                    const filename = matches ? matches[1] : null;\n                    return Promise.all([response.blob(), filename]);\n                })\n                .then(([blob, filename]) => {\n                    const url = window.URL.createObjectURL(new Blob([blob]));\n                    const link = document.createElement(\'a\');\n                    link.href = url;\n                    link.setAttribute(\'download\', `${filename}`);\n                    document.body.appendChild(link);\n                    link.click();\n                    link.parentNode.removeChild(link);\n                })\n                .catch(error => {\n                    alert(\'Failed to download files as ZIP. Please try again later.\');\n                    console.error(\'Error:\', error);\n                });\n        }\n    </script>\n</body>\n\n</html>"
        )
    }
}

pub(crate) fn start() -> std::result::Result<(), MdownError> {
    let mut ips = vec![];
    if let Ok(interfaces) = get_if_addrs() {
        for (times, interface) in interfaces.iter().enumerate() {
            println!("{}) {}", times + 1, interface.ip());
            ips.push(interface.ip().to_string());
        }
    } else {
        println!("Unable to retrieve interface addresses");
    }

    print!("> ");
    match io::stdout().flush() {
        Ok(_) => (),
        Err(err) => {
            return Err(MdownError::IoError(err, String::new()));
        }
    }

    let mut input = String::new();

    match io::stdin().read_line(&mut input) {
        Ok(_) => (),
        Err(err) => {
            return Err(MdownError::IoError(err, String::new()));
        }
    }

    let number: usize = match input.trim().parse() {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::ConversionError(err.to_string()));
        }
    };

    let ip_address = match ips.get(number - 1) {
        Some(value) => value,
        None => {
            return Err(
                MdownError::CustomError(
                    String::from("Invalid IP address"),
                    String::from("IP_address")
                )
            );
        }
    };

    match
        ctrlc::set_handler(|| {
            log!("[user] Ctrl+C received! Exiting...");
            log!("[web] Closing server");

            match utils::remove_cache() {
                Ok(()) => (),
                Err(err) => {
                    handle_error!(&err, String::from("ctrl_handler"));
                }
            }
            std::process::exit(0);
        })
    {
        Ok(()) => (),
        Err(err) => {
            return Err(
                MdownError::CustomError(
                    format!("Failed setting up ctrl handler, {}", err.to_string()),
                    String::from("CTRL_handler")
                )
            );
        }
    }

    let listener = match TcpListener::bind(format!("{}:3000", ip_address)) {
        Ok(listener) => listener,
        Err(err) => {
            return Err(MdownError::IoError(err, String::new()));
        }
    };
    println!("Server listening on {}:3000 ...", ip_address);

    let url = format!("http://{}:3000/", ip_address);
    if let Err(err) = webbrowser::open(&url) {
        eprintln!("Error opening web browser: {}", err);
    }

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            thread::spawn(move || {
                if let Err(err) = handle_client(stream) {
                    eprintln!("Error handling client: {}", err);
                }
            });
        }
    }

    Ok(())
}
