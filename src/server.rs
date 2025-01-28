use if_addrs::get_if_addrs;
use serde_json::{ Value, json };
use std::{
    fs::{ self, File },
    io::{ self, BufRead, BufReader, Read, Write },
    net::{ TcpListener, TcpStream },
    path::Path,
    thread,
};
include!(concat!(env!("OUT_DIR"), "/error_404_jpg.rs"));

use crate::{
    args,
    error::MdownError,
    getter::get_query,
    handle_error,
    log,
    utils,
    version_manager::get_current_version,
    zip_func,
};

fn get_directory_content(path: &str) -> Result<Value, MdownError> {
    let mut result = serde_json::Map::new();
    let decoded_str = match percent_encoding::percent_decode_str(path).decode_utf8() {
        Ok(decoded_str) => decoded_str.to_string(),
        Err(err) => {
            return Err(MdownError::ConversionError(err.to_string(), 11200));
        }
    };

    let dir = match fs::read_dir(&decoded_str) {
        Ok(dir) => dir,
        Err(err) => {
            return Err(MdownError::IoError(err, decoded_str, 11201));
        }
    };
    for entry in dir {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                return Err(MdownError::IoError(err, decoded_str, 11202));
            }
        };
        let file_name = match entry.file_name().into_string() {
            Ok(file_name) => file_name,
            Err(_err) => {
                return Err(
                    MdownError::ConversionError(
                        String::from("Failed to convert file name to string"),
                        11203
                    )
                );
            }
        };
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(err) => {
                return Err(MdownError::IoError(err, file_name, 11204));
            }
        };
        let mut file_info =
            json!({
            "size": metadata.len(),
            "modified": match metadata.modified() {
                Ok(value) => value,
                Err(err) => {
                    return Err(MdownError::IoError(err, file_name, 11205));
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
                                String::from("Could not get file_info as mutable object"),
                                11206
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

fn handle_client(stream: TcpStream) -> Result<(), MdownError> {
    let mut stream = BufReader::new(stream);
    let mut request_line = String::new();
    match stream.read_line(&mut request_line) {
        Ok(_n) => (),
        Err(err) => {
            return Err(MdownError::IoError(err, String::new(), 11207));
        }
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    let path = request_line.split_whitespace().nth(1).unwrap_or("/");
    if parts.len() >= 2 {
        let query_params = get_query(parts);
        if path.starts_with("/__search__") {
            let file_path: String = if path.starts_with("/__search__?") {
                match query_params.get("path").cloned() {
                    Some(value) => value,
                    None => String::from("."),
                }
            } else {
                String::from(".")
            };
            let json_response = match get_directory_content(&file_path) {
                Ok(value) => value,
                Err(err) => {
                    return Err(MdownError::JsonError(err.to_string(), 11208));
                }
            };
            let response_body = match serde_json::to_string(&json_response) {
                Ok(value) => value,
                Err(err) => {
                    return Err(MdownError::JsonError(err.to_string(), 11209));
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
                    return Err(MdownError::IoError(err, String::new(), 11210));
                }
            };
        } else if path.starts_with("/__preview__?") {
            let file_path = match query_params.get("path").cloned() {
                Some(value) => format!(".\\{}", value),
                None => {
                    return Ok(());
                }
            };

            let decoded_str = match percent_encoding::percent_decode_str(&file_path).decode_utf8() {
                Ok(decoded_str) => decoded_str.to_string().replace("./", "").replace("/", ""),
                Err(err) => {
                    return Err(MdownError::ConversionError(err.to_string(), 11211));
                }
            };

            let contents = if decoded_str.ends_with(".cbz") {
                match zip_func::extract_image_from_zip(&decoded_str) {
                    Ok(contents) => contents,
                    Err(err) => {
                        return Err(MdownError::ChainedError(Box::new(err), 11236));
                    }
                }
            } else {
                match fs::read(&decoded_str) {
                    Ok(contents) => contents,
                    Err(err) => {
                        return Err(MdownError::IoError(err, decoded_str, 11212));
                    }
                }
            };

            let mut response = String::new();
            response.push_str("HTTP/1.1 200 OK\r\n");
            response.push_str("Content-Type: image/png\r\n");
            response.push_str("Content-Length: ");
            response.push_str(&contents.len().to_string());
            response.push_str("\r\n\r\n");

            match stream.get_mut().write_all(response.as_bytes()) {
                Ok(_n) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, String::new(), 11213));
                }
            }
            match stream.get_mut().write_all(&contents) {
                Ok(_n) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, String::new(), 11214));
                }
            }
        } else if path.starts_with("/__download__?") {
            let file_path = match query_params.get("path").cloned() {
                Some(value) => value,
                None => {
                    return Ok(());
                }
            };

            let mut decoded_str = match
                percent_encoding::percent_decode_str(&file_path).decode_utf8()
            {
                Ok(decoded_str) => decoded_str.to_string(),
                Err(err) => {
                    return Err(MdownError::ConversionError(err.to_string(), 11215));
                }
            };

            if decoded_str.ends_with('/') {
                decoded_str.pop();
            }

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
                    return Err(MdownError::IoError(err, dst_file, 11216));
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
                    return Err(MdownError::IoError(err, String::new(), 11217));
                }
            }
            match stream.get_mut().write_all(&contents) {
                Ok(_n) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, String::new(), 11218));
                }
            }

            match fs::remove_file(&dst_file) {
                Ok(_) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, dst_file, 11219));
                }
            };
        } else if path.starts_with("/__version__") {
            let response = format!(
                "{}{}",
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n",
                get_current_version()
            );
            match stream.get_mut().write_all(response.as_bytes()) {
                Ok(_n) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, String::new(), 11220));
                }
            };
        } else if path.starts_with("/__get__?") {
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
                            String::from("Resource"),
                            11221
                        )
                    );
                }
            };
            match stream.get_mut().write_all(content) {
                Ok(_n) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, String::new(), 11223));
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
                    return Err(MdownError::IoError(err, String::new(), 11224));
                }
            };
        } else {
            let decoded_str = match percent_encoding::percent_decode_str(path).decode_utf8() {
                Ok(decoded_str) => decoded_str.to_string(),
                Err(err) => {
                    return Err(MdownError::ConversionError(err.to_string(), 11225));
                }
            };
            let file_path = format!(".{}", decoded_str);
            if Path::new(&file_path).is_file() {
                let contents = match fs::read(&file_path) {
                    Ok(contents) => contents,
                    Err(err) => {
                        return Err(MdownError::IoError(err, String::new(), 11226));
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
                        return Err(MdownError::IoError(err, String::new(), 11227));
                    }
                }
                match stream.get_mut().write_all(&contents) {
                    Ok(_n) => (),
                    Err(err) => {
                        return Err(MdownError::IoError(err, String::new(), 11228));
                    }
                };
            } else {
                let response = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
                match stream.get_mut().write_all(response.as_bytes()) {
                    Ok(_n) => (),
                    Err(err) => {
                        return Err(MdownError::IoError(err, String::new(), 11229));
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
            "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"UTF-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"><title>404 Error - Page Not Found</title><style>body {font-family: Arial, sans-serif;background-color: #f7f7f7;color: #333;margin: 0;padding: 0;text-align: center;}.container {position: absolute;top: 50%;left: 50%;transform: translate(-50%, -50%);}h1 {font-size: 36px;margin-bottom: 20px;}p {font-size: 18px;margin-bottom: 20px;}a {color: #007bff;text-decoration: none;}a:hover {text-decoration: underline;}</style></head><body><div class=\"container\"><h1>404 Error - Page Not Found</h1><p>The page you are looking for might have been removed, had its name changed, or is temporarily unavailable.</p><p>Go back to <a href=\"/\">home page</a>.</p></div></body></html>"
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
            "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"UTF-8\" /><meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" /><title>File Manager</title><style>body {font-family: Arial, sans-serif;background-color: #121212;color: #fff;margin: 0;padding: 0;display: grid;justify-content: center;align-items: center;height: 100vh;}h2 {font-size: 40px;margin-left: 20px;}.container {width: 80%;max-width: 800px;background-color: #272727;padding: 20px;box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);border-radius: 8px;display: flex;flex-direction: column;align-items: center;}.file-list {list-style-type: none;padding: 0;width: 100%;}.file-list li {margin-bottom: 5px;}.file-list li a {text-decoration: none;color: lightblue;cursor: pointer;}.file-info {border: 1px solid #555;padding: 10px;margin-top: 20px;width: 100%;background-color: #333;}#header {display: flex;align-items: center;}#version {margin-left: 5px;}.controls {display: flex;gap: 10px;margin-top: 10px;width: 100%;}.controls input,.controls button {flex: 1;}input {width: 100%;padding: 10px;margin-bottom: 16px;box-sizing: border-box;border: 1px solid #555;border-radius: 4px;background-color: #333;color: #fff;}.download {background-color: #4caf50;color: #fff;padding: 10px 15px;border: none;border-radius: 4px;cursor: pointer;transition: background-color 0.5s;}.download:hover {background-color: #45a049;}.button {background-color: white;transition: background-color 0.5s;padding: 10px 15px;border: none;border-radius: 4px;cursor: pointer;}.button:hover {background-color: lightgrey;}</style></head><body><div class=\"container\"><h2 id=\"header\">File Manager Mdown<p id=\"version\"></p></h2><div><label for=\"ipAddress\">Enter IP Address:</label><input type=\"text\" id=\"ipAddress\" /><button class=\"button\"onclick=\"fetchFiles()\">Connect</button><button class=\"button\" onclick=\"goToParentDirectory()\">Parent Directory</button><button class=\"download\" onclick=\"downloadAsZip()\">Download As ZIP</button></div><ul class=\"file-list\" id=\"fileList\"></ul><div class=\"file-info\" id=\"fileInfo\"></div></div><script>fetch(\"__version__\").then((response) => {if (!response.ok) {throw new Error(\"Network response was not ok\");}return response.text();}).then((text) => {document.getElementById(\"version\").textContent = `v${text}`;}).catch((error) => {console.error(\"There was a problem fetching the text:\", error);});var path_hist = \"\";function displayFiles(files) {const fileList = document.getElementById(\"fileList\");fileList.innerHTML = \"\";const directories = [];const regularFiles = [];for (const key in files) {const file = files[key];if (file.type === \"directory\") {directories.push(file);} else {regularFiles.push(file);}}directories.sort((a, b) => a.path.localeCompare(b.path));regularFiles.sort((a, b) => a.path.localeCompare(b.path));const sortedFiles = [...directories, ...regularFiles];sortedFiles.forEach((file) => {const listItem = document.createElement(\"li\");const link = document.createElement(\"a\");link.setAttribute(\"data-isDir\", file.type === \"directory\");link.setAttribute(\"data-path\", file.path);link.textContent = file.path;link.addEventListener(\"click\", () => {const fileInfo = document.getElementById(\"fileInfo\");fileInfo.innerHTML = \"\";if (file.type === \"directory\") {fetchFiles(path_hist + file.path);} else {displayFileInfo(file);}});listItem.appendChild(link);fileList.appendChild(listItem);});}function displayFileInfo(file) {const encoded_path = encodeURIComponent(path_hist + \"\\\\\" + file.path);const fileInfo = document.getElementById(\"fileInfo\");const milliseconds =file.modified.secs_since_epoch * 1000 +Math.round(file.modified.nanos_since_epoch / 1000000);let content = `<h3>File Details</h3><p>Name: ${file.path}</p><p>Size: ${file.size} bytes</p><p>Last Modified: ${new Date(milliseconds).toLocaleString()}</p><img src=\"__preview__?path=${encoded_path}\" alt=\"\" style=\"width: inherit;\">`;if (file.type !== \"directory\") {content += `<a href=\"http://${document.getElementById(\"ipAddress\").value}:3000/${path_hist + file.path}\" download style=\"color: #fff;>Download</a>`;}fileInfo.innerHTML = content;}function fetchFiles(path = \".\") {const encoded_path = encodeURIComponent(path);const ipAddress = document.getElementById(\"ipAddress\").value;if (!ipAddress) {alert(\"Please enter an IP address.\");return;}fetch(`http://${ipAddress}:3000/__search__?path=${encoded_path}`).then((response) => response.json()).then((data) => {displayFiles(data);}).catch((error) => {alert(\"Failed to fetch files. Please try again later.\");console.error(\"Error:\", error);});path_hist = path + \"/\";}function goToParentDirectory() {const ipAddress = document.getElementById(\"ipAddress\").value;var currentPath = path_hist.split(\"/\").slice(0, -2).join(\"/\") + \"/\";if (currentPath == \"/\") {currentPath = \"./\";}path_hist = currentPath;const encoded_path = encodeURIComponent(currentPath);fetch(`http://${ipAddress}:3000/__search__?path=${encoded_path}`).then((response) => response.json()).then((data) => {displayFiles(data);}).catch((error) => {alert(\"Failed to fetch files. Please try again later.\");console.error(\"Error:\", error);});}function downloadAsZip() {const ipAddress = document.getElementById(\"ipAddress\").value;const currentPath = path_hist;if (!ipAddress) {alert(\"Please enter an IP address.\");return;}fetch(`http://${ipAddress}:3000/__download__?path=${encodeURIComponent(currentPath)}`,{ method: \"GET\" }).then((response) => {const headers = response.headers.get(\"content-disposition\");const filenameRegex = /filename=[\"\']?([^\"\']+)/;const matches = headers.match(filenameRegex);const filename = matches ? matches[1] : null;return Promise.all([response.blob(), filename]);}).then(([blob, filename]) => {const url = window.URL.createObjectURL(new Blob([blob]));const link = document.createElement(\"a\");link.href = url;link.setAttribute(\"download\", `${filename}`);document.body.appendChild(link);link.click();link.parentNode.removeChild(link);}).catch((error) => {alert(\"Failed to download files as ZIP. Please try again later.\");console.error(\"Error:\", error);});}</script></body></html>"
        )
    }
}

pub(crate) fn start() -> Result<(), MdownError> {
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
            return Err(MdownError::IoError(err, String::new(), 11230));
        }
    }

    let mut input = String::new();

    match io::stdin().read_line(&mut input) {
        Ok(_) => (),
        Err(err) => {
            return Err(MdownError::IoError(err, String::new(), 11231));
        }
    }

    let number: usize = match input.trim().parse() {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::ConversionError(err.to_string(), 11232));
        }
    };

    let ip_address = match ips.get(number - 1) {
        Some(value) => value,
        None => {
            return Err(
                MdownError::CustomError(
                    String::from("Invalid IP address"),
                    String::from("IP_address"),
                    11233
                )
            );
        }
    };

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
                    11234
                )
            );
        }
    }

    let listener = match TcpListener::bind(format!("{}:3000", ip_address)) {
        Ok(listener) => listener,
        Err(err) => {
            return Err(MdownError::IoError(err, String::new(), 11235));
        }
    };
    println!("Server listening on {}:3000 ...", ip_address);

    let url = format!("http://{}:3000/", ip_address);
    if let Err(err) = webbrowser::open(&url) {
        eprintln!("Error opening web browser: {}", err);
    }

    for stream in listener.incoming().flatten() {
        thread::spawn(move || {
            if let Err(err) = handle_client(stream) {
                eprintln!("Error handling client: {}", err);
            }
        });
    }

    Ok(())
}
