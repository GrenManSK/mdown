use rusqlite::{ params, Connection, OptionalExtension };
use std::{ io::{ Read, Write }, process::Command, result::Result };

use crate::{ args, download, error::MdownError, getter, resolute, metadata::DB };

include!(concat!(env!("OUT_DIR"), "/data_json.rs"));

fn initialize_db(conn: &Connection) -> Result<(), MdownError> {
    match
        conn.execute(
            "CREATE TABLE IF NOT EXISTS resources (
            id INTEGER PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            data TEXT NOT NULL,
            is_binary BOOLEAN NOT NULL
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
    let mut stmt = match conn.prepare("SELECT data, is_binary FROM resources WHERE name = ?1") {
        Ok(stmt) => stmt,
        Err(err) => {
            return Err(MdownError::DatabaseError(err));
        }
    };

    match
        stmt
            .query_row(params![name], |row| {
                let data: String = row.get(0).unwrap();
                let is_binary: bool = row.get(1).unwrap();

                if is_binary {
                    #[allow(deprecated)]
                    let decoded_data = base64
                        ::decode(&data)
                        .map_err(|e|
                            MdownError::CustomError(e.to_string(), String::from("Base64Error"))
                        )
                        .unwrap();
                    Ok(Some(decoded_data))
                } else {
                    Ok(Some(data.into_bytes()))
                }
            })
            .optional()
    {
        Ok(result) =>
            Ok(match result {
                Some(data) => data,
                None => None,
            }),
        Err(err) => Err(MdownError::DatabaseError(err)),
    }
}

fn write_resource(
    conn: &Connection,
    name: &str,
    data: &[u8],
    is_binary: bool
) -> Result<u64, MdownError> {
    let data_str = if is_binary {
        #[allow(deprecated)]
        base64::encode(data)
    } else {
        String::from_utf8(data.to_vec())
            .map_err(|e| MdownError::CustomError(e.to_string(), String::from("Base64Error")))
            .unwrap()
    };

    match
        conn.execute(
            "INSERT INTO resources (name, data, is_binary) VALUES (?1, ?2, ?3)
                ON CONFLICT(name) DO UPDATE SET data = excluded.data, is_binary = excluded.is_binary",
            params![name, data_str, is_binary]
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

fn delete_resource(conn: &Connection, name: &str) -> Result<(), MdownError> {
    match conn.execute("DELETE FROM resources WHERE name = ?1", params![name]) {
        Ok(_) => Ok(()),
        Err(err) => Err(MdownError::DatabaseError(err)),
    }
}

pub(crate) async fn init() -> std::result::Result<(), MdownError> {
    let db_path = match getter::get_db_path() {
        Ok(path) => path,
        Err(err) => {
            return Err(err);
        }
    };

    let conn = match Connection::open(&db_path) {
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
    let mut ftd = false;

    let json_data_string = String::from_utf8_lossy(DATA_JSON).to_string();
    let json_data = match serde_json::from_str::<DB>(&json_data_string) {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::JsonError(String::from(err.to_string())));
        }
    };

    let files = json_data.files;
    for file in files.iter() {
        let mut cont = false;
        for i in file.dependencies.iter() {
            if !*args::ARGS_FORCE_SETUP {
                match i.as_str() {
                    "web" => {
                        if !*args::ARGS_WEB {
                            cont = true;
                        }
                    }
                    "gui" => {
                        if !*args::ARGS_GUI {
                            cont = true;
                        }
                    }
                    "server" => {
                        if !*args::ARGS_SERVER {
                            cont = true;
                        }
                    }
                    _ => (),
                }
            }
        }

        if cont {
            continue;
        }

        let typ = file.r#type.clone();

        if typ == "yt-dlp" {
            let name = &file.name.clone();
            let db_name = name.replace(".", "_").replace(" ", "_").to_uppercase();

            let db_item = match read_resource(&conn, &db_name) {
                Ok(value) => value,
                Err(err) => {
                    return Err(err);
                }
            };
            if db_item.is_none() {
                if !ftd {
                    println!("First time setup");
                }
                if !yt_dlp {
                    ftd = true;
                    match download_yt_dlp(&full_path).await {
                        Ok(_) => (),
                        Err(err) => {
                            return Err(err);
                        }
                    }
                    yt_dlp = true;
                }
                let url = &file.url.clone();
                let dmca = &file.dmca.clone();

                println!("{}", dmca);

                for _ in 0..2 {
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
                                eprintln!("\nProcess exited with status: {}\n", status);
                                continue;
                            }
                            break;
                        }
                        Err(_err) => {
                            eprintln!("\nFailed to spawn process\n");
                            continue;
                        }
                    }
                }

                let file_bytes = match read_file_to_bytes(&name) {
                    Ok(value) => value,
                    Err(_err) => {
                        continue;
                    }
                };

                let initial_data_1: &[u8] = &file_bytes;
                match write_resource(&conn, &db_name, initial_data_1, true) {
                    Ok(_id) => (),
                    Err(err) => {
                        return Err(err);
                    }
                }
                println!("Added {} to database\n", db_name);
                match std::fs::remove_file(name) {
                    Ok(_) => (),
                    Err(err) => {
                        return Err(MdownError::IoError(err, String::from(name)));
                    }
                };
            }
        }
    }

    if yt_dlp {
        if std::fs::metadata(&full_path).is_ok() {
            match std::fs::remove_file(&full_path) {
                Ok(_) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, full_path));
                }
            };
        }
    }

    if *args::ARGS_FORCE_SETUP {
        if !ftd {
            println!("All requirements have been already installed");
        }
        std::process::exit(0);
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

    print!("Fetching {}\r", url);
    match std::io::stdout().flush() {
        Ok(()) => (),
        Err(err) => {
            return Err(MdownError::IoError(err, String::new()));
        }
    }
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
            return Err(MdownError::IoError(err, full_path.to_string()));
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
                resolute::SUSPENDED.lock().push(MdownError::IoError(err, full_path.to_string()));
            }
        }
        downloaded += chunk.len() as u64;
        let current_time = std::time::Instant::now();
        if current_time.duration_since(last_check_time) >= interval {
            let percentage = ((100.0 / (total_size as f32)) * (downloaded as f32)).round() as i64;
            let perc_string = download::get_perc(percentage);
            let message = format!(
                "Downloading yt-dlp_min.exe {}% - {:.2}mb of {:.2}mb [{:.2}mb/s]\r",
                perc_string,
                (downloaded as f32) / 1024.0 / 1024.0,
                final_size,
                (((downloaded as f32) - last_size) * 10.0) / 1024.0 / 1024.0
            );
            print!("{}", message);
            match std::io::stdout().flush() {
                Ok(()) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, String::new()));
                }
            }
            last_check_time = current_time;
            last_size = downloaded as f32;
        }
    }
    let message = format!(
        "Downloading yt-dlp_min.exe {}% - {:.2}mb of {:.2}mb",
        100,
        (downloaded as f32) / 1024.0 / 1024.0,
        (total_size as f32) / 1024.0 / 1024.0
    );
    println!("{}\n", message);
    Ok(())
}

pub(crate) fn setup_settings() -> Result<String, MdownError> {
    let db_path = match getter::get_db_path() {
        Ok(path) => path,
        Err(err) => {
            return Err(err);
        }
    };

    let conn = match Connection::open(&db_path) {
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
    match args::ARGS.lock().subcommands.clone() {
        Some(args::Commands::Settings { folder }) => {
            match folder {
                Some(Some(folder)) => {
                    match write_resource(&conn, "folder", folder.as_bytes(), false) {
                        Ok(_id) => (),
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }
                Some(None) =>
                    match delete_resource(&conn, "folder") {
                        Ok(_id) => (),
                        Err(err) => {
                            return Err(err);
                        }
                    }
                None => (),
            }
        }
        Some(_) => (),
        None => (),
    }

    let folder = match read_resource(&conn, "folder") {
        Ok(Some(value)) =>
            String::from_utf8(value)
                .map_err(|e| MdownError::CustomError(e.to_string(), String::from("Base64Error")))
                .unwrap(),
        Ok(None) => String::from("."),
        Err(err) => {
            return Err(err);
        }
    };
    Ok(folder)
}
