//!# Manga Downloader
//!
//!`manga_downloader` is a command-line tool for managing and downloading manga. It supports various functionalities such as downloading manga by title, managing a database of downloaded content, and customizing the download process with various options.
//!
//!## Features
//!
//!- **Manga Downloading**: Download manga based on URL, language, title, volume, and chapter.
//!- **Database Management**: Check and update downloaded manga, view current manga and chapters in the database, and show logs.
//!- **Customization**: Configure the download process with options like maximum consecutive downloads, start offsets, and folder management.
//!- **Logging**: Enable logging and save logs to a file.
//!- **Web Mode**: Enter web mode to interact with the application through a web browser.
//!- **Music**: Play music during the download process with various options.
//!- **Server Mode**: Start the application in server mode.
//!- **GUI Support**: Experimental GUI mode available for certain features.
//!- **Development Options**: Options for debugging and development.
//!
//!## Usage
//!
//!To use the `manga_downloader` crate, you can run it from the command line with various options and subcommands. Below are some common options:
//!
//!- `--url <URL>`: The URL of the manga to download.
//!- `--lang <LANG>`: The language of the manga.
//!- `--title <TITLE>`: The title of the manga.
//!- `--folder <FOLDER>`: The folder to store downloaded manga.
//!- `--volume <VOLUME>`: The volume number of the manga.
//!- `--chapter <CHAPTER>`: The chapter number of the manga.
//!- `--saver`: Enable the saver mode.
//!- `--stat`: Generate a statistics file.
//!- `--quiet`: Suppress output.
//!- `--max_consecutive <NUMBER>`: Maximum number of consecutive downloads of images.
//!- `--force`: Force download even if the file exists.
//!- `--offset <OFFSET>`: The start offset for chapters.
//!- `--database_offset <OFFSET>`: The start offset for the database.
//!- `--unsorted`: Do not sort the database.
//!- `--cwd <DIR>`: Change the current working directory.
//!- `--encode <URL>`: Print URL in a program-readable format.
//!- `--log`: Enable logging and write to `log.json`.
//!- `--search <TITLE>`: Search for manga by title.
//!- `--web`: Enter web mode and open a browser on port 8080.
//!- `--music <OPTION>`: Play music during downloading.
//!- `--server`: Start in server mode.
//!- `--gui`: Experimental GUI version.
//!- `--debug`: Enable debugging.
//!- `--debug_file`: Debug file-related operations.
//!- `--dev`: Enable development mode.
//!
//!## Subcommands
//!
//!- `database`: Commands related to database management.
//!  - `--check`: Check downloaded files for errors.
//!  - `--update`: Update downloaded files.
//!  - `--show [ID]`: Show current manga in the database or a specific manga by ID.
//!  - `--show_all [ID]`: Show current chapters in the database or a specific chapter by ID.
//!  - `--show_log`: Show current logs in the database.
//!
//!- `settings`: Commands related to application settings.
//!  - `--folder [NAME]`: Set or remove the default folder name.
//!
//!- `app`: Commands related to application management.
//!  - `--force_setup`: Force the first-time setup.
//!  - `--force_delete`: Force delete the `.lock` file.
//!  - `--delete`: Delete `dat.json`.
//!  - `--reset`: Delete all files created by the program.
//!
//!## Example
//!
//!To download a manga with specific options:
//!
//!```sh
//!manga_downloader --url "http://example.com/manga" --lang "en" --title "My Manga" --folder "manga_folder" --volume "1" --chapter "1" --saver --log
//!
//!
//!# Manga Downloader
//!
//!This crate is a command-line tool for downloading manga, managing manga databases, and interacting with the application through various modes. It supports functionalities such as downloading manga, checking and updating database entries, and logging.
//!
//!## Modules
//!
//!- **args**: Handles command-line arguments and configuration.
//!- **db**: Manages database operations.
//!- **download**: Manages the manga downloading process.
//!- **getter**: Provides functions for retrieving data.
//!- **macros**: Contains custom macros used throughout the crate.
//!- **metadata**: Manages metadata related to manga.
//!- **resolute**: Handles finalization and resolution of application state.
//!- **utils**: Provides utility functions for various tasks.
//!- **zip_func**: Handles zip file operations.
//!
//!### Optional Features
//!
//!- **music**: Plays background music during downloads (enabled with the `music` feature).
//!- **gui**: Provides a graphical user interface (enabled with the `gui` feature).
//!- **server**: Enables server mode (enabled with the `server` feature).
//!- **web**: Provides web-based interaction (enabled with the `web` feature).

use chrono::DateTime;
use crosscurses::stdscr;
use glob::glob;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use remove_dir_all::remove_dir_all;
use serde_json::Value;
use std::{ cmp::Ordering, env, fs::{ self, File }, io::Write, process::exit, sync::Arc };

mod args;
mod db;
mod download;
mod error;
mod getter;
mod macros;
mod metadata;
mod resolute;
mod tutorial;
mod utils;
mod version_manager;
mod zip_func;

#[cfg(feature = "music")]
mod music;

#[cfg(feature = "gui")]
mod gui;

#[cfg(feature = "server")]
mod server;

#[cfg(feature = "web")]
mod web;

/// Displays a string on the screen at the specified coordinates.
///
/// This function writes the given `value` string to the terminal screen at the
/// position specified by `(y, x)` coordinates. It respects the terminal's maximum
/// width to avoid overwriting other parts of the screen. The output is only refreshed
/// if certain global flags are not set, which indicates whether the program is running
/// in web mode, GUI mode, or other special states.
///
/// # Parameters
///
/// - `y`: The y-coordinate (row) on the screen where the string should be displayed.
/// - `x`: The x-coordinate (column) on the screen where the string should start.
/// - `value`: The string to be displayed on the screen.
///
/// # Note
///
/// This function uses `stdscr()` from the `curses` library to manage terminal output.
#[inline]
fn string(y: u32, x: u32, value: &str) {
    if
        *args::ARGS_WEB ||
        *args::ARGS_GUI ||
        *args::ARGS_CHECK ||
        *args::ARGS_UPDATE ||
        *args::ARGS_QUIET
    {
        return;
    }
    stdscr().mvaddnstr(y as i32, x as i32, value, (MAXPOINTS.max_x - x) as i32);
    stdscr().refresh();
}

/// Logs the end of a process identified by `handle_id`.
///
/// This function adds the provided `handle_id` to the list of completed process handles.
/// This is used for tracking and managing the end of various tasks or operations.
///
/// # Parameters
///
/// - `handle_id`: A string that represents the unique identifier of the process or operation
///   that has ended.
///
/// # Note
///
/// This function utilizes a mutex to ensure thread-safe access to the `HANDLE_ID_END` list.
#[inline]
fn log_end(handle_id: Box<str>) {
    resolute::HANDLE_ID_END.lock().push(handle_id);
}

lazy_static! {
    /// Stores the maximum dimensions of the terminal screen.
    ///
    /// This lazy-static variable holds the maximum width (`max_x`) and height (`max_y`) of the
    /// terminal screen as determined at runtime. It is used to constrain output and ensure that
    /// it fits within the terminal boundaries.
    pub(crate) static ref MAXPOINTS: metadata::MaxPoints = metadata::MaxPoints {
        max_x: match stdscr().get_max_x() {
            value @ 0.. => value as u32,
            _ => 100,
        },
        max_y: match stdscr().get_max_y() {
            value @ 0.. => value as u32,
            _ => 100,
        },
    };

    /// Indicates whether the final end state has been reached.
    ///
    /// This mutex-protected boolean value is used to determine if the program has reached
    /// its final end state, allowing for graceful exit.
    pub(crate) static ref IS_END: Mutex<bool> = Mutex::new(false);
}

/// Returns a message indicating that the provided UUID is in an incorrect format.
///
/// This function is used to handle cases where a UUID provided in the `url` does not conform to the expected format. It logs a message indicating the wrong format and provides guidance on the correct UUID format. The function returns a placeholder string `"*"`, indicating that the UUID format is invalid.
///
/// # Parameters:
/// - `url: &str` - The string representation of the UUID that is being validated.
///
/// # Returns:
/// - `&str` - A placeholder string `"*"` indicating that the UUID format is invalid.
///
/// # Example:
/// ```rust
/// let url = "123e4567-e89b-12d3-a456-426614174000";
/// let result = wrong_uuid_format(url);
/// println!("{}", result);  // Output: "*"
/// ```
///
/// # Notes:
/// - The function also calls a `string` function (not provided here) to log the invalid UUID format and the expected format.
/// - The expected UUID format is `"8-4-4-4-12"`, which corresponds to a standard UUID format (e.g., `123e4567-e89b-12d3-a456-426614174000`).
///
/// # Error Handling:
/// - This function does not handle errors directly but provides a warning message about the incorrect UUID format.
#[inline]
fn wrong_uuid_format(url: &str) -> &str {
    string(3, 0, &format!("Wrong format of UUID ({})", url));
    string(4, 0, "Should be 8-4-4-4-12 (123e4567-e89b-12d3-a456-426614174000)");
    "*"
}

#[tokio::main]
async fn main() {
    // Attempt to start the application and handle any errors that may occur.
    match start().await {
        Ok(()) => error::handle_suspended(),
        Err(err) => {
            let err_code = error::handle_final(&err);
            // Attempt to remove any cache files and ignore errors.
            match utils::remove_cache() {
                Ok(()) => (),
                Err(_err) => (),
            }
            exit(err_code);
        }
    }

    // Attempt to remove any cache files and ignore errors.
    match utils::remove_cache() {
        Ok(()) => (),
        Err(_err) => (),
    }

    // If no special flags or arguments are set, configure terminal input modes.
    if *resolute::INITSCR_INIT.lock() {
        crosscurses::echo();
        crosscurses::cbreak();
    }

    // If the final end state has been reached, exit the program successfully.
    if *resolute::FINAL_END.lock() {
        exit(0);
    }
}

/// Initializes and starts the application based on provided arguments and settings.
///
/// This asynchronous function performs the following tasks in order:
/// 1. **Setup Settings**: Loads configuration settings from the database.
/// 2. **Argument Handling**: Handles special arguments for encoding, resetting, and other operations.
/// 3. **Database Initialization**: Initializes the database.
/// 4. **Current Directory Setup**: Sets the working directory for the application.
/// 5. **Delete Operation**: Handles deletion of data if the corresponding argument is set.
/// 6. **Log Display**: Displays logs if the corresponding argument is set.
/// 7. **Cache Folder Creation**: Creates a cache folder for the application.
/// 8. **Music Feature**: Starts the music feature if enabled and specified.
/// 9. **Subscriber Setup**: Sets up subscribers for web, GUI, update, or server modes.
/// 10. **Log Handler**: Starts the log handler if enabled.
/// 11. **Language Setup**: Configures the language settings for the application.
/// 12. **Show or Show All**: Displays additional information if specified.
/// 13. **Check or Update**: Performs checks or updates if specified.
/// 14. **Server Mode**: Starts the server if enabled and specified.
/// 15. **GUI Mode**: Starts the GUI if enabled and specified.
/// 16. **Web Mode**: Starts the web interface if enabled and specified.
/// 17. **Resolve Start**: Resolves the starting path and requirements for the application.
/// 18. **UUID Handling**: Handles and validates UUIDs for data retrieval.
/// 19. **Manga Information Retrieval**: Retrieves and processes manga information.
/// 20. **Resolve End**: Finalizes the process and cleans up.
///
/// # Returns
///
/// This function returns a `Result<(), error::MdownError>`. It returns `Ok(())` if all operations are successful.
/// Otherwise, it returns an `Err` with details about the encountered error.
///
/// # Errors
///
/// Possible errors include database setup failures, invalid arguments, file I/O errors, JSON parsing errors,
/// and HTTP request errors. Each step handles specific errors and reports them accordingly.
///
/// # Notes
///
/// - The function utilizes conditional compilation to include or exclude features like web, music, server,
///   and GUI based on feature flags.
/// - Debug messages are used extensively to trace the execution flow and aid in debugging.
async fn start() -> Result<(), error::MdownError> {
    // Setup configuration settings from the database
    let (settings, changed) = match db::setup_settings() {
        Ok((settings, changed)) => (settings, changed),
        Err(err) => {
            return Err(error::MdownError::ChainedError(Box::new(err), 10111));
        }
    };

    match version_manager::check_app_ver() {
        Ok(_value) => (),
        Err(err) => {
            return Err(error::MdownError::ChainedError(Box::new(err), 10112));
        }
    }

    if *args::ARGS_APP_UPDATE {
        return match version_manager::app_update().await {
            Ok(_) => Ok(()),
            Err(err) => Err(error::MdownError::ChainedError(Box::new(err), 10113)),
        };
    }

    match version_manager::check_update().await {
        Ok(_) => (),
        Err(err) => {
            handle_error!(&err);
        }
    }

    if args::ARGS.lock().backup {
        return match utils::backup_handler(true) {
            Ok(()) => Ok(()),
            Err(err) => Err(error::MdownError::ChainedError(Box::new(err), 10114)),
        };
    }

    if changed {
        return Ok(());
    }

    if *args::ARGS_SHOW_SETTINGS {
        utils::show_settings(settings);
        return Ok(());
    }

    // Update arguments with folder settings from the configuration
    args::ARGS.lock().change_settings(settings);

    // Handle encoding argument
    if !(*args::ARGS_ENCODE).is_empty() {
        debug!("start encode");
        #[cfg(feature = "web")]
        println!("{}", web::encode(&args::ARGS_ENCODE));
        #[cfg(not(feature = "web"))]
        println!("Encode is not supported; You have to enable web feature");
        return Ok(());
    }

    // Handle reset argument
    if *args::ARGS_RESET {
        debug!("args_reset");
        return utils::reset();
    }

    // Handle backup argument
    if *args::ARGS_CH_BACKUP {
        debug!("args_ch_backup");
        return utils::backup_choose();
    }

    // Initialize the database
    match db::init().await {
        Ok(()) => (),
        Err(err) => {
            return Err(error::MdownError::ChainedError(Box::new(err), 10115));
        }
    }

    // Set the current working directory
    match env::set_current_dir(args::ARGS_CWD.as_str()) {
        Ok(()) => debug!("cwd set to {}", *args::ARGS_CWD),
        Err(err) => {
            return Err(error::MdownError::IoError(err, args::ARGS_CWD.to_string(), 10100));
        }
    }

    // Handle delete argument
    if *args::ARGS_DELETE {
        return resolute::args_delete();
    }

    // Handle show log argument
    if *args::ARGS_SHOW_LOG {
        debug!("show_log");
        return resolute::show_log().await;
    }

    // Create cache folder
    match utils::create_cache_folder() {
        Ok(()) => debug!("created cache folder"),
        Err(err) => {
            return Err(error::MdownError::ChainedError(Box::new(err), 10116));
        }
    }

    // Handle music feature
    if args::ARGS.lock().music.is_some() {
        #[cfg(feature = "music")]
        {
            tokio::spawn(async { music::start() });
            debug!("music instance started");
        }
        #[cfg(not(feature = "music"))]
        eprintln!("Music feature is not enabled; You have to enable music feature");
    }

    // Setup subscriber for web, GUI, update, or server modes
    if *args::ARGS_WEB || *args::ARGS_GUI || *args::ARGS_UPDATE || *args::ARGS_SERVER {
        match utils::setup_subscriber() {
            Ok(()) => debug!("setup subscriber"),
            Err(err) => {
                return Err(error::MdownError::ChainedError(Box::new(err), 10117));
            }
        }
    }

    // Start log handler if enabled
    if *args::ARGS_LOG {
        debug!("log_handler instance started");
        tokio::spawn(async { utils::log_handler() });
    }

    // Handle show or show all arguments
    if args::ARGS_SHOW.is_some() || args::ARGS_SHOW_ALL.is_some() {
        debug!("show || show all");
        return resolute::show().await;
    }

    // Set language to download
    *resolute::LANGUAGE.lock() = args::ARGS.lock().lang.clone();
    debug!("language is set to {}", &args::ARGS.lock().lang);

    // Perform check or update operations
    if *args::ARGS_CHECK || *args::ARGS_UPDATE {
        debug!("start resolve_check");
        return resolute::resolve_check().await;
    }

    // Handle server mode
    if *args::ARGS_SERVER {
        debug!("start server");
        #[cfg(feature = "server")]
        return server::start();
        #[cfg(not(feature = "server"))]
        {
            println!("Server is not supported");
            *resolute::ENDED.lock() = true;
            return Ok(());
        }
    }

    // Handle GUI mode
    if *args::ARGS_GUI {
        debug!("start gui");
        #[cfg(feature = "gui")]
        return gui::start();
        #[cfg(not(feature = "gui"))]
        {
            println!("Gui is not supported");
            *resolute::ENDED.lock() = true;
            return Ok(());
        }
    }

    // Handle web mode
    if *args::ARGS_WEB {
        debug!("start web");
        #[cfg(feature = "web")]
        return web::start().await;
        #[cfg(not(feature = "web"))]
        {
            println!("Web is not supported");
            *resolute::ENDED.lock() = true;
            return Ok(());
        }
    }

    // Resolve starting file path and requirements
    let main_lock_file_path = match utils::main_lock_file() {
        Ok(main_lock_file_path) => main_lock_file_path,
        Err(err) => {
            return Err(error::MdownError::ChainedError(Box::new(err), 10118));
        }
    };

    // Setup requirements if not in quiet mode
    if !*args::ARGS_QUIET {
        utils::setup_requirements(main_lock_file_path.clone());
    }

    // Initialize manga name and status code
    let mut manga_name = String::from("!");
    let mut status_code = match reqwest::StatusCode::from_u16(200) {
        Ok(code) => code,
        Err(err) => {
            return Err(
                error::MdownError::CustomError(
                    err.to_string(),
                    String::from("InvalidStatusCode"),
                    10101
                )
            );
        }
    };
    let mut err_code_network = 0;
    let url_from_search;

    // Retrieve and debug URL
    let url = args::ARGS.lock().url.clone();
    debug!("\nstarting to search for uuid in '{}'", url);

    // Handle UUID retrieval and validation
    let id = if args::ARGS.lock().search.as_str() != "*" {
        debug!("using search");
        url_from_search = match utils::search().await {
            Ok(id) => id,
            Err(err) => {
                return Err(error::MdownError::ChainedError(Box::new(err), 10119));
            }
        };
        url_from_search.as_str()
    } else if let Some(id_temp) = utils::resolve_regex(&url) {
        debug!("using whole url");
        if utils::is_valid_uuid(id_temp.as_str()) {
            id_temp.as_str()
        } else {
            wrong_uuid_format(&url)
        }
    } else if utils::is_valid_uuid(&url) {
        debug!("using url as uuid");
        url.as_str()
    } else if url == "UNSPECIFIED" {
        debug!("url is not specified");
        "*"
    } else {
        wrong_uuid_format(&url)
    };

    // Process manga information if valid ID is found
    if id != "*" {
        match process_manga_json(&id, &mut err_code_network, &mut status_code).await {
            Ok(manga_name_json) => {
                let obj = match perform_manga_download(manga_name_json).await {
                    Ok(obj) => obj,
                    Err(err) => {
                        return Err(error::MdownError::ChainedError(Box::new(err), 10120));
                    }
                };
                manga_name = match resolute::resolve(obj, &id).await {
                    Ok(value) => value,
                    Err(err) => {
                        handle_error!(&err, String::from("program"));
                        String::from("!")
                    }
                };
            }
            Err(err) => {
                return Err(error::MdownError::ChainedError(Box::new(err), 10121));
            }
        }
    } else {
        debug!("unable to get uuid");
    }

    // Finalize the process and cleanup
    match utils::resolve_end(&main_lock_file_path, &manga_name, status_code, err_code_network) {
        Ok(()) => (),
        Err(err) => eprintln!("Error: {}", err),
    }

    if args::ARGS.lock().backup {
        match utils::backup_handler(false) {
            Ok(()) => (),
            Err(err) => debug!("Error: {}", err),
        }
    }

    utils::resolve_final_end();

    *resolute::ENDED.lock() = true;

    // Final key input is handled in `utils::ctrl_handler`
    Ok(())
}

/// Asynchronously processes manga JSON data.
///
/// This function retrieves manga data based on the provided `id`, processes it, and handles errors that may occur during the network request. It updates the error codes and status codes passed by reference and logs relevant information during the process. The function returns the manga information in JSON format if successful, or returns a custom error if it encounters any issues.
///
/// # Parameters:
/// - `id: &str` - The ID of the manga to fetch data for. This ID will be used in the network request to retrieve the corresponding manga information.
/// - `err_code_network: &mut u32` - A mutable reference to a network error code. This will be updated if a network error occurs during the manga data retrieval.
/// - `status_code: &mut reqwest::StatusCode` - A mutable reference to the HTTP status code. This will be updated with the status code from the request.
///
/// # Returns:
/// - `Result<String, error::MdownError>` - The result of the operation. If successful, it returns the manga data as a `String` (JSON format). If there is an error, it returns a `CustomError`.
///
/// # Example:
/// ```rust
/// let manga_data = process_manga_json("manga_id", &mut err_code, &mut status_code).await;
/// match manga_data {
///     Ok(data) => println!("Manga data: {}", data),
///     Err(e) => println!("Error: {:?}", e),
/// }
/// ```
///
/// # Notes:
/// - This function performs logging at different stages of the process, such as when the manga ID is acquired and when the manga data is successfully retrieved or encounters an error.
/// - If the tutorial flag is enabled, the tutorial information is displayed.
/// - In case of a network error, the function processes the error and updates the error code and status code accordingly.
/// - The `get_manga_json` function is awaited and, if successful, returns the manga data in JSON format. In case of failure, the function calls `process_manga_error` to handle the error.
async fn process_manga_json(
    id: &str,
    err_code_network: &mut u32,
    status_code: &mut reqwest::StatusCode
) -> Result<String, error::MdownError> {
    debug!("id acquired: {}\n", id);
    *resolute::MANGA_ID.lock() = id.to_string();
    string(0, 0, &format!("Extracted ID: {}", id));
    match db::check_tutorial() {
        Ok(()) => (),
        Err(err) => {
            error::suspend_error(err);
        }
    }
    string(1, 0, "Getting manga information ...");
    if *tutorial::TUTORIAL.lock() {
        tutorial::manga_info();
    }
    match getter::get_manga_json(id).await {
        Ok(manga_name_json) => {
            string(1, 0, "Getting manga information DONE");
            return Ok(manga_name_json);
        }
        Err(code) => {
            string(1, 0, "Getting manga information ERROR");
            process_manga_error(code, err_code_network, status_code);
            return Err(error::MdownError::CustomError(String::from(""), String::from(""), 10110));
        }
    }
}

/// Asynchronously performs a manga download and returns the parsed manga data.
///
/// This function downloads manga data based on the provided JSON string. It processes the JSON data and returns the parsed manga information as a `serde_json::Map`. In case of any error, it returns an appropriate error.
///
/// # Parameters:
/// - `manga_name_json: String` - The manga data in JSON format, usually obtained from a previous API call. This JSON string is parsed into a `serde_json::Map` for further processing.
///
/// # Returns:
/// - `Result<serde_json::Map<String, serde_json::Value>, error::MdownError>` - A result indicating the success or failure of the operation. On success, it returns a `serde_json::Map` containing the parsed manga data. If the data is not in the expected format, it returns a `JsonError`.
///
/// # Example:
/// ```rust
/// let manga_data = perform_manga_download(manga_name_json).await;
/// match manga_data {
///     Ok(data) => println!("Manga data: {:?}", data),
///     Err(e) => println!("Error: {:?}", e),
/// }
/// ```
///
/// # Notes:
/// - The function first checks for the "music" feature and, if enabled, sets the music stage to `Init`.
/// - It then attempts to parse the `manga_name_json` string into a JSON object (`serde_json::Map`).
/// - If the JSON is valid and correctly formatted, it returns the parsed object. If the format is unexpected, it returns a `JsonError`.
/// - If an error occurs while fetching or parsing the JSON, a `ChainedError` is returned with additional context.
///
/// # Error Handling:
/// - If the function encounters an error while fetching or parsing the JSON, it returns an appropriate `MdownError`, such as a `ChainedError` or a `JsonError`.
async fn perform_manga_download(
    manga_name_json: String
) -> Result<serde_json::Map<String, serde_json::Value>, error::MdownError> {
    #[cfg(feature = "music")]
    {
        *resolute::MUSIC_STAGE.lock() = metadata::MusicStage::Init;
    }
    let json_value = match utils::get_json(&manga_name_json) {
        Ok(value) => value,
        Err(err) => {
            return Err(error::MdownError::ChainedError(Box::new(err), 10122));
        }
    };
    if let Value::Object(obj) = json_value {
        debug!("parsed manga information");
        return Ok(obj);
    } else {
        Err(error::MdownError::JsonError(String::from("Unexpected JSON value"), 10102))
    }
}

/// Processes errors that occur during manga-related network operations.
///
/// This function handles different types of errors during the manga download process, such as network errors or status errors, and updates the network error code and status code accordingly.
///
/// # Parameters:
/// - `code: error::MdownError` - The error that occurred during the operation. It can be a network-related error (`NetworkError`), a status error (`StatusError`), or other types of errors.
/// - `err_code_network: &mut u32` - A mutable reference to the network error code. This value will be updated based on the error type.
/// - `status_code: &mut reqwest::StatusCode` - A mutable reference to the status code. This value will be updated with the error's status code, if applicable.
///
/// # Returns:
/// This function does not return a value. It updates the `err_code_network` and `status_code` references passed to it based on the error type.
///
/// # Example:
/// ```rust
/// let mut err_code_network = 0;
/// let mut status_code = reqwest::StatusCode::OK;
/// process_manga_error(MdownError::NetworkError(...), &mut err_code_network, &mut status_code);
/// ```
///
/// # Notes:
/// - The function matches on the type of error (`MdownError`) and processes it accordingly.
/// - If the error is a `NetworkError`, the function updates the network error code and attempts to extract the status from the error.
/// - If the error is a `StatusError`, the function updates both the network error code and the status code with the information from the error.
/// - For other error types, the function prints a generic "Unexpected error" message.
///
/// # Error Handling:
/// - The function ensures that both the `err_code_network` and `status_code` are updated based on the error's specific type. If an unknown error type is encountered, it prints a message indicating an unexpected error.
fn process_manga_error(
    code: error::MdownError,
    err_code_network: &mut u32,
    status_code: &mut reqwest::StatusCode
) {
    match code {
        error::MdownError::NetworkError(ref err, err_code) => {
            *err_code_network = err_code;
            if let Some(status_error) = err.status() {
                *status_code = status_error;
            } else {
                resolve_error_code(code, status_code);
            }
        }
        error::MdownError::StatusError(ref err, err_code) => {
            *err_code_network = err_code;
            *status_code = *err;
        }
        _ => {
            println!("Unexpected error");
        }
    }
}

/// Resolves an error code into an appropriate HTTP status code.
///
/// This function takes a custom error (`MdownError`), extracts the relevant status code from it (if available), and updates the provided `status_code` with the corresponding `reqwest::StatusCode`. It helps map custom error codes to valid HTTP status codes for further handling.
///
/// # Parameters:
/// - `code: error::MdownError` - The error that contains an error code as part of its message. The error code should be a numeric value at the start of the error string (e.g., "404 Not Found").
/// - `status_code: &mut reqwest::StatusCode` - A mutable reference to the `status_code` variable. This will be updated with the resolved HTTP status code if a valid one is found.
///
/// # Returns:
/// This function does not return a value. It modifies the `status_code` reference based on the error code parsed from the `MdownError`.
///
/// # Example:
/// ```rust
/// let mut status_code = reqwest::StatusCode::OK;
/// let error = MdownError::NetworkError("404 Not Found".into(), 404);
/// resolve_error_code(error, &mut status_code);
/// assert_eq!(status_code, reqwest::StatusCode::NOT_FOUND);
/// ```
///
/// # Notes:
/// - The function splits the error message by whitespace and tries to parse the first part as a numeric status code.
/// - If a valid numeric status code is found, it attempts to convert it into a `reqwest::StatusCode` using `StatusCode::from_u16`.
/// - If parsing or status code conversion fails, it leaves the `status_code` unchanged or defaults to `StatusCode::OK` in some cases.
///
/// # Error Handling:
/// - If the error message does not contain a valid numeric status code or if the conversion fails, the function logs a message saying "Invalid status string."
fn resolve_error_code(code: error::MdownError, status_code: &mut reqwest::StatusCode) {
    let code = code.into();
    let parts: Vec<&str> = code.split_whitespace().collect();
    if let Some(status_code_tmp) = parts.first() {
        let number = status_code_tmp.parse::<u16>().unwrap_or_default();
        if number != 0 {
            if let Ok(status_code_tmp) = reqwest::StatusCode::from_u16(number) {
                *status_code = status_code_tmp;
            }
        }
    } else {
        println!("Invalid status string");
    }
}

/// Downloads manga chapters based on the provided manga JSON data and arguments.
///
/// This asynchronous function performs the following tasks:
/// 1. **Initial Setup**: Initializes various internal state variables and settings.
/// 2. **File Search**: Searches for existing `.cbz` files and collects their metadata.
/// 3. **JSON Parsing**: Parses the provided manga JSON to extract chapter information.
/// 4. **Chapter Processing**: Iterates over each chapter, checking conditions for downloading based on various parameters.
/// 5. **File Download**: Downloads and saves the chapter data if conditions are met.
/// 6. **Finalization**: Finalizes the download process, including cleanup and logging.
///
/// # Parameters
///
/// - `manga_json: String`
///   The JSON string containing manga data to be processed.
/// - `arg_force: bool`
///   A flag indicating whether to force download even if the chapter is already downloaded.
///
/// # Returns
///
/// This function returns a `Result<Vec<String>, error::MdownError>`. It returns:
/// - `Ok(Vec<String>)` containing a list of filenames of successfully downloaded chapters.
/// - `Err(error::MdownError)` if any errors occur during the process.
///
/// # Errors
///
/// Possible errors include:
/// - JSON parsing errors if the provided manga JSON is invalid.
/// - File I/O errors when searching for existing `.cbz` files or during file operations.
/// - Errors from network requests or JSON deserialization of chapter data.
/// - Any custom errors related to metadata handling or file operations.
///
/// # Notes
///
/// - The function performs extensive logging and debugging to trace the process and identify issues.
/// - It supports various conditions for skipping chapters based on user arguments, existing files, and metadata.
/// - Utilizes concurrency with asynchronous operations for downloading and file processing.
///
pub(crate) async fn download_manga(
    manga_json: String,
    arg_force: bool
) -> Result<Vec<String>, error::MdownError> {
    debug!("");
    debug!("download_manga");

    // Reset the current chapter parsed counter
    *resolute::CURRENT_CHAPTER_PARSED.lock() = 0;

    // Retrieve and clone necessary settings
    let manga_name = &*resolute::MANGA_NAME.lock().clone();
    let volume = args::ARGS.lock().volume.clone();
    let chapter = args::ARGS.lock().chapter.clone();
    // Volume to download set by user
    let arg_volume = getter::get_arg(&volume);
    // Chapter to download set by user
    let arg_chapter = getter::get_arg(&chapter);
    // Offset set by user
    let arg_offset: u32 = getter::get_arg(&args::ARGS.lock().offset).parse().unwrap_or_default();
    // Initialize storage for downloaded files and other metrics
    let (mut downloaded, hist) = (vec![], &mut vec![]);
    let (mut times, mut moves) = (0, 0);
    let language = resolute::LANGUAGE.lock().clone();
    let mut filename;
    let json_value = match utils::get_json(&manga_json) {
        Ok(value) => value,
        Err(err) => {
            return Err(error::MdownError::ChainedError(Box::new(err), 10123));
        }
    };
    // To rate limit the download speed so we would not exceed rate limit
    let interval = std::time::Duration::from_millis(1000);

    let mut all_ids = vec![];
    let mut all_num = vec![];

    debug!("checking for .cbz files in {}", resolute::MWD.lock());

    // Search for existing .cbz files and collect their metadata
    if let Ok(value) = glob("*.cbz") {
        for entry in value.filter_map(Result::ok) {
            if let Some(entry) = entry.to_str() {
                debug!("found entry in glob: {}", entry);
                if let Ok(manga) = resolute::check_for_metadata(entry) {
                    all_ids.push(manga.id.clone());
                    all_num.push(manga.chapter.clone());
                }
            }
        }
    }
    // Search for existing .cbz files and collect their metadata
    if let Ok(value) = glob(&format!("{}\\*.cbz", getter::get_folder_name())) {
        for entry in value.filter_map(Result::ok) {
            if let Some(entry) = entry.to_str() {
                debug!("found entry in glob: {}", entry);
                if let Ok(manga) = resolute::check_for_metadata(entry) {
                    all_ids.push(manga.id.clone());
                    all_num.push(manga.chapter.clone());
                }
            }
        }
    }
    debug!("found ids {:?}", all_ids);
    debug!("found chapter numbers {:?}", all_num);

    let mut tutorial_skip = true;
    let mut tutorial_metadata = true;

    // Parse the manga JSON to extract chapter information
    match serde_json::from_value::<metadata::MangaResponse>(json_value) {
        Ok(obj) => {
            debug!("parsed manga data");
            let mut data_array = utils::sort(&obj.data);
            debug!("data array sorted");

            match resolute::parse_scanlation_file() {
                Ok(()) => (),
                Err(_err) => (),
            }

            let data_len = data_array.len();
            *resolute::CURRENT_CHAPTER_PARSED_MAX.lock() = data_len as u64;

            if !*args::ARGS_CHECK {
                let mut index = 0;
                let mut data_number = 0;
                while index < data_array.len() {
                    let parsed = format!(
                        "   Parsed chapters: {}/{}",
                        resolute::CURRENT_CHAPTER_PARSED.lock(),
                        resolute::CURRENT_CHAPTER_PARSED_MAX.lock()
                    );
                    string(0, MAXPOINTS.max_x - (parsed.len() as u32), &parsed);

                    let array_item = getter::get_attr_as_same_from_vec(&data_array, index);
                    let value = array_item.id.clone();
                    let id = value.trim_matches('"');
                    let id_string = id.to_string();
                    let (chapter_attr, lang, _, chapter_num, title) =
                        getter::get_metadata(array_item);

                    if
                        (all_num.contains(&chapter_num) || all_ids.contains(&id_string)) &&
                        !arg_force
                    {
                        data_array.remove(index);
                        let vol = match chapter_attr.volume.unwrap_or_default().as_str() {
                            "" => String::new(),
                            value => format!("Vol.{} ", value),
                        };

                        filename = utils::FileName {
                            manga_name: manga_name.to_string(),
                            vol: vol.to_string(),
                            chapter_num: chapter_num.to_string(),
                            title: title.to_string(),
                            folder: getter::get_folder_name().to_string(),
                        };

                        let folder_path = filename.get_folder_name();

                        if lang != language && language != "*" {
                            utils::skip(folder_path, data_number, &mut moves, hist, 1);
                            debug!(
                                "Removing {} from data array because wrong language; found '{}', target '{}'",
                                id_string,
                                lang,
                                language
                            );
                        } else {
                            utils::skip(folder_path, data_number, &mut moves, hist, 1);
                            debug!("Removing {} from data array because is already downloaded", id_string);
                        }
                        *resolute::CURRENT_CHAPTER_PARSED_MAX.lock() -= 1;
                    } else if lang != language && language != "*" {
                        data_array.remove(index);
                        let vol = match chapter_attr.volume.unwrap_or_default().as_str() {
                            "" => String::new(),
                            value => format!("Vol.{} ", value),
                        };

                        filename = utils::FileName {
                            manga_name: manga_name.to_string(),
                            vol: vol.to_string(),
                            chapter_num: chapter_num.to_string(),
                            title: title.to_string(),
                            folder: getter::get_folder_name().to_string(),
                        };

                        let folder_path = filename.get_folder_name();
                        utils::skip(folder_path, data_number, &mut moves, hist, 1);
                        *resolute::CURRENT_CHAPTER_PARSED_MAX.lock() -= 1;
                        debug!(
                            "Removing {} from data array because wrong language; found '{}', target '{}'",
                            id_string,
                            lang,
                            language
                        );
                    } else {
                        index += 1;
                    }
                    data_number += 1;
                }
                debug!("Removed {} entries", data_len - index);
            }
            let data_len = data_array.len();
            *resolute::CURRENT_CHAPTER_PARSED_MAX.lock() = data_len as u64;
            utils::clear_screen(1);

            // Process each chapter
            for item in 0..data_len {
                debug!("parsing chapter entry {}", item);
                let mut date_change = false;
                let parsed = format!(
                    "   Parsed chapters: {}/{}",
                    resolute::CURRENT_CHAPTER_PARSED.lock(),
                    resolute::CURRENT_CHAPTER_PARSED_MAX.lock()
                );
                string(0, MAXPOINTS.max_x - (parsed.len() as u32), &parsed);

                let array_item = getter::get_attr_as_same_from_vec(&data_array, item);
                let value = array_item.id.clone();
                let id = value.trim_matches('"');
                let id_string = id.to_string();
                *resolute::CHAPTER_ID.lock() = id.to_string().clone();

                debug!("chapter id: {}", id);

                let message = format!("({}) Found chapter with id: {}", item as u32, id);
                if
                    *args::ARGS_WEB ||
                    *args::ARGS_GUI ||
                    *args::ARGS_CHECK ||
                    *args::ARGS_UPDATE ||
                    *args::ARGS_LOG
                {
                    log!(&message);
                }
                string(1, 0, &format!(" {}", message));

                if *tutorial::TUTORIAL.lock() && item == 0 {
                    tutorial::found_chapter();
                }

                let (chapter_attr, lang, pages, chapter_num, mut title) =
                    getter::get_metadata(array_item);

                title = resolute::title(title);

                let vol = match chapter_attr.volume.unwrap_or_default().as_str() {
                    "" => String::new(),
                    value => format!(" Vol.{} ;", value),
                };

                let con_chap = resolute::resolve_skip(arg_chapter, &chapter_num);
                let con_vol = resolute::resolve_skip(arg_volume, &vol);

                filename = utils::FileName {
                    manga_name: manga_name.to_string(),
                    vol: vol.to_string(),
                    chapter_num: chapter_num.to_string(),
                    title: title.to_string(),
                    folder: getter::get_folder_name().to_string(),
                };
                let folder_path = filename.get_folder_name();

                // Determine if chapter should be downloaded
                if
                    (lang == language || language == "*") &&
                    fs::metadata(filename.get_file_w_folder()).is_ok() &&
                    !arg_force &&
                    !(match resolute::check_for_metadata_saver(&filename.get_file_w_folder()) {
                        Ok(metadata) => if !*args::ARGS_CHECK { metadata } else { false }
                        Err(err) => {
                            return Err(error::MdownError::ChainedError(Box::new(err), 10124));
                        }
                    }) &&
                    ({
                        if *args::ARGS_CHECK {
                            let chapter_ids = resolute::CHAPTER_IDS.lock();
                            let data_id = match chapter_ids.get(&chapter_num) {
                                Some(id) => id,
                                None => &String::new(),
                            };
                            !(data_id != id && *data_id != String::new())
                        } else {
                            true
                        }
                    })
                {
                    debug!("found downloaded chapter and have same saver value as user defined");
                    let mut cont = true;
                    let update_date = chapter_attr.updatedAt.clone();
                    match DateTime::parse_from_rfc3339(&update_date) {
                        Ok(datetime) => {
                            let mut dates = resolute::CHAPTER_DATES.lock();
                            let empty = String::new();

                            let cur_date = match dates.get(&chapter_num) {
                                Some(date) => date.to_owned(),
                                None => empty,
                            };

                            match DateTime::parse_from_rfc3339(&cur_date) {
                                Ok(datetime_cur) => {
                                    match datetime_cur.cmp(&datetime) {
                                        Ordering::Greater => {
                                            debug!(
                                                "dates didn't match but date in local database was ahead of the date in mangadex database"
                                            );
                                            resolute::FIXED_DATES
                                                .lock()
                                                .push(chapter_num.to_string());
                                            resolute::CHAPTERS_TO_REMOVE
                                                .lock()
                                                .push(
                                                    metadata::ChapterMetadata::new(
                                                        &chapter_num,
                                                        &cur_date,
                                                        id
                                                    )
                                                );
                                        }
                                        Ordering::Less => {
                                            debug!(
                                                "dates didn't match so program will download it if update flag is set"
                                            );
                                            date_change = true;
                                            cont = false;
                                            dates.remove(&chapter_num);
                                            if *args::ARGS_UPDATE {
                                                resolute::CHAPTERS_TO_REMOVE
                                                    .lock()
                                                    .push(
                                                        metadata::ChapterMetadata::new(
                                                            &chapter_num,
                                                            &cur_date,
                                                            id
                                                        )
                                                    );
                                            }
                                        }
                                        Ordering::Equal => (),
                                    }
                                }
                                Err(_err) => (),
                            }
                        }
                        Err(_err) => (),
                    }
                    *resolute::CURRENT_CHAPTER_PARSED.lock() += 1;
                    if cont && (lang == language || language == "*") {
                        resolute::CHAPTERS
                            .lock()
                            .push(metadata::ChapterMetadata::new(&chapter_num, &update_date, id));
                        utils::skip(folder_path, item, &mut moves, hist, 2);
                        continue;
                    }
                }

                // Skip chapter if conditions are not met
                if con_vol {
                    debug!("skipping because volume didn't match");
                    utils::skip_didnt_match("volume", item, &mut moves, hist);
                    if *tutorial::TUTORIAL.lock() && tutorial_skip {
                        tutorial::skip();
                        tutorial_skip = false;
                    }
                    continue;
                }
                if con_chap {
                    debug!("skipping because chapter didn't match");
                    utils::skip_didnt_match("chapter", item, &mut moves, hist);
                    if *tutorial::TUTORIAL.lock() && tutorial_skip {
                        tutorial::skip();
                        tutorial_skip = false;
                    }
                    continue;
                }
                if pages == 0 {
                    debug!(
                        "skipping because variable pages is 0; probably because chapter is not supported on mangadex, third party"
                    );
                    utils::skip_custom("pages is 0", item, &mut moves, hist);
                    if *tutorial::TUTORIAL.lock() && tutorial_skip {
                        tutorial::skip();
                        tutorial_skip = false;
                    }
                    continue;
                }
                if
                    ((lang == language || language == "*") &&
                        !resolute::CHAPTERS
                            .lock()
                            .iter()
                            .any(|item| item.number == chapter_num) &&
                        !all_ids.contains(&id_string)) ||
                    arg_force
                {
                    if *args::ARGS_CHECK {
                        if all_num.contains(&chapter_num) {
                            debug!("chapter number is already in database");
                            continue;
                        }
                        debug!("{}, {}", id, chapter_num);
                    }
                    debug!("chapter went through customs and is ready to be downloaded");
                    if *args::ARGS_CHECK {
                        let dates = resolute::CHAPTER_DATES.lock();
                        let empty = String::new();

                        let cur_date = match dates.get(&chapter_num) {
                            Some(date) => date.to_owned(),
                            None => empty,
                        };
                        resolute::CHAPTERS_TO_REMOVE
                            .lock()
                            .push(metadata::ChapterMetadata::new(&chapter_num, &cur_date, id));
                    }
                    let update_date = chapter_attr.updatedAt.clone();
                    *resolute::CURRENT_CHAPTER_PARSED.lock() += 1;
                    if arg_offset > times {
                        debug!(
                            "skipping because offset flag is set, {} times more",
                            arg_offset - times
                        );
                        utils::skip_offset(item, &mut moves, hist);
                        times += 1;
                        *resolute::CURRENT_CHAPTER_PARSED.lock() += 1;
                        continue;
                    }
                    utils::clear_screen(2);
                    let folder_path_tmp = &filename.get_folder_w_end();
                    let folder_path = folder_path_tmp.as_str();
                    let message = format!(
                        "  Metadata: Language: {}; Pages: {};{} Chapter: {}{}",
                        lang,
                        pages,
                        vol,
                        chapter_num,
                        match title.as_str() {
                            "" => String::new(),
                            _ => format!("; Title: {}", title),
                        }
                    );
                    if
                        *args::ARGS_WEB ||
                        *args::ARGS_GUI ||
                        *args::ARGS_CHECK ||
                        *args::ARGS_UPDATE ||
                        *args::ARGS_LOG
                    {
                        log!(&message);
                    }
                    string(2, 0, &message);
                    if *tutorial::TUTORIAL.lock() && tutorial_metadata {
                        tutorial::metadata();
                        tutorial_metadata = false;
                    }
                    if
                        !*args::ARGS_CHECK ||
                        !resolute::CHAPTERS
                            .lock()
                            .iter()
                            .any(|chapter| chapter.number == chapter_num)
                    {
                        if *args::ARGS_CHECK {
                            debug!("was added to to download list because check flag is set");
                            match date_change {
                                true =>
                                    resolute::TO_DOWNLOAD_DATE.lock().push(chapter_num.to_string()),
                                false => resolute::TO_DOWNLOAD.lock().push(chapter_num.to_string()),
                            }
                            continue;
                        }
                        let scanlation_group = match resolute::resolve_group(array_item).await {
                            Ok(scanlation_group) => scanlation_group,
                            Err(err) => {
                                handle_error!(&err, String::from("group"));
                                metadata::ScanlationMetadata {
                                    name: String::from("None"),
                                    website: String::from("None"),
                                }
                            }
                        };
                        debug!(
                            "found chapter's scanlation group: {} {}",
                            scanlation_group.name,
                            scanlation_group.website
                        );

                        let start_time = std::time::Instant::now();

                        match getter::get_chapter(id).await {
                            Ok(json) => {
                                let json_value = match utils::get_json(&json) {
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(
                                            error::MdownError::ChainedError(Box::new(err), 10125)
                                        );
                                    }
                                };
                                let obj = match
                                    serde_json::from_value::<metadata::ChapterData>(json_value)
                                {
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(
                                            error::MdownError::JsonError(err.to_string(), 10103)
                                        );
                                    }
                                };
                                #[cfg(feature = "music")]
                                {
                                    *resolute::MUSIC_STAGE.lock() = metadata::MusicStage::Start;
                                }
                                debug!("starting to download chapter");
                                match
                                    download_chapter(
                                        id,
                                        obj,
                                        array_item,
                                        &title,
                                        &filename,
                                        &update_date,
                                        &scanlation_group
                                    ).await
                                {
                                    Ok(()) => (),
                                    Err(err) => handle_error!(&err, String::from("chapter")),
                                }
                            }
                            Err(err) => error::suspend_error(err),
                        }
                        if *IS_END.lock() {
                            return Ok(downloaded);
                        }
                        if !resolute::SCANLATION_GROUPS.lock().contains(&scanlation_group) {
                            match resolute::get_scanlation_group_to_file(&scanlation_group) {
                                Ok(()) => (),
                                Err(err) => {
                                    return Err(
                                        error::MdownError::ChainedError(Box::new(err), 10109)
                                    );
                                }
                            }
                        }
                        utils::clear_screen(5);
                        string(
                            6,
                            0,
                            &format!(
                                "  Converting images to cbz files: {}.cbz",
                                filename.get_folder()
                            )
                        );
                        let file_name = filename.get_file_w_folder();
                        zip_func::to_zip(folder_path, &file_name);
                        match remove_dir_all(folder_path) {
                            Ok(()) => (),
                            Err(err) => {
                                return Err(
                                    error::MdownError::IoError(err, folder_path.to_string(), 10104)
                                );
                            }
                        }

                        let elapsed = std::time::Instant::now().duration_since(start_time);

                        if elapsed < interval && item + 1 != data_len {
                            string(
                                8,
                                0,
                                &format!(
                                    "  Sleeping for {} seconds",
                                    (interval - elapsed).as_secs_f64()
                                )
                            );
                            std::thread::sleep(interval - elapsed);
                        }

                        utils::clear_screen(2);
                        if
                            *args::ARGS_WEB ||
                            *args::ARGS_GUI ||
                            *args::ARGS_CHECK ||
                            *args::ARGS_UPDATE
                        {
                            resolute::WEB_DOWNLOADED.lock().push(file_name);
                        } else {
                            downloaded.push(filename.get_file_w_folder_w_cwd());
                        }
                        let mut current_chapter = resolute::CURRENT_CHAPTER.lock();
                        current_chapter.clear();
                    }
                } else {
                    debug!("skipping because language is wrong");
                    string(2, 0, &" ".repeat(MAXPOINTS.max_x as usize).to_string());
                    let message = format!(
                        "Skipping because of wrong language; found '{}', target '{}' ...",
                        lang,
                        language
                    );
                    string(2, 0, &format!("  {}", message));

                    if *tutorial::TUTORIAL.lock() && tutorial_skip {
                        tutorial::skip();
                        tutorial_skip = false;
                    }

                    if *args::ARGS_CHECK {
                        all_num.push(chapter_num);
                    }

                    if
                        *args::ARGS_WEB ||
                        *args::ARGS_GUI ||
                        *args::ARGS_CHECK ||
                        *args::ARGS_UPDATE ||
                        *args::ARGS_LOG
                    {
                        log!(&format!("({}) {}", item, message));
                    }

                    *resolute::CURRENT_CHAPTER_PARSED_MAX.lock() -= 1;
                }
            }
            let parsed = format!(
                "   Parsed chapters: {}/{}",
                resolute::CURRENT_CHAPTER_PARSED.lock(),
                resolute::CURRENT_CHAPTER_PARSED_MAX.lock()
            );
            string(0, MAXPOINTS.max_x - (parsed.len() as u32), &parsed);
        }
        Err(err) => {
            return Err(error::MdownError::JsonError(err.to_string(), 10105));
        }
    }
    Ok(downloaded)
}

/// Downloads images for a specific chapter of a manga and handles related metadata.
///
/// This asynchronous function performs the following tasks:
/// 1. **Setup and Logging**: Prepares the necessary directories, lock files, and logs the progress.
/// 2. **Image Retrieval**: Retrieves the list of images for the chapter and determines the appropriate image source based on the available metadata.
/// 3. **Download Process**: Downloads the images in parallel, using specified concurrency limits.
/// 4. **Metadata Handling**: Creates and writes metadata related to the chapter into a file.
/// 5. **Finalization**: Cleans up temporary files, updates global states, and resolves additional data as needed.
///
/// # Parameters
///
/// - `id: &str`
///   The unique identifier of the chapter to be downloaded.
/// - `obj: metadata::ChapterData`
///   Contains metadata about the chapter, including image base URL and image data.
/// - `manga_json: &metadata::ChapterResponse`
///   Contains additional attributes of the chapter, such as page count and update date.
/// - `title: &str`
///   The title of the chapter.
/// - `filename: &utils::FileName`
///   Contains the name and path information for the chapter's files.
/// - `update_date: &str`
///   The last updated date of the chapter.
/// - `scanlation: &metadata::ScanlationMetadata`
///   Information about the scanlation group responsible for the chapter.
///
/// # Returns
///
/// This function returns a `Result<(), error::MdownError>`. It returns:
/// - `Ok(())` if the chapter images and metadata are successfully downloaded and processed.
/// - `Err(error::MdownError)` if any errors occur during the process.
///
/// # Errors
///
/// Possible errors include:
/// - File I/O errors when creating or writing to lock and metadata files.
/// - Errors during the image download process.
/// - JSON serialization/deserialization errors when handling metadata.
/// - Any custom errors related to image handling or metadata creation.
///
/// # Notes
///
/// - The function utilizes asynchronous operations for downloading images in parallel.
/// - It includes detailed logging and debugging statements to track progress and errors.
/// - The download process respects concurrency limits and uses progress bars for user feedback.
/// - Temporary files and directories are managed carefully to ensure proper cleanup.
///
/// # Example
///
/// ```rust
/// let chapter_id = "12345";
/// let chapter_data = metadata::ChapterData { ... };
/// let manga_info = metadata::ChapterResponse { ... };
/// let chapter_title = "Chapter Title";
/// let file_info = utils::FileName { ... };
/// let last_update = "2024-08-31T00:00:00Z";
/// let scanlation_group = metadata::ScanlationMetadata { ... };
///
/// match download_chapter(
///     chapter_id,
///     chapter_data,
///     &manga_info,
///     chapter_title,
///     &file_info,
///     last_update,
///     &scanlation_group
/// ).await {
///     Ok(()) => println!("Chapter downloaded successfully."),
///     Err(err) => eprintln!("Error occurred: {}", err),
/// }
/// ```
///
pub(crate) async fn download_chapter(
    id: &str,
    obj: metadata::ChapterData,
    manga_json: &metadata::ChapterResponse,
    title: &str,
    filename: &utils::FileName,
    update_date: &str,
    scanlation: &metadata::ScanlationMetadata
) -> Result<(), error::MdownError> {
    let manga_name = &filename.manga_name;
    let vol = &filename.vol;
    let chapter = &filename.chapter_num;
    string(3, 0, &format!("  Downloading images in folder: {}:", filename.get_folder_name()));
    if
        *args::ARGS_WEB ||
        *args::ARGS_GUI ||
        *args::ARGS_CHECK ||
        *args::ARGS_UPDATE ||
        *args::ARGS_LOG
    {
        let mut current_chapter = resolute::CURRENT_CHAPTER.lock();
        current_chapter.clear();
        current_chapter.push_str(&filename.get_folder_name());
        drop(current_chapter);
        log!(&format!("Downloading images in folder: {}", filename.get_folder_name()));
    }
    let image_base_url = obj.baseUrl;
    let data_array = obj.chapter;
    let chapter_hash = data_array.hash;
    let saver = get_saver!();
    let mut images = match saver {
        metadata::Saver::data => data_array.data.clone(),
        metadata::Saver::dataSaver =>
            match data_array.dataSaver {
                Some(ref data) => data.clone(),
                None => Vec::new(),
            }
    };
    if images.is_empty() {
        images = match get_saver!(true) {
            metadata::Saver::data => data_array.data,
            metadata::Saver::dataSaver => data_array.dataSaver.unwrap_or_default(),
        };
    }
    let images_length = images.len();

    *resolute::CURRENT_PAGE.lock() = 0;
    *resolute::CURRENT_PAGE_MAX.lock() = images_length as u64;

    let lock_file = filename.get_lock();
    let mut lock_file_inst = match File::create(&lock_file) {
        Ok(file) => file,
        Err(err) => {
            return Err(error::MdownError::IoError(err, lock_file.clone(), 10106));
        }
    };
    match write!(lock_file_inst, "0") {
        Ok(()) => (),
        Err(err) => {
            eprintln!("Error: writing in chapter lock file {}", err);
        }
    }
    debug!("lock file created successfully");
    match fs::create_dir_all(filename.get_folder_w_end()) {
        Ok(()) => (),
        Err(err) => eprintln!("Error: creating directory {} {}", filename.get_folder_w_end(), err),
    }
    debug!("folder in cache created successfully");

    let mut metadata_file = match File::create(format!("{}_metadata", filename.get_folder_w_end())) {
        Ok(file) => file,
        Err(err) => {
            return Err(error::MdownError::IoError(err, lock_file.clone(), 10107));
        }
    };
    let attr = manga_json.attributes.clone();

    let pages = attr.pages.to_string();

    let response_map = metadata::ChapterMetadataIn {
        name: resolute::MANGA_NAME.lock().to_string(),
        id: id.to_string(),
        manga_id: resolute::MANGA_ID.lock().to_string(),
        saver: *resolute::SAVER.lock(),
        title: title.to_string(),
        pages,
        chapter: chapter.to_string(),
        volume: vol.to_string(),
        scanlation: scanlation.clone(),
    };

    let json = match serde_json::to_string_pretty(&response_map) {
        Ok(value) => value,
        Err(err) => {
            return Err(error::MdownError::JsonError(err.to_string(), 10108));
        }
    };
    match write!(metadata_file, "{}", json) {
        Ok(()) => (),
        Err(err) => {
            eprintln!("Error: writing in chapter metadata file {}", err);
        }
    }

    debug!("metadata file created successfully");

    let lock_file_wait = filename.get_folder_name();

    tokio::spawn(async move { utils::wait_for_end(&lock_file_wait, images_length).await });
    let start = if MAXPOINTS.max_x / 3 < (images_length as u32) / 2 {
        1
    } else {
        MAXPOINTS.max_x / 3 - (images_length as u32) / 2
    };

    let iter = args::ARGS.lock().max_consecutive.clone();

    let loop_for = ((images_length as f32) / (iter as f32)).ceil();

    let mut images_length_temp = images_length;

    for i in 0..loop_for as usize {
        let end_task;
        if images_length_temp > iter {
            end_task = (i + 1) * iter;
            images_length_temp -= iter;
        } else {
            end_task = images_length;
            images_length_temp = 0;
        }
        let start_task = i * iter;

        let pr_title = match !title.is_empty() {
            true => format!(" - {}", title),
            false => String::new(),
        };

        let tasks = (start_task..end_task).map(|item| {
            let image_temp = getter::get_attr_as_same_as_index(&images, item).to_string();
            let chapter_hash = Arc::from(chapter_hash.clone());
            let saver = Arc::from(match saver {
                metadata::Saver::data => "data",
                metadata::Saver::dataSaver => "data-saver",
            });
            let image = Arc::from(image_temp.trim_matches('"'));
            let image_base_url = Arc::from(image_base_url.clone());
            let page = item + 1;

            let folder_name = utils::process_filename(
                &format!("{} - {}Ch.{}{}", manga_name, vol, chapter, pr_title)
            );
            let file_name = utils::process_filename(
                &format!("{} - {}Ch.{}{} - {}.jpg", manga_name, vol, chapter, pr_title, page)
            );
            let file_name_brief = utils::process_filename(
                &format!("{}Ch.{} - {}.jpg", vol, chapter, page)
            );

            let full_path = format!(".cache/{}/{}", folder_name, file_name);

            tokio::spawn(async move {
                match
                    download::download_image(
                        image_base_url,
                        chapter_hash,
                        image,
                        page,
                        &folder_name,
                        &file_name_brief,
                        &full_path,
                        saver,
                        start
                    ).await
                {
                    Ok(()) => (),
                    Err(err) => {
                        handle_error!(&err, String::from("image"));
                    }
                };
            })
        });

        utils::progress_bar_preparation(start, images_length, 4);

        if *tutorial::TUTORIAL.lock() && *tutorial::TUTORIAL_CHAPTER.lock() {
            tutorial::images();
            *tutorial::TUTORIAL_CHAPTER.lock() = false;
        }

        futures::future::join_all(tasks).await;

        if *IS_END.lock() {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            *IS_END.lock() = false;
            return Ok(());
        }
    }

    let chapter_met = metadata::ChapterMetadata::new(chapter, update_date, id);
    resolute::CHAPTERS.lock().push(chapter_met);

    match resolute::resolve_dat() {
        Ok(()) => (),
        Err(err) => eprintln!("resolute::resolve_dat() in download_chapter() Error: {}", err),
    }
    match fs::remove_file(&lock_file) {
        Ok(()) => (),
        Err(_err) => (), // Removing .cache/NAME - CH.X.lock file will result in error
    }

    resolute::CURRENT_CHAPTER.lock().clear();
    *resolute::CURRENT_PAGE.lock() = 0;
    *resolute::CURRENT_PAGE_MAX.lock() = 0;
    *resolute::CURRENT_PERCENT.lock() = 0.0;
    *resolute::CURRENT_SIZE.lock() = 0.0;
    *resolute::CURRENT_SIZE_MAX.lock() = 0.0;

    Ok(())
}
