use rusqlite::{ Connection, OptionalExtension, params };
use std::{ io::{ Read, Write }, process::Command, result::Result };

use crate::{
    args,
    download,
    debug,
    error::{ MdownError, suspend_error },
    getter,
    metadata,
    tutorial::TUTORIAL,
};

include!(concat!(env!("OUT_DIR"), "/data_json.rs"));

pub const DB_FOLDER: &str = "2001";
pub const DB_STAT: &str = "2002";
pub const DB_TUTORIAL: &str = "2003";
pub const DB_BACKUP: &str = "2004";
#[cfg(feature = "music")]
pub const DB_MUSIC: &str = "2101";
pub const DB_UPDATE_TIME: &str = "2201";

/// Updates the database with a new update timestamp.
///
/// This function retrieves the database path, opens a connection, and writes the provided
/// timestamp (`time_str`) to the database under the `DB_UPDATE_TIME` resource.
///
/// # Parameters
/// - `time_str`: A string representing the update timestamp.
///
/// # Returns
/// - `Ok(())` if the update time is successfully written to the database.
/// - `Err(MdownError)`: An error if the database path retrieval, connection, or write operation fails.
///
/// # Error Codes
/// - `10641`: Failed to retrieve the database path.
/// - `10630`: Failed to open a database connection.
/// - `10678`: Failed to write the update time to the database.
///
/// # Example
/// ```
/// if let Err(e) = set_update_time("2025-02-04T12:00:00Z") {
///     eprintln!("Failed to update time: {:?}", e);
/// }
/// ```
pub(crate) fn set_update_time(time_str: &str) -> Result<(), MdownError> {
    let db_path = match getter::get_db_path() {
        Ok(path) => path,
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 10641));
        }
    };

    // Open a connection to the database
    let conn = match Connection::open(&db_path) {
        Ok(conn) => conn,
        Err(err) => {
            return Err(MdownError::DatabaseError(err, 10630));
        }
    };
    return match write_resource(&conn, DB_UPDATE_TIME, time_str.as_bytes(), false) {
        Ok(_id) => Ok(()),
        Err(err) => Err(MdownError::ChainedError(Box::new(err), 10678)),
    };
}

/// Retrieves the update time from the database.
///
/// This function attempts to fetch the update time from the database by reading the
/// resource identified by `DB_UPDATE_TIME`. If successful, it returns the update time
/// as a `String`. If no value is found, it returns `None`. If any errors occur during
/// the process, they are wrapped in the appropriate `MdownError` variants for handling.
///
/// # Errors
/// - `MdownError::ChainedError(10642)`: If `getter::get_db_path` returns an error.
/// - `MdownError::DatabaseError(10629)`: If there is an issue opening the database connection.
/// - `MdownError::CustomError(10628)`: If there is a failure converting the byte value to a `String` from UTF-8.
/// - `MdownError::ChainedError(10679)`: If there is an error during UTF-8 conversion of the update time.
/// - `MdownError::ChainedError(10680)`: If there is an error reading the resource from the database.
///
/// # Returns
/// - `Ok(Some(String))`: If the update time is successfully retrieved from the database.
/// - `Ok(None)`: If no update time is found in the database.
/// - `Err(MdownError)`: In case of any errors during the process.
///
/// # Example
/// ```
/// match get_update_time() {
///     Ok(Some(update_time)) => {
///         println!("Update time: {}", update_time);
///     }
///     Ok(None) => {
///         println!("No update time found.");
///     }
///     Err(e) => {
///         eprintln!("Error occurred: {:?}", e);
///     }
/// }
/// ```
pub(crate) fn get_update_time() -> Result<Option<String>, MdownError> {
    let db_path = match getter::get_db_path() {
        Ok(path) => path,
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 10642));
        }
    };

    // Open a connection to the database
    let conn = match Connection::open(&db_path) {
        Ok(conn) => conn,
        Err(err) => {
            return Err(MdownError::DatabaseError(err, 10629));
        }
    };
    return match read_resource(&conn, DB_UPDATE_TIME) {
        Ok(Some(value)) =>
            match
                String::from_utf8(value).map_err(|e|
                    MdownError::CustomError(e.to_string(), String::from("Base64Error"), 10628)
                )
            {
                Ok(update_time) => {
                    debug!("update_time from database: {:?}", update_time);
                    Ok(Some(update_time))
                }
                Err(err) => Err(MdownError::ChainedError(Box::new(err), 10679)),
            }
        Ok(None) => Ok(None),
        Err(err) => Err(MdownError::ChainedError(Box::new(err), 10680)),
    };
}

/// Retrieves a resource from the database by name.
///
/// This function attempts to fetch a resource from the database by reading the resource
/// identified by `name`. If successful, it returns the resource as a `String`. If no value
/// is found, it returns `None`. If any errors occur during the process, they are wrapped
/// in the appropriate `MdownError` variants for handling.
///
/// # Arguments
/// - `name`: The name of the resource to retrieve from the database.
///
/// # Errors
/// - `MdownError::ChainedError(100643)`: If `getter::get_db_path` returns an error.
/// - `MdownError::DatabaseError(10638)`: If there is an issue opening the database connection.
/// - `MdownError::CustomError(10639)`: If there is a failure converting the byte value to a `String` from UTF-8.
/// - `MdownError::ChainedError(10681)`: If there is an error during UTF-8 conversion of the resource.
/// - `MdownError::ChainedError(10682)`: If there is an error reading the resource from the database.
///
/// # Returns
/// - `Ok(Some(String))`: If the resource is successfully retrieved from the database.
/// - `Ok(None)`: If no resource is found in the database.
/// - `Err(MdownError)`: In case of any errors during the process.
///
/// # Example
/// ```
/// match read_resource_lone("example_resource") {
///     Ok(Some(resource)) => {
///         println!("Resource: {}", resource);
///     }
///     Ok(None) => {
///         println!("Resource not found.");
///     }
///     Err(e) => {
///         eprintln!("Error occurred: {:?}", e);
///     }
/// }
/// ```
pub(crate) fn read_resource_lone(name: &str) -> Result<Option<String>, MdownError> {
    let db_path = match getter::get_db_path() {
        Ok(path) => path,
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 100643));
        }
    };

    let conn = match Connection::open(&db_path) {
        Ok(conn) => conn,
        Err(err) => {
            return Err(MdownError::DatabaseError(err, 10638));
        }
    };
    return match read_resource(&conn, name) {
        Ok(Some(value)) =>
            match
                String::from_utf8(value).map_err(|e|
                    MdownError::CustomError(e.to_string(), String::from("Base64Error"), 10639)
                )
            {
                Ok(resource) => {
                    debug!("{} from database: {:?}", name, resource);
                    Ok(Some(resource))
                }
                Err(err) => Err(MdownError::ChainedError(Box::new(err), 10681)),
            }
        Ok(None) => Ok(None),
        Err(err) => Err(MdownError::ChainedError(Box::new(err), 10682)),
    };
}

/// Writes a resource to the database.
///
/// This function attempts to write a resource to the database with the given `name` and `data`.
/// The `is_binary` flag indicates whether the data should be treated as binary. If the write
/// operation is successful, it returns the number of bytes written. If any errors occur during
/// the process, they are wrapped in the appropriate `MdownError` variants for handling.
///
/// # Arguments
/// - `name`: The name of the resource to be written to the database.
/// - `data`: A byte slice representing the data to be written.
/// - `is_binary`: A flag indicating whether the data is binary (`true`) or not (`false`).
///
/// # Errors
/// - `MdownError::ChainedError(10643)`: If `getter::get_db_path` returns an error.
/// - `MdownError::DatabaseError(10640)`: If there is an issue opening the database connection.
/// - `MdownError::ChainedError(10683)`: If there is an error during the write operation.
///
/// # Returns
/// - `Ok(u64)`: The number with the ID of the inserted or updated resource on success.
/// - `Err(MdownError)`: In case of any errors during the process.
///
/// # Example
/// ```
/// match write_resource_lone("example_resource", &data, true) {
///     Ok(bytes_written) => {
///         println!("Successfully wrote {} bytes.", bytes_written);
///     }
///     Err(e) => {
///         eprintln!("Error occurred: {:?}", e);
///     }
/// }
/// ```
pub(crate) fn write_resource_lone(
    name: &str,
    data: &[u8],
    is_binary: bool
) -> Result<u64, MdownError> {
    let db_path = match getter::get_db_path() {
        Ok(path) => path,
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 10643));
        }
    };

    let conn = match Connection::open(&db_path) {
        Ok(conn) => conn,
        Err(err) => {
            return Err(MdownError::DatabaseError(err, 10640));
        }
    };
    return match write_resource(&conn, name, data, is_binary) {
        Ok(value) => Ok(value),
        Err(err) => Err(MdownError::ChainedError(Box::new(err), 10683)),
    };
}

/// Initializes the database by creating the `resources` table if it does not already exist.
///
/// This function executes a SQL statement to create the `resources` table within the provided database connection.
/// The table includes the following fields:
/// - `id`: An integer that serves as the primary key.
/// - `name`: A unique text field that cannot be null.
/// - `data`: A text field that cannot be null, intended to store the resource's data.
/// - `is_binary`: A boolean field indicating whether the resource data is binary.
///
/// # Arguments
/// * `conn` - A reference to a `Connection` object representing the database connection.
///
/// # Returns
/// * `Result<(), MdownError>` - Returns `Ok(())` if the table is created successfully or already exists,
///   or an `MdownError` on failure.
///
/// # Errors
/// * Returns `MdownError::DatabaseError` if there is an issue executing the SQL statement.
///
/// # Panics
/// * This function does not explicitly panic.
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
            return Err(MdownError::DatabaseError(err, 10600));
        }
    }
    Ok(())
}
/// Reads a resource from the database by its name.
///
/// This function retrieves the `data` and `is_binary` fields from the `resources` table for a given resource name.
/// If the resource is found, the data is returned as a `Vec<u8>`. If the data is stored as binary (indicated by the `is_binary` flag),
/// it is decoded from a base64 string. Otherwise, it is returned as raw bytes.
///
/// # Arguments
/// * `conn` - A reference to a `Connection` object representing the database connection.
/// * `name` - A string slice that holds the name of the resource to be retrieved.
///
/// # Returns
/// * `Result<Option<Vec<u8>>, MdownError>` - Returns `Ok(Some(Vec<u8>))` if the resource is found,
///   `Ok(None)` if the resource does not exist, or an `MdownError` on failure.
///
/// # Errors
/// * Returns `MdownError::DatabaseError` if there is an issue with the SQL query.
/// * Returns `MdownError::CustomError` with a `Base64Error` if there is an issue decoding the base64-encoded data.
///
/// # Panics
/// * This function does not explicitly panic.
///
/// # Deprecated
/// * The `base64::decode` function used in this code is marked as deprecated in some contexts, but it is still used here.
pub(crate) fn read_resource(conn: &Connection, name: &str) -> Result<Option<Vec<u8>>, MdownError> {
    // Prepare the SQL statement to select the data and is_binary fields from the resources table
    let mut stmt = match conn.prepare("SELECT data, is_binary FROM resources WHERE name = ?1") {
        Ok(stmt) => stmt,
        Err(err) => {
            // Return a DatabaseError if preparing the statement fails
            return Err(MdownError::DatabaseError(err, 10601));
        }
    };

    // Execute the query and process the result
    match
        stmt
            .query_row(params![name], |row| {
                // Extract the data and is_binary fields from the row
                let data: String = match row.get(0) {
                    Ok(value) => value,
                    Err(err) => {
                        // Return the error if fetching the data field fails
                        return Err(err);
                    }
                };
                let is_binary: bool = match row.get(1) {
                    Ok(value) => value,
                    Err(err) => {
                        // Return the error if fetching the is_binary field fails
                        return Err(err);
                    }
                };

                // Decode the data based on whether it is binary
                if is_binary {
                    #[allow(deprecated)]
                    let decoded_data = match
                        base64::decode(&data).map_err(|e| {
                            // Wrap base64 decoding errors in a CustomError
                            MdownError::CustomError(
                                e.to_string(),
                                String::from("Base64Error"),
                                10602
                            )
                        })
                    {
                        Ok(value) => value,
                        Err(_err) => {
                            // Return an InvalidQuery error if base64 decoding fails
                            return Err(rusqlite::Error::InvalidQuery);
                        }
                    };
                    Ok(Some(decoded_data))
                } else {
                    // Return the data as raw bytes if it is not binary
                    Ok(Some(data.into_bytes()))
                }
            })
            .optional()
    {
        Ok(result) => Ok(result.unwrap_or_default()),
        Err(err) => Err(MdownError::DatabaseError(err, 10603)),
    }
}

/// Writes a resource to the database, either inserting a new entry or updating an existing one.
///
/// This function adds a new resource to the `resources` table or updates an existing one if a resource with the same name already exists.
/// The resource data is converted to a string format based on whether it is binary or not. If `is_binary` is true, the data is base64 encoded.
/// Otherwise, it is converted to a UTF-8 string.
///
/// # Arguments
/// * `conn` - A reference to a `Connection` object representing the database connection.
/// * `name` - A string slice that holds the name of the resource to be written or updated.
/// * `data` - A slice of bytes representing the resource data.
/// * `is_binary` - A boolean indicating whether the data is binary (true) or text (false).
///
/// # Returns
/// * `Result<u64, MdownError>` - Returns `Ok(u64)` with the ID of the inserted or updated resource on success,
///   or an `MdownError` on failure.
///
/// # Errors
/// * Returns `MdownError::CustomError` with a `Base64Error` if converting the data to a string fails while `is_binary` is false.
/// * Returns `MdownError::DatabaseError` if there is an issue executing the SQL statement.
///
/// # Panics
/// * This function does not explicitly panic.
///
/// # Deprecated
/// * The `base64::encode` function used in this code is marked as deprecated in some contexts, but it is still used here.
fn write_resource(
    conn: &Connection,
    name: &str,
    data: &[u8],
    is_binary: bool
) -> Result<u64, MdownError> {
    // Convert data to a string representation based on whether it is binary or not
    let data_str = if is_binary {
        #[allow(deprecated)]
        base64::encode(data)
    } else {
        match
            String::from_utf8(data.to_vec()).map_err(|e| {
                // Wrap UTF-8 conversion errors in a CustomError
                MdownError::CustomError(e.to_string(), String::from("Base64Error"), 10604)
            })
        {
            Ok(value) => value,
            Err(err) => {
                // Return the error if UTF-8 conversion fails
                return Err(MdownError::ChainedError(Box::new(err), 10644));
            }
        }
    };

    // Execute the SQL statement to insert or update the resource
    match
        conn.execute(
            "INSERT INTO resources (name, data, is_binary) VALUES (?1, ?2, ?3)
            ON CONFLICT(name) DO UPDATE SET data = excluded.data, is_binary = excluded.is_binary",
            params![name, data_str, is_binary]
        )
    {
        Ok(_) => {
            // Return the ID of the inserted or updated resource
            let id = conn.last_insert_rowid() as u64;
            Ok(id)
        }
        Err(err) => {
            // Return a DatabaseError if executing the statement fails
            Err(MdownError::DatabaseError(err, 10605))
        }
    }
}

/// Deletes a resource from the database by its name.
///
/// This function removes a resource entry from the `resources` table based on the provided name.
/// If the resource exists, it will be deleted from the table.
///
/// # Arguments
/// * `conn` - A reference to a `Connection` object representing the database connection.
/// * `name` - A string slice that holds the name of the resource to be deleted.
///
/// # Returns
/// * `Result<(), MdownError>` - Returns `Ok(())` if the deletion is successful,
///   or an `MdownError` on failure.
///
/// # Errors
/// * Returns `MdownError::DatabaseError` if there is an issue executing the SQL statement.
///
/// # Panics
/// * This function does not explicitly panic.
fn delete_resource(conn: &Connection, name: &str) -> Result<(), MdownError> {
    // Execute the SQL statement to delete the resource with the given name
    match conn.execute("DELETE FROM resources WHERE name = ?1", params![name]) {
        Ok(_) => Ok(()),
        Err(err) => Err(MdownError::DatabaseError(err, 10606)),
    }
}

/// Initializes the setup process for the application, including database setup and file management.
///
/// This asynchronous function performs several tasks to prepare the application:
/// 1. Initializes the database by calling `initialize_db`.
/// 2. Reads configuration data from a JSON source to determine which files need to be managed.
/// 3. Downloads the `yt-dlp` executable if necessary and uses it to process files based on the configuration.
/// 4. Adds the processed files to the database and cleans up any temporary files.
///
/// # Returns
/// * `Result<(), MdownError>` - Returns `Ok(())` on successful completion of the setup process, or an `MdownError` on failure.
///
/// # Errors
/// * Returns `MdownError::DatabaseError` if there are issues with database operations.
/// * Returns `MdownError::JsonError` if parsing the JSON configuration fails.
/// * Returns `MdownError::CustomError` if there are issues with base64 encoding/decoding or other custom errors.
/// * Returns `MdownError::IoError` if there are issues with file operations, such as removing files.
///
/// # Panics
/// * This function does not explicitly panic.
///
/// # Workflow
/// 1. **Database Initialization:** Opens a connection to the database and sets up the required tables if they do not already exist.
/// 2. **Configuration Handling:** Reads and parses a JSON configuration to determine which resources need to be handled.
/// 3. **Resource Handling:**
///    - For each file specified in the configuration, checks if it is needed based on provided flags.
///    - Downloads and processes files if they are not already present in the database.
///    - Updates the database with the newly processed files.
/// 4. **Cleanup:** Removes temporary files and the `yt-dlp` executable if they are no longer needed.
/// 5. **Exit Conditions:** Exits the process if `ARGS_FORCE_SETUP` is true, indicating all requirements are installed.
pub(crate) async fn init() -> Result<(), MdownError> {
    debug!("initializing database");

    // Get the path to the database
    let db_path = match getter::get_db_path() {
        Ok(path) => path,
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 10645));
        }
    };

    // Open a connection to the database
    let conn = match Connection::open(&db_path) {
        Ok(conn) => conn,
        Err(err) => {
            return Err(MdownError::DatabaseError(err, 10607));
        }
    };

    // Initialize the database schema
    match initialize_db(&conn) {
        Ok(_) => (),
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 10677));
        }
    }

    debug!("db initialized");
    let full_path = String::from("yt-dlp_min.exe");

    let mut yt_dlp = false;
    let mut ftd = false;

    // Parse JSON configuration data
    let json_data_string = String::from_utf8_lossy(DATA_JSON).to_string();
    let json_data = match serde_json::from_str::<metadata::DB>(&json_data_string) {
        Ok(value) => value,
        Err(err) => {
            return Err(MdownError::JsonError(err.to_string(), 10608));
        }
    };

    let files = json_data.files;
    for file in files.iter() {
        let mut cont = false;
        let name = &file.name.clone();
        let db_name = &file.db_name.clone();
        for i in file.dependencies.iter() {
            // Check dependencies based on flags
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

        // Skip download if not needed
        if cont {
            debug!("Skipping download of {} because it is not needed", name);
            continue;
        }

        let typ = file.r#type.clone();

        // Process 'yt-dlp' type files
        if typ == "yt-dlp" {
            debug!("yt-dlp");

            // Check if the file is already in the database
            let db_item = match read_resource(&conn, db_name) {
                Ok(value) => value,
                Err(err) => {
                    return Err(MdownError::ChainedError(Box::new(err), 10646));
                }
            };
            if db_item.is_none() {
                debug!("File {} is NOT in database", name);
                if !ftd {
                    println!("First time setup");
                }
                if !yt_dlp {
                    ftd = true;
                    // Download yt-dlp executable if needed
                    match download_yt_dlp(&full_path).await {
                        Ok(_) => (),
                        Err(err) => {
                            return Err(MdownError::ChainedError(Box::new(err), 10647));
                        }
                    }
                    yt_dlp = true;
                }
                let url = &file.url.clone();
                let dmca = &file.dmca.clone();

                println!("{}", dmca);

                // Execute yt-dlp to process the file
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
                            if let Some(stdout) = child.stdout.take() {
                                print_output(stdout, "stdout".to_string());
                            } else {
                                eprintln!("\nFailed to capture stdout\n");
                            }

                            if let Some(stderr) = child.stderr.take() {
                                print_output(stderr, "stderr".to_string());
                            } else {
                                eprintln!("\nFailed to capture stderr\n");
                            }

                            let status = match child.wait() {
                                Ok(status) => status,
                                Err(_err) => {
                                    eprintln!("\nFailed to wait for process\n");
                                    continue;
                                }
                            };

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

                // Read the processed file and update the database
                let file_bytes = match read_file_to_bytes(name) {
                    Ok(value) => value,
                    Err(_err) => {
                        continue;
                    }
                };

                let initial_data_1: &[u8] = &file_bytes;
                match write_resource(&conn, db_name, initial_data_1, true) {
                    Ok(_id) => (),
                    Err(err) => {
                        return Err(MdownError::ChainedError(Box::new(err), 10648));
                    }
                }
                println!("Added {} to database\n", db_name);
                match std::fs::remove_file(name) {
                    Ok(_) => (),
                    Err(err) => {
                        return Err(MdownError::IoError(err, String::from(name), 10609));
                    }
                };
            } else {
                debug!("File {} is in database", name);
            }
        }
    }

    // Remove yt-dlp executable if it was downloaded
    if yt_dlp && std::fs::metadata(&full_path).is_ok() {
        match std::fs::remove_file(&full_path) {
            Ok(_) => (),
            Err(err) => {
                return Err(MdownError::IoError(err, full_path, 10610));
            }
        };
    }

    // Exit if force setup is enabled
    if *args::ARGS_FORCE_SETUP {
        if !ftd {
            println!("All requirements have been already installed");
        }
        std::process::exit(0);
    }

    debug!("database configuration complete\n");

    Ok(())
}

/// Prints the output from a `Read` source to the console with a specified label.
///
/// This function spawns a new thread to read from the provided `Read` source and print the output to the console.
/// The output is labeled with the provided `label`. The function handles carriage returns (`\r`) by overwriting the
/// previous line with the label and continues printing subsequent bytes as characters.
///
/// # Type Parameters
/// * `R` - A type that implements the `Read` trait, which allows reading bytes from the source.
///
/// # Arguments
/// * `reader` - An instance of a type implementing `Read` from which the output will be read and printed.
/// * `label` - A string to be used as a label for the output, indicating the source of the data.
///
/// # Behavior
/// * Reads data from the `reader` in chunks of up to 1024 bytes.
/// * Prints the data to the console, handling carriage returns to overwrite the previous line with the label.
/// * Flushes the console output to ensure that all data is printed immediately.
///
/// # Errors
/// * Prints an error message to standard error if there is an issue flushing the console or reading from the `reader`.
///
/// # Panics
/// * This function does not explicitly panic.
fn print_output<R>(reader: R, label: String) where R: Read + Send + 'static {
    let mut reader = std::io::BufReader::new(reader);
    let mut buffer = [0; 1024];
    std::thread::spawn(move || {
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    // End of input
                    break;
                }
                Ok(n) => {
                    // Read `n` bytes from the buffer
                    let output = &buffer[..n];
                    for &byte in output {
                        if byte == b'\r' {
                            // Handle carriage return by printing the label
                            print!("\r{}: ", label);
                        } else {
                            // Print byte as a character
                            print!("{}", byte as char);
                        }
                    }
                    use std::io::Write;
                    match std::io::stdout().flush() {
                        Ok(_) => (),
                        Err(err) => {
                            // Print an error message if flushing stdout fails
                            eprintln!("Error flushing stdout: {}", err);
                            break;
                        }
                    };
                }
                Err(e) => {
                    // Print an error message if reading from the reader fails
                    eprintln!("Error reading {}: {}", label, e);
                    break;
                }
            }
        }
    });
}

/// Reads the contents of a file into a vector of bytes.
///
/// This function opens a file specified by `file_path`, reads its contents, and stores them in a `Vec<u8>`.
/// If the file cannot be opened or read, it returns an `std::io::Error`.
///
/// # Arguments
/// * `file_path` - A string slice that holds the path to the file to be read.
///
/// # Returns
/// * `std::io::Result<Vec<u8>>` - Returns `Ok(Vec<u8>)` with the file contents as bytes on success,
///   or an `std::io::Error` on failure.
///
/// # Errors
/// * Returns an `std::io::Error` if the file cannot be opened or read, for example, if the file does not exist
///   or if there are I/O errors during reading.
///
/// # Panics
/// * This function does not explicitly panic.
fn read_file_to_bytes(file_path: &str) -> std::io::Result<Vec<u8>> {
    // Open the file at the specified path
    let mut file = match std::fs::File::open(file_path) {
        Ok(file) => file,
        Err(err) => {
            // Return the error if the file cannot be opened
            return Err(err);
        }
    };
    let mut buffer = Vec::new();
    // Read the entire file into the buffer
    match file.read_to_end(&mut buffer) {
        Ok(_) => (),
        Err(err) => {
            // Return the error if reading the file fails
            return Err(err);
        }
    }
    // Return the file contents as a vector of bytes
    Ok(buffer)
}

/// Downloads the `yt-dlp_min.exe` file from a specified URL and saves it to the provided path.
///
/// This asynchronous function performs an HTTP GET request to download the `yt-dlp_min.exe` file.
/// It displays the download progress in the console, handles errors related to network requests,
/// and manages file writing operations. The function periodically updates the progress of the download
/// and provides feedback on the console.
///
/// # Arguments
/// * `full_path` - A string slice that holds the path where the downloaded file will be saved.
///
/// # Returns
/// * `Result<(), MdownError>` - Returns `Ok(())` on success or an `MdownError` on failure.
///
/// # Errors
/// * Returns `MdownError::NetworkError` if there is an issue with the network request or reading chunks from the response.
/// * Returns `MdownError::IoError` if there is an issue with file operations, such as creating or writing to the file.
///
/// # Panics
/// * This function does not explicitly panic.
///
/// # Example
/// ```no_run
/// #[tokio::main]
/// async fn main() -> Result<(), MdownError> {
///     download_yt_dlp("path/to/save/yt-dlp_min.exe").await
/// }
/// ```
async fn download_yt_dlp(full_path: &str) -> Result<(), MdownError> {
    // Initialize the HTTP client
    let client = match download::get_client() {
        Ok(client) => client,
        Err(err) => {
            return Err(MdownError::NetworkError(err, 10611));
        }
    };
    let url = match get_ytdlp().await {
        Ok(url) => url,
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 10649));
        }
    };

    // Print a message indicating that the download is starting
    print!("Fetching {}\r", url);
    match std::io::stdout().flush() {
        Ok(()) => (),
        Err(err) => {
            return Err(MdownError::IoError(err, String::new(), 10612));
        }
    }

    // Send an HTTP GET request to download the file
    let mut response = match client.get(&url).send().await {
        Ok(response) => response,
        Err(err) => {
            return Err(MdownError::NetworkError(err, 10613));
        }
    };
    println!("Fetching {} DONE", url);

    // Get the total size and final size of the file from the response
    let (total_size, final_size_string) = download::get_size(&response);

    // Create the file where the downloaded data will be saved
    let mut file = match std::fs::File::create(full_path) {
        Ok(file) => file,
        Err(err) => {
            return Err(MdownError::IoError(err, full_path.to_string(), 10614));
        }
    };
    let (mut downloaded, mut last_size) = (0, 0);
    let interval = std::time::Duration::from_millis(100);
    let mut last_check_time = std::time::Instant::now();

    while
        //prettier-ignore
        // Read chunks of data from the response and write them to the file
        let Some(chunk) = match response.chunk().await {
            Ok(Some(chunk)) => Some(chunk),
            Ok(None) => None,
            Err(err) => {
                return Err(MdownError::NetworkError(err, 10615));
            }
        }
    {
        // Write the chunk to the file
        match file.write_all(&chunk) {
            Ok(()) => (),
            Err(err) => {
                return Err(MdownError::IoError(err, full_path.to_string(), 10616));
            }
        }
        downloaded += chunk.len() as u64;
        let current_time = std::time::Instant::now();

        // Update the progress display periodically
        if current_time.duration_since(last_check_time) >= interval {
            let percentage = (100.0 / (total_size as f32)) * (downloaded as f32);
            let perc_string = download::get_perc(percentage);
            let current_mb = bytefmt::format(downloaded);
            let current_mbs = bytefmt::format(downloaded - last_size);
            let message = format!(
                "Downloading yt-dlp_min.exe {}% - {} of {} [{}/s]\r",
                perc_string,
                current_mb,
                final_size_string,
                current_mbs
            );
            print!("{}", message);
            match std::io::stdout().flush() {
                Ok(()) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, String::new(), 10617));
                }
            }
            last_check_time = current_time;
            last_size = downloaded;
        }
    }

    let current_mb = bytefmt::format(downloaded);
    let max_mb = bytefmt::format(total_size);

    // Print the final download progress
    let message = format!("Downloading yt-dlp_min.exe {}% - {} of {}", 100, current_mb, max_mb);
    println!("{}\n", message);
    Ok(())
}

/// Retrieves the download URL for the latest `yt-dlp.exe` release from GitHub.
///
/// This function sends an HTTP GET request to the GitHub API to fetch the latest release details
/// for `yt-dlp`. It checks if the release contains the `yt-dlp.exe` asset and returns its download
/// URL if found. If any errors occur during the request, parsing, or asset lookup, they are returned
/// as the appropriate `MdownError` variants.
///
/// # Errors
/// - `MdownError::NetworkError(10631)`: If there is an error while creating the HTTP client.
/// - `MdownError::NetworkError(10632)`: If there is an error while sending the HTTP request.
/// - `MdownError::CustomError(10633)`: If the response from the server is not successful.
/// - `MdownError::JsonError(10634)`: If there is an error parsing the JSON response.
/// - `MdownError::NotFoundError(10637)`: If the `assets` array is not found in the JSON response.
/// - `MdownError::NotFoundError(10635)`: If the download URL for `yt-dlp.exe` is not found in the assets.
/// - `MdownError::NotFoundError(10636)`: If `yt-dlp.exe` is not found in the release assets.
///
/// # Returns
/// - `Ok(String)`: The download URL for `yt-dlp.exe` if found.
/// - `Err(MdownError)`: In case of any errors during the process.
///
/// # Example
/// ```
/// match get_ytdlp().await {
///     Ok(download_url) => {
///         println!("Download URL: {}", download_url);
///     }
///     Err(e) => {
///         eprintln!("Error occurred: {:?}", e);
///     }
/// }
/// ```
async fn get_ytdlp() -> Result<String, MdownError> {
    let url = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";
    let client = match download::get_client() {
        Ok(client) => client,
        Err(err) => {
            return Err(MdownError::NetworkError(err, 10631));
        }
    };

    let response = match
        client.get(url).header("User-Agent", "Rust-yt-dlp-downloader").send().await
    {
        Ok(response) => response,
        Err(err) => {
            return Err(MdownError::NetworkError(err, 10632));
        }
    };

    if !response.status().is_success() {
        return Err(
            MdownError::CustomError(
                String::from("ResponseError"),
                String::from("Response is not success"),
                10633
            )
        );
    }

    let json: serde_json::Value = match response.json().await {
        Ok(json) => json,
        Err(err) => {
            return Err(MdownError::JsonError(err.to_string(), 10634));
        }
    };

    let assets = match json["assets"].as_array() {
        Some(assets) => assets,
        None => {
            return Err(
                MdownError::NotFoundError(
                    String::from("No 'assets' array found in the JSON response"),
                    10637
                )
            );
        }
    };

    if
        let Some(asset) = assets
            .iter()
            .find(|asset| { asset["name"].as_str().map_or(false, |name| name == "yt-dlp.exe") })
    {
        if let Some(download_url) = asset["browser_download_url"].as_str() {
            return Ok(download_url.to_string());
        } else {
            return Err(
                MdownError::NotFoundError("Download URL for yt-dlp.exe not found".into(), 10635)
            );
        }
    } else {
        return Err(
            MdownError::NotFoundError("yt-dlp.exe not found in the release assets".into(), 10636)
        );
    }
}

/// Sets up settings by configuring database access and updating settings based on command-line arguments.
///
/// This function performs the following tasks:
/// 1. Retrieves the database path and opens a connection to it.
/// 2. Initializes the database schema if it hasn't been set up already.
/// 3. Updates the settings in the database based on command-line arguments (if provided).
/// 4. Reads the settings from the database and returns them.
///
/// # Returns
/// * `Result<metadata::Settings, MdownError>` - Returns `Ok(metadata::Settings)` with the retrieved settings on success,
///   or an `MdownError` on failure.
///
/// # Errors
/// * Returns `MdownError::DatabaseError` if there is an issue with the database connection or operations.
/// * Returns `MdownError::CustomError` if there is an issue with decoding data from the database.
///
/// # Panics
/// * This function does not explicitly panic.
///
/// # Example
/// ```no_run
/// fn main() -> Result<(), MdownError> {
///     let settings = setup_settings()?;
///     // Use settings here
///     Ok(())
/// }
/// ```
pub(crate) fn setup_settings() -> Result<(metadata::Settings, bool), MdownError> {
    debug!("setup_settings");

    // Retrieve the database path
    let db_path = match getter::get_db_path() {
        Ok(path) => path,
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 10650));
        }
    };

    // Open a connection to the database
    let conn = match Connection::open(&db_path) {
        Ok(conn) => conn,
        Err(err) => {
            return Err(MdownError::DatabaseError(err, 10618));
        }
    };

    // Initialize the database schema
    match initialize_db(&conn) {
        Ok(_) => (),
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 10651));
        }
    }

    let mut changed = false;

    // Update settings in the database based on command-line arguments
    match args::ARGS.lock().subcommands.clone() {
        Some(
            args::Commands::Settings {
                folder,
                stat,
                backup,
                #[cfg(feature = "music")]
                music,
                clear,
                #[cfg(not(feature = "music"))]
                ..
            },
        ) => {
            match folder {
                Some(Some(folder)) => {
                    match write_resource(&conn, DB_FOLDER, folder.as_bytes(), false) {
                        Ok(_id) => (),
                        Err(err) => {
                            return Err(MdownError::ChainedError(Box::new(err), 10652));
                        }
                    }
                }
                Some(None) => {
                    match delete_resource(&conn, DB_FOLDER) {
                        Ok(_id) => (),
                        Err(err) => {
                            return Err(MdownError::ChainedError(Box::new(err), 10653));
                        }
                    }
                }
                None => (),
            }
            match stat {
                Some(Some(stat)) => {
                    if stat == "0" || stat == "1" {
                        match write_resource(&conn, DB_STAT, stat.as_bytes(), false) {
                            Ok(_id) => (),
                            Err(err) => {
                                return Err(MdownError::ChainedError(Box::new(err), 10654));
                            }
                        }
                    } else {
                        suspend_error(
                            MdownError::CustomError(
                                String::from("stat should be 1 or 0"),
                                String::from("UserError"),
                                10619
                            )
                        );
                    }
                }
                Some(None) => {
                    match delete_resource(&conn, DB_STAT) {
                        Ok(_id) => (),
                        Err(err) => {
                            return Err(MdownError::ChainedError(Box::new(err), 10655));
                        }
                    }
                }
                None => (),
            }
            match backup {
                Some(Some(backup)) => {
                    match write_resource(&conn, DB_BACKUP, backup.as_bytes(), false) {
                        Ok(_id) => (),
                        Err(err) => {
                            return Err(MdownError::ChainedError(Box::new(err), 10656));
                        }
                    }
                }
                Some(None) => {
                    match delete_resource(&conn, DB_BACKUP) {
                        Ok(_id) => (),
                        Err(err) => {
                            return Err(MdownError::ChainedError(Box::new(err), 10657));
                        }
                    }
                }
                None => (),
            }
            #[cfg(feature = "music")]
            match music {
                Some(Some(music)) => {
                    match write_resource(&conn, DB_MUSIC, music.as_bytes(), false) {
                        Ok(_id) => (),
                        Err(err) => {
                            return Err(MdownError::ChainedError(Box::new(err), 10658));
                        }
                    }
                }
                Some(None) => {
                    match delete_resource(&conn, DB_MUSIC) {
                        Ok(_id) => (),
                        Err(err) => {
                            return Err(MdownError::ChainedError(Box::new(err), 10659));
                        }
                    }
                }
                None => (),
            }
            if clear {
                match delete_resource(&conn, DB_FOLDER) {
                    Ok(_id) => (),
                    Err(err) => {
                        return Err(MdownError::ChainedError(Box::new(err), 10660));
                    }
                }
                match delete_resource(&conn, DB_STAT) {
                    Ok(_id) => (),
                    Err(err) => {
                        return Err(MdownError::ChainedError(Box::new(err), 10661));
                    }
                }
                match delete_resource(&conn, DB_TUTORIAL) {
                    Ok(_id) => (),
                    Err(err) => {
                        return Err(MdownError::ChainedError(Box::new(err), 10662));
                    }
                }
                match delete_resource(&conn, DB_BACKUP) {
                    Ok(_id) => (),
                    Err(err) => {
                        return Err(MdownError::ChainedError(Box::new(err), 10663));
                    }
                }
                #[cfg(feature = "music")]
                match delete_resource(&conn, DB_MUSIC) {
                    Ok(_id) => (),
                    Err(err) => {
                        return Err(MdownError::ChainedError(Box::new(err), 10664));
                    }
                }
            }
            changed = true;
        }
        Some(_) => (),
        None => (),
    }

    // Read the folder setting from the database
    let folder = match read_resource(&conn, DB_FOLDER) {
        Ok(Some(value)) =>
            match
                String::from_utf8(value).map_err(|e|
                    MdownError::CustomError(e.to_string(), String::from("Base64Error"), 10620)
                )
            {
                Ok(folder) => {
                    debug!("folder from database: {:?}", folder);
                    folder
                }
                Err(err) => {
                    return Err(MdownError::ChainedError(Box::new(err), 10665));
                }
            }
        Ok(None) => args::ARGS.lock().folder.clone(),
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 10666));
        }
    };
    // Read the stat setting from the database
    let stat = match read_resource(&conn, DB_STAT) {
        Ok(Some(value)) =>
            match
                String::from_utf8(value).map_err(|e|
                    MdownError::CustomError(e.to_string(), String::from("Base64Error"), 10621)
                )
            {
                Ok(stat) => {
                    let stat = match stat.as_str() {
                        "1" => true,
                        "0" => false,
                        _ => {
                            suspend_error(
                                MdownError::CustomError(
                                    String::from("stat should be 1 or 0"),
                                    String::from("UserError"),
                                    10622
                                )
                            );
                            false
                        }
                    };
                    debug!("stat from database: {:?}", stat);
                    stat
                }
                Err(err) => {
                    return Err(MdownError::ChainedError(Box::new(err), 10667));
                }
            }
        Ok(None) => args::ARGS.lock().stat,
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 10668));
        }
    };

    // Read the backup setting from the database
    let backup = match read_resource(&conn, DB_BACKUP) {
        Ok(Some(value)) =>
            match
                String::from_utf8(value).map_err(|e|
                    MdownError::CustomError(e.to_string(), String::from("Base64Error"), 10623)
                )
            {
                Ok(backup) => {
                    let backup = match backup.as_str() {
                        "1" => true,
                        "0" => false,
                        _ => {
                            suspend_error(
                                MdownError::CustomError(
                                    String::from("backup should be 1 or 0"),
                                    String::from("UserError"),
                                    10624
                                )
                            );
                            false
                        }
                    };
                    debug!("backup from database: {:?}", backup);
                    backup
                }
                Err(err) => {
                    return Err(MdownError::ChainedError(Box::new(err), 10669));
                }
            }
        Ok(None) => true,
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 10670));
        }
    };

    #[cfg(feature = "music")]
    // Read the music setting from the database
    let music = match read_resource(&conn, DB_MUSIC) {
        Ok(Some(value)) =>
            match
                String::from_utf8(value).map_err(|e|
                    MdownError::CustomError(e.to_string(), String::from("Base64Error"), 10625)
                )
            {
                Ok(music) => {
                    debug!("music from database: {:?}", music);
                    Some(Some(music))
                }
                Err(err) => {
                    return Err(MdownError::ChainedError(Box::new(err), 10671));
                }
            }
        Ok(None) => args::ARGS.lock().music.clone(),
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 10672));
        }
    };

    // Create and return the settings object
    let settings = metadata::Settings { folder, stat, backup, #[cfg(feature = "music")] music };

    debug!("{:?}\n", settings);

    Ok((settings, changed))
}

/// Checks and updates the tutorial status based on database and argument values.
///
/// This function checks whether the tutorial should be shown by looking for the relevant entry
/// in the database. It reads the tutorial flag from the database and updates the global `TUTORIAL`
/// state accordingly. If no tutorial value is found, it attempts to initialize the tutorial flag
/// and store it in the database. The function also checks various argument flags (`ARGS_TUTORIAL`,
/// `ARGS_SKIP_TUTORIAL`) to force specific tutorial behavior.
///
/// # Errors
/// - `MdownError::ChainedError(10673)`: If there is an error retrieving the database path.
/// - `MdownError::DatabaseError(10626)`: If there is an error opening the database connection.
/// - `MdownError::CustomError(10627)`: If there is an error converting the byte value of the tutorial flag to a `String`.
/// - `MdownError::ChainedError(10674)`: If there is an error while handling the tutorial flag from the database.
/// - `MdownError::ChainedError(10675)`: If there is an error while attempting to write the tutorial flag to the database.
/// - `MdownError::ChainedError(10676)`: If there is an error while reading the tutorial flag from the database.
///
/// # Returns
/// - `Ok(())`: If the operation is successful.
/// - `Err(MdownError)`: In case of any errors during the process.
///
/// # Example
/// ```
/// match check_tutorial() {
///     Ok(()) => {
///         println!("Tutorial check completed successfully.");
///     }
///     Err(e) => {
///         eprintln!("Error occurred: {:?}", e);
///     }
/// }
/// ```
pub(crate) fn check_tutorial() -> Result<(), MdownError> {
    debug!("check_tutorial");

    // Retrieve the database path
    let db_path = match getter::get_db_path() {
        Ok(path) => path,
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 10673));
        }
    };

    // Open a connection to the database
    let conn = match Connection::open(&db_path) {
        Ok(conn) => conn,
        Err(err) => {
            return Err(MdownError::DatabaseError(err, 10626));
        }
    };

    match read_resource(&conn, DB_TUTORIAL) {
        Ok(Some(value)) =>
            match
                String::from_utf8(value).map_err(|e|
                    MdownError::CustomError(e.to_string(), String::from("Base64Error"), 10627)
                )
            {
                Ok(tutorial) => {
                    debug!("tutorial from database: {:?}", tutorial);
                    if tutorial == "1" {
                        *TUTORIAL.lock() = true;
                    }
                }
                Err(err) => {
                    return Err(MdownError::ChainedError(Box::new(err), 10674));
                }
            }
        Ok(None) => {
            if
                !*args::ARGS_WEB &&
                !*args::ARGS_GUI &&
                !*args::ARGS_CHECK &&
                !*args::ARGS_UPDATE &&
                !*args::ARGS_QUIET &&
                !*args::ARGS_RESET &&
                !args::ARGS_SHOW.is_some() &&
                !args::ARGS_SHOW_ALL.is_some() &&
                *args::ARGS_ENCODE == String::new() &&
                !*args::ARGS_DELETE &&
                !*args::ARGS_SHOW_LOG
            {
                *TUTORIAL.lock() = true;
                match write_resource(&conn, DB_TUTORIAL, b"0", false) {
                    Ok(_id) => (),
                    Err(err) => {
                        return Err(MdownError::ChainedError(Box::new(err), 10675));
                    }
                };
            }
        }
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 10676));
        }
    }

    if *args::ARGS_TUTORIAL {
        *TUTORIAL.lock() = true;
    } else if *args::ARGS_SKIP_TUTORIAL {
        *TUTORIAL.lock() = false;
    }
    Ok(())
}
