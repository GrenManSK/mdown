use rusqlite::{ params, Connection, OptionalExtension };
use serde_json::Value;
use std::{ io::{ Write, Read }, process::Command, result::Result };

use crate::{ download, error::MdownError, getter, resolute, utils };

include!(concat!(env!("OUT_DIR"), "/data_json.rs"));

fn initialize_db(conn: &Connection) -> Result<(), MdownError> {
    match
        conn.execute(
            "CREATE TABLE IF NOT EXISTS resources (
            id INTEGER PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            data BLOB NOT NULL
            )",
            []
        )
    {
        Ok(_) => (),
        Err(err) => {
            return Err(MdownError::DatabaseError(err));
        }
    }
    Ok(())
}

pub(crate) fn read_resource(conn: &Connection, name: &str) -> Result<Option<Vec<u8>>, MdownError> {
    let mut stmt = match conn.prepare("SELECT data FROM resources WHERE name = ?1") {
        Ok(stmt) => stmt,
        Err(err) => {
            return Err(MdownError::DatabaseError(err));
        }
    };
    match stmt.query_row(params![name], |row| row.get(0)).optional() {
        Ok(result) =>
            match result {
                Some(data) => Ok(Some(data)),
                None => Ok(None),
            }
        Err(err) => {
            return Err(MdownError::DatabaseError(err));
        }
    }
}

fn write_resource(conn: &Connection, name: &str, data: &[u8]) -> Result<u64, MdownError> {
    match
        conn.execute(
            "INSERT INTO resources (name, data) VALUES (?1, ?2)
        ON CONFLICT(name) DO UPDATE SET data = excluded.data",
            params![name, data]
        )
    {
        Ok(_) => {
            let id = conn.last_insert_rowid() as u64;
            Ok(id)
        }
        Err(err) => {
            return Err(MdownError::DatabaseError(err));
        }
    }
}

pub(crate) async fn init() -> std::result::Result<(), MdownError> {
    let db_path = match getter::get_db_path() {
        Ok(path) => path,
        Err(err) => {
            return Err(err);
        }
    };

    let conn = match Connection::open(db_path) {
        Ok(conn) => conn,
        Err(err) => {
            return Err(MdownError::DatabaseError(err));
        }
    };

    match initialize_db(&conn) {
        Ok(_) => (),
        Err(err) => {
            return Err(err);
        }
    }
    let full_path = String::from("yt-dlp_min.exe");

    let mut yt_dlp = false;

    let json_data_string = String::from_utf8_lossy(DATA_JSON).to_string();
    let json_data = match utils::get_json(&json_data_string) {
        Ok(json_data) => json_data,
        Err(err) => {
            return Err(err);
        }
    };

    if let Some(files) = json_data.get("files").and_then(Value::as_array) {
        for file in files.iter() {
            let typ = match file.get("type").and_then(Value::as_str) {
                Some(typ) => typ,
                None => {
                    return Err(MdownError::JsonError(String::from("type not found")));
                }
            };

            if typ == "yt-dlp" {
                let name = match file.get("name").and_then(Value::as_str) {
                    Some(name) => name,
                    None => {
                        return Err(MdownError::JsonError(String::from("name not found")));
                    }
                };
                let db_name = name.replace(".", "_").replace(" ", "_").to_uppercase();

                let db_item = match read_resource(&conn, &db_name) {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(err);
                    }
                };
                if db_item.is_none() {
                    if !yt_dlp {
                        match download_yt_dlp(&full_path).await {
                            Ok(_) => (),
                            Err(err) => {
                                return Err(err);
                            }
                        }
                        yt_dlp = true;
                    }
                    let url = match file.get("url").and_then(Value::as_str) {
                        Some(url) => url,
                        None => {
                            return Err(MdownError::JsonError(String::from("url not found")));
                        }
                    };
                    let dmca = match file.get("dmca").and_then(Value::as_str) {
                        Some(dmca) => dmca,
                        None => {
                            return Err(MdownError::JsonError(String::from("dmca not found")));
                        }
                    };

                    println!("{}", dmca);

                    match
                        Command::new(".\\yt-dlp_min.exe")
                            .arg(url)
                            .arg("--output")
                            .arg(name)
                            .arg("--format")
                            .arg("ba")
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::piped())
                            .spawn()
                    {
                        Ok(mut child) => {
                            let stdout = child.stdout.take().expect("Failed to capture stdout");
                            let stderr = child.stderr.take().expect("Failed to capture stderr");

                            print_output(stdout, "stdout".to_string());
                            print_output(stderr, "stderr".to_string());

                            let status = child.wait().expect("Failed to wait on child");

                            if !status.success() {
                                eprintln!("Process exited with status: {}", status);
                                return Err(
                                    MdownError::IoError(
                                        std::io::Error::new(
                                            std::io::ErrorKind::Other,
                                            "Process failed"
                                        ),
                                        None
                                    )
                                );
                            }
                        }
                        Err(err) => {
                            return Err(MdownError::IoError(err, None));
                        }
                    }

                    let file_bytes = match read_file_to_bytes(name) {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(MdownError::IoError(err, Some(String::from(name))));
                        }
                    };

                    let initial_data_1: &[u8] = &file_bytes;
                    match write_resource(&conn, &db_name, initial_data_1) {
                        Ok(_id) => (),
                        Err(err) => {
                            return Err(err);
                        }
                    }
                    println!("Added {} to database\n", db_name);
                    match std::fs::remove_file(name) {
                        Ok(_) => (),
                        Err(err) => {
                            return Err(MdownError::IoError(err, Some(String::from(name))));
                        }
                    };
                }
            }
        }
    }

    if yt_dlp {
        if std::fs::metadata(&full_path).is_ok() {
            match std::fs::remove_file(&full_path) {
                Ok(_) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, Some(full_path.clone())));
                }
            };
        }
    }

    Ok(())
}

fn print_output<R: Read + Send + 'static>(reader: R, label: String) {
    let mut reader = std::io::BufReader::new(reader);
    let mut buffer = [0; 1024];
    std::thread::spawn(move || {
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    break;
                }
                Ok(n) => {
                    let output = &buffer[..n];
                    for &byte in output {
                        if byte == b'\r' {
                            print!("\r{}: ", label);
                        } else {
                            print!("{}", byte as char);
                        }
                    }
                    use std::io::Write;
                    std::io::stdout().flush().unwrap();
                }
                Err(e) => {
                    eprintln!("Error reading {}: {}", label, e);
                    break;
                }
            }
        }
    });
}

fn read_file_to_bytes(file_path: &str) -> std::io::Result<Vec<u8>> {
    let mut file = match std::fs::File::open(file_path) {
        Ok(file) => file,
        Err(err) => {
            return Err(err);
        }
    };
    let mut buffer = Vec::new();
    match file.read_to_end(&mut buffer) {
        Ok(_) => (),
        Err(err) => {
            return Err(err);
        }
    }
    Ok(buffer)
}

async fn download_yt_dlp(full_path: &str) -> Result<(), MdownError> {
    let client = match download::get_client() {
        Ok(client) => client,
        Err(err) => {
            return Err(MdownError::NetworkError(err));
        }
    };
    let url = "https://github.com/yt-dlp/yt-dlp/releases/download/2024.04.09/yt-dlp_min.exe";

    println!("Fetching {}", url);
    let mut response = match client.get(url).send().await {
        Ok(response) => { response }
        Err(err) => {
            return Err(MdownError::NetworkError(err));
        }
    };
    println!("Fetching {} DONE", url);

    let (total_size, final_size) = download::get_size(&response);

    let mut file = match std::fs::File::create(&full_path) {
        Ok(file) => file,
        Err(err) => {
            return Err(MdownError::IoError(err, Some(full_path.to_string())));
        }
    };
    let (mut downloaded, mut last_size) = (0, 0.0);
    let interval = std::time::Duration::from_millis(100);
    let mut last_check_time = std::time::Instant::now();

    while
        //prettier-ignore
        let Some(chunk) = match response.chunk().await {
                Ok(Some(chunk)) => Some(chunk),
                Ok(None) => None,
                Err(err) => {
                    return Err(MdownError::NetworkError(err));
                }
            }
    {
        match file.write_all(&chunk) {
            Ok(()) => (),
            Err(err) => {
                resolute::SUSPENDED
                    .lock()
                    .push(MdownError::IoError(err, Some(full_path.to_string())));
            }
        }
        downloaded += chunk.len() as u64;
        let current_time = std::time::Instant::now();
        if current_time.duration_since(last_check_time) >= interval {
            let percentage = ((100.0 / (total_size as f32)) * (downloaded as f32)).round() as i64;
            let perc_string = download::get_perc(percentage);
            let message = format!(
                "Downloading yt-dlp_min.exe {}% - {:.2}mb of {:.2}mb [{:.2}mb/s]",
                perc_string,
                (downloaded as f32) / (1024 as f32) / (1024 as f32),
                final_size,
                (((downloaded as f32) - last_size) * 10.0) / (1024 as f32) / (1024 as f32)
            );
            println!("{}", message);
            last_check_time = current_time;
            last_size = downloaded as f32;
        }
    }
    let message = format!(
        "Downloading yt-dlp_min.exe {}% - {:.2}mb of {:.2}mb",
        100,
        (downloaded as f32) / (1024 as f32) / (1024 as f32),
        (total_size as f32) / (1024 as f32) / (1024 as f32)
    );
    println!("{}\n", message);
    Ok(())
}
