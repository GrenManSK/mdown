use serde_json::Value;
use std::{
    fs::{ self, File, OpenOptions },
    io::Write,
    sync::Arc,
    thread::sleep,
    time::{ Duration, Instant },
};

use crate::{
    args,
    debug,
    error::{ MdownError, suspend_error },
    getter,
    IS_END,
    log,
    MAXPOINTS,
    metadata,
    resolute::{ CURRENT_PAGE, MWD },
    string,
    utils,
};
/// Creates and configures a `reqwest::Client` for making HTTP requests.
///
/// This function sets up a `reqwest::Client` with a custom user-agent string. The client can be used to make
/// HTTP requests with the specified configuration.
///
/// # Returns
/// * `Result<reqwest::Client, reqwest::Error>` - Returns `Ok(reqwest::Client)` on success, or a `reqwest::Error` on failure.
///
/// # Errors
/// * Returns `reqwest::Error` if there is an issue building the HTTP client.
///
/// # Panics
/// * This function does not explicitly panic.
///
/// # Example
/// ```no_run
/// fn main() -> Result<(), reqwest::Error> {
///     let client = get_client()?;
///     // Use the client here to make requests
///     Ok(())
/// }
/// ```
pub(crate) fn get_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client
        ::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:122.0) Gecko/20100101 Firefox/122.0"
        )
        .build()
}

/// Sends an HTTP GET request to a constructed URL based on the provided parameters.
///
/// This asynchronous function builds a URL using the provided `base_url`, `c_hash`, `cover_hash`, and `mode` parameters.
/// It then performs an HTTP GET request to the constructed URL using a `reqwest::Client`. The function handles any errors
/// that occur during URL parsing or the HTTP request.
///
/// # Arguments
/// * `base_url` - An `Arc<str>` representing the base URL for the request.
/// * `c_hash` - An `Arc<str>` representing the hash of the content.
/// * `cover_hash` - An `Arc<str>` representing the cover hash.
/// * `mode` - A string slice that determines the path mode in the URL.
///
/// # Returns
/// * `Result<reqwest::Response, MdownError>` - Returns `Ok(reqwest::Response)` on success, or an `MdownError` on failure.
///
/// # Errors
/// * Returns `MdownError::NetworkError` if there is an issue with the HTTP request.
/// * Returns `MdownError::ConversionError` if there is an issue with URL parsing or joining.
///
/// # Panics
/// * This function does not explicitly panic.
///
/// # Example
/// ```no_run
/// async fn main() -> Result<(), MdownError> {
///     let response = get_response(
///         Arc::from("https://example.com"),
///         Arc::from("content_hash"),
///         Arc::from("cover_hash"),
///         "mode"
///     ).await?;
///     // Use the response here
///     Ok(())
/// }
/// ```
pub(crate) async fn get_response(
    base_url: Arc<str>,
    c_hash: Arc<str>,
    cover_hash: Arc<str>,
    mode: &str
) -> Result<reqwest::Response, MdownError> {
    let client = match get_client() {
        Ok(client) => client,
        Err(err) => {
            return Err(MdownError::NetworkError(err));
        }
    };
    let base_url = match url::Url::parse(base_url.as_ref()) {
        Ok(url) => url,
        Err(err) => {
            return Err(MdownError::ConversionError(err.to_string()));
        }
    };
    let url = format!("\\{}\\{}\\{}", mode, c_hash, cover_hash);

    let full_url = match base_url.join(&url) {
        Ok(url) => url,
        Err(err) => {
            return Err(MdownError::ConversionError(err.to_string()));
        }
    };

    debug!("sending request to: {}", full_url);

    match client.get(full_url).send().await {
        Ok(response) => { Ok(response) }
        Err(err) => { Err(MdownError::NetworkError(err)) }
    }
}

/// Retrieves the total size of the response content and its size in megabytes.
///
/// This function extracts the content length from the HTTP response and calculates its size in megabytes.
///
/// # Arguments
/// * `response` - A reference to a `reqwest::Response` from which the content size is retrieved.
///
/// # Returns
/// * `(u64, f32)` - A tuple where the first element is the total size in bytes, and the second element is the size in megabytes.
///
/// # Example
/// ```no_run
/// fn main() -> Result<(), MdownError> {
///     let response = reqwest::get("https://example.com/file").await?;
///     let (size_bytes, size_mb) = get_size(&response);
///     println!("Size: {} bytes ({} MB)", size_bytes, size_mb);
///     Ok(())
/// }
/// ```
pub(crate) fn get_size(response: &reqwest::Response) -> (u64, f32) {
    let total_size: u64 = response.content_length().unwrap_or_default();
    (total_size, (total_size as f32) / 1024.0 / 1024.0)
}

/// Formats a percentage value as a right-aligned string.
///
/// This function takes a percentage value and formats it to a three-digit string, right-aligned.
///
/// # Arguments
/// * `percentage` - The percentage value to format.
///
/// # Returns
/// * `String` - The formatted percentage string.
///
/// # Example
/// ```no_run
/// fn main() {
///     let perc = get_perc(75);
///     println!("Progress: {}%", perc);
/// }
/// ```
pub(crate) fn get_perc(percentage: i64) -> String {
    let mut buffer = itoa::Buffer::new();
    let perc = buffer.format(percentage);
    format!("{:>3}", perc)
}

/// Formats a floating-point number (`f32`) as a string with two decimal places.
///
/// This function takes a `f32` number and uses the `ryu` crate to convert it to a string.
/// It then formats the string to display two decimal places.
///
/// # Arguments
/// * `number` - A `f32` floating-point number to be formatted.
///
/// # Returns
/// * `String` - A string representing the formatted number with two decimal places.
///
/// # Example
/// ```
/// let formatted = get_float(3.14159);
/// assert_eq!(formatted, "3.14");
/// ```
///
/// # Panics
/// * This function does not explicitly panic.
pub(crate) fn get_float(number: f32) -> String {
    let mut buffer = ryu::Buffer::new();
    let perc = buffer.format(number);
    format!("{:.2}", perc)
}

/// Sends an HTTP GET request to the specified URL using a `reqwest::Client`.
///
/// This asynchronous function performs an HTTP GET request to the `full_url` using a `reqwest::Client`
/// and returns the response. It handles any errors related to the HTTP request.
///
/// # Arguments
/// * `full_url` - A string slice representing the full URL to which the GET request is made.
///
/// # Returns
/// * `Result<reqwest::Response, MdownError>` - Returns `Ok(reqwest::Response)` on success, or an `MdownError` on failure.
///
/// # Errors
/// * Returns `MdownError::NetworkError` if there is an issue with the HTTP request.
///
/// # Panics
/// * This function does not explicitly panic.
///
/// # Example
/// ```no_run
/// async fn main() -> Result<(), MdownError> {
///     let response = get_response_client("https://example.com/file").await?;
///     // Use the response here
///     Ok(())
/// }
/// ```
pub(crate) async fn get_response_client(full_url: &str) -> Result<reqwest::Response, MdownError> {
    let client = match get_client() {
        Ok(client) => client,
        Err(err) => {
            return Err(MdownError::NetworkError(err));
        }
    };

    match client.get(full_url).send().await {
        Ok(response) => Ok(response),
        Err(err) => { Err(MdownError::NetworkError(err)) }
    }
}

/// Downloads a cover image from a remote server and saves it to a specified folder.
///
/// This asynchronous function constructs a URL to fetch the cover image based on provided parameters.
/// It then downloads the image in chunks, updates a progress indicator, and saves it to a local file.
/// The function handles different types of logging and displays download progress based on command-line arguments.
///
/// # Arguments
/// * `image_base_url` - An `Arc<str>` representing the base URL for the cover image.
/// * `c_hash` - An `Arc<str>` representing the hash of the content.
/// * `cover_hash` - An `Arc<str>` representing the hash of the cover image.
/// * `folder` - An `Arc<str>` representing the directory where the cover image will be saved.
///
/// # Returns
/// * `Result<(), MdownError>` - Returns `Ok(())` if the download and save operations are successful, or an `MdownError` if any errors occur.
///
/// # Errors
/// * Returns `MdownError::IoError` if there is an issue creating or writing to the file.
/// * Returns `MdownError::NetworkError` if there is an issue with the HTTP request or response handling.
///
/// # Panics
/// * This function does not explicitly panic.
///
/// # Example
/// ```no_run
/// #[tokio::main]
/// async fn main() -> Result<(), MdownError> {
///     let image_base_url = Arc::from("https://example.com/images");
///     let c_hash = Arc::from("content_hash");
///     let cover_hash = Arc::from("cover_hash");
///     let folder = Arc::from("/path/to/folder");
///
///     download_cover(image_base_url, c_hash, cover_hash, folder).await?;
///     Ok(())
/// }
/// ```
pub(crate) async fn download_cover(
    image_base_url: Arc<str>,
    c_hash: Arc<str>,
    cover_hash: Arc<str>,
    folder: Arc<str>
) -> Result<(), MdownError> {
    // Log if any of the relevant command-line arguments are set
    if
        *args::ARGS_WEB ||
        *args::ARGS_GUI ||
        *args::ARGS_CHECK ||
        *args::ARGS_UPDATE ||
        *args::ARGS_LOG
    {
        log!("Downloading cover");
    }

    // Display initial progress message
    string(2, 0, "Downloading cover_art");

    // Fetch the cover image response
    let mut response = match get_response(image_base_url, c_hash, cover_hash, "covers").await {
        Ok(res) => res,
        Err(err) => {
            return Err(err);
        }
    };
    let (total_size, _) = get_size(&response);

    // Create or open the file to save the cover image
    let mut file = if args::ARGS.lock().update {
        match File::create("_cover.png") {
            Ok(file) => file,
            Err(err) => {
                return Err(MdownError::IoError(err, format!("{}\\_cover.png", MWD.lock())));
            }
        }
    } else {
        match File::create(format!("{}\\_cover.png", folder)) {
            Ok(file) => file,
            Err(err) => {
                return Err(MdownError::IoError(err, format!("{}\\_cover.png", folder)));
            }
        }
    };

    let interval = Duration::from_millis(250);
    let mut last_check_time = Instant::now();
    let mut downloaded = 0;

    // Download the image in chunks and update progress
    while
        // prettier-ignore or #[rustfmt::skip]
        let Some(chunk) = match response.chunk().await {
            Ok(size) => size,
            Err(err) => {
                return Err(MdownError::NetworkError(err));
            }
        }
    {
        match file.write_all(&chunk) {
            Ok(()) => (),
            Err(err) => {
                suspend_error(MdownError::IoError(err, format!("{}\\_cover.png", folder)));
            }
        }
        downloaded += chunk.len() as u64;
        let current_time = Instant::now();
        if current_time.duration_since(last_check_time) >= interval {
            last_check_time = current_time;
            let percentage = ((100.0 / (total_size as f32)) * (downloaded as f32)).round() as i64;
            let perc_string = get_perc(percentage);
            let message = format!("Downloading cover art {}%", perc_string);
            string(
                2,
                0,
                &format!(
                    "{} {}",
                    message,
                    "#".repeat(
                        ((((MAXPOINTS.max_x - (message.len() as u32)) as f32) /
                            (total_size as f32)) *
                            (downloaded as f32)) as usize
                    )
                )
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
        }
    }

    // Display final progress message
    let message = "Downloading cover art DONE";
    string(2, 0, &format!("{}{}", message, " ".repeat((MAXPOINTS.max_x as usize) - message.len())));
    if
        *args::ARGS_WEB ||
        *args::ARGS_GUI ||
        *args::ARGS_CHECK ||
        *args::ARGS_UPDATE ||
        *args::ARGS_LOG
    {
        log!(&message);
    }

    Ok(())
}

/// Fetches statistics for a given manga and saves them to a Markdown file.
///
/// This asynchronous function retrieves statistics data for a manga based on the provided `id` and `manga_name`.
/// It then processes the JSON data to extract various statistics and writes them to a Markdown file in a specified folder.
/// The function also updates a progress indicator and handles logging based on command-line arguments.
///
/// # Arguments
/// * `id` - A `&str` representing the unique identifier for the manga whose statistics are to be fetched.
/// * `manga_name` - A `&str` representing the name of the manga to be included in the Markdown file.
///
/// # Returns
/// * `Result<(), MdownError>` - Returns `Ok(())` if the operation is successful, or an `MdownError` if any errors occur.
///
/// # Errors
/// * Returns `MdownError::IoError` if there is an issue creating or writing to the file.
/// * Returns `MdownError::JsonError` if there is an issue parsing the JSON response or extracting data.
///
/// # Panics
/// * This function does not explicitly panic.
///
/// # Example
/// ```no_run
/// #[tokio::main]
/// async fn main() -> Result<(), MdownError> {
///     let manga_id = "12345";
///     let manga_name = "My Manga";
///
///     download_stat(manga_id, manga_name).await?;
///     Ok(())
/// }
/// ```
pub(crate) async fn download_stat(id: &str, manga_name: &str) -> Result<(), MdownError> {
    let folder = getter::get_folder_name();

    // Log the operation if any relevant command-line arguments are set
    if
        *args::ARGS_WEB ||
        *args::ARGS_GUI ||
        *args::ARGS_CHECK ||
        *args::ARGS_UPDATE ||
        *args::ARGS_LOG
    {
        log!("Getting statistics");
    }
    string(3, 0, "Getting statistics ...");

    // Retrieve the statistics JSON data
    let response = match getter::get_statistic_json(id).await {
        Ok(response) => response,
        Err(err) => {
            return Err(err);
        }
    };

    // Create or open the file for saving statistics
    let mut file = if args::ARGS.lock().update {
        match File::create("_statistics.md") {
            Ok(file) => file,
            Err(err) => {
                return Err(MdownError::IoError(err, format!("{}\\_statistics.md", MWD.lock())));
            }
        }
    } else {
        match File::create(format!("{}\\_statistics.md", folder)) {
            Ok(file) => file,
            Err(err) => {
                return Err(MdownError::IoError(err, format!("{}\\_statistics.md", folder)));
            }
        }
    };

    // Prepare the Markdown content
    let mut data = String::from(&("# ".to_owned() + manga_name + "\n\n"));

    // Parse and process the JSON response
    let json_value = match utils::get_json(&response) {
        Ok(value) => value,
        Err(err) => {
            suspend_error(MdownError::JsonError(err.to_string()));
            return Ok(());
        }
    };
    match json_value {
        Value::Object(obj) => {
            let statistics = match obj.get("statistics").and_then(|stat| stat.get(id)) {
                Some(stat) => stat,
                None => {
                    return Err(MdownError::JsonError(String::from("Didn't find statistics")));
                }
            };
            match serde_json::from_value::<metadata::Statistics>(statistics.clone()) {
                Ok(stat) => {
                    let comments = stat.comments;
                    let thread_id = comments.threadId;
                    let replies_count = comments.repliesCount;
                    let rating = stat.rating;
                    let average = rating.average;
                    let bayesian = rating.bayesian;
                    let distribution = rating.distribution;
                    let follows = stat.follows;

                    // Append statistics information to Markdown content
                    data += &format!("---\n\n## RATING\n\nRating: {}\n\n", average);
                    data += &format!("Bayesian: {}\n\n---\n\n", bayesian);
                    for i in 1..11 {
                        data += &get_dist(&distribution, i);
                    }
                    data += &format!("## Follows: {}\n\n", follows);
                    data += &format!(
                        "## Comments\n\nThread: <https://forums.mangadex.org/threads/{}>\n\nNumber of comments in thread: {}\n",
                        thread_id,
                        replies_count
                    );
                }
                Err(err) => {
                    suspend_error(MdownError::JsonError(err.to_string()));
                    return Ok(());
                }
            }
        }
        _ => {
            return Err(MdownError::JsonError(String::from("Could not parse statistics json")));
        }
    }

    // Write the Markdown content to the file
    match file.write_all(data.as_bytes()) {
        Ok(()) => (),
        Err(err) => {
            return Err(MdownError::IoError(err, format!("{}\\_statistics.md", folder)));
        }
    }

    // Display completion message
    string(3, 0, "Getting statistics DONE");
    Ok(())
}

/// Formats the rating distribution for a given rating.
///
/// This function retrieves the number of occurrences for a specified rating value from a `metadata::RatingDistribution`
/// and formats it as a string suitable for Markdown or text output.
///
/// # Arguments
/// * `distribution` - A reference to a `metadata::RatingDistribution` structure that holds the count of each rating value.
/// * `i` - A `usize` representing the rating value for which to retrieve the count. Must be between 1 and 10.
///
/// # Returns
/// * `String` - A formatted string showing the rating value and its corresponding count.
///
/// # Errors
/// * If `i` is outside the range of 1 to 10, the function returns an empty string.
///
/// # Example
/// ```rust
/// let distribution = metadata::RatingDistribution {
///     one: 5,
///     two: 10,
///     three: 15,
///     four: 20,
///     five: 25,
///     six: 30,
///     seven: 35,
///     eight: 40,
///     nine: 45,
///     ten: 50,
/// };
///
/// let result = get_dist(&distribution, 5);
/// println!("{}", result); // Output: "5: 25\n\n"
/// ```
///
fn get_dist(distribution: &metadata::RatingDistribution, i: usize) -> String {
    let value = match i {
        1 => distribution.one,
        2 => distribution.two,
        3 => distribution.three,
        4 => distribution.four,
        5 => distribution.five,
        6 => distribution.six,
        7 => distribution.seven,
        8 => distribution.eight,
        9 => distribution.nine,
        10 => distribution.ten,
        _ => {
            return String::new();
        }
    };
    format!("{}: {}\n\n", i, value)
}

/// Downloads an image from a specified URL and saves it to a given path.
///
/// This function handles downloading an image, tracking progress, and saving it to a local path. It also manages
/// caching and logging information based on various application modes.
///
/// # Arguments
/// * `image_base_url` - The base URL for the image, typically including the server address and endpoint.
/// * `c_hash` - A hash string used to identify the specific image or resource on the server.
/// * `f_name` - The file name or identifier for the image to download.
/// * `page` - The page number or index for the image being downloaded.
/// * `folder_name` - The name of the folder where the image will be saved.
/// * `file_name_brief` - A brief description of the file name for logging purposes.
/// * `full_path` - The full local path where the image will be saved.
/// * `saver` - A string identifier for the type of resource being downloaded.
/// * `start` - The starting position for logging or progress tracking.
///
/// # Returns
/// * `Result<(), MdownError>` - Returns `Ok(())` if the download completes successfully, or an error of type `MdownError` if something goes wrong.
///
/// # Errors
/// * `MdownError::NetworkError` - If there is an issue with the network request to get the image.
/// * `MdownError::IoError` - If there is an issue with file operations or cache management.
/// * `MdownError::JsonError` - If there's an issue with JSON parsing, though this is not directly applicable here.
///
/// # Example
/// ```rust
/// use std::sync::Arc;
/// use std::fs::File;
/// use std::io::prelude::*;
/// use std::time::Duration;
/// use tokio::time::sleep;
///
/// let image_base_url = Arc::from("https://example.com/images");
/// let c_hash = Arc::from("abc123");
/// let f_name = Arc::from("image.png");
/// let folder_name = "images";
/// let file_name_brief = "image";
/// let full_path = "path/to/save/image.png";
/// let saver = Arc::from("saver_id");
/// let start = 0;
///
/// // Call the function (in an async context)
/// tokio::spawn(async move {
///     if let Err(e) = download_image(image_base_url, c_hash, f_name, 1, folder_name, file_name_brief, full_path, saver, start).await {
///         eprintln!("Failed to download image: {:?}", e);
///     }
/// });
/// ```
///
/// # Notes
/// * **Progress Tracking:** The function updates progress on the console or logs it based on the application's mode.
/// * **Caching:** Lock files are used to manage concurrent downloads and cache metadata.
pub(crate) async fn download_image(
    image_base_url: Arc<str>,
    c_hash: Arc<str>,
    f_name: Arc<str>,
    page: usize,
    folder_name: &str,
    file_name_brief: &str,
    full_path: &str,
    saver: Arc<str>,
    start: u32
) -> Result<(), MdownError> {
    let page_str = page.to_string() + &" ".repeat(3 - page.to_string().len());
    let lock_file = format!(".cache\\{}.lock", folder_name);
    if
        *args::ARGS_WEB ||
        *args::ARGS_GUI ||
        *args::ARGS_CHECK ||
        *args::ARGS_UPDATE ||
        *args::ARGS_LOG
    {
        log!(&format!("Starting image download {}", page));
    }

    let download = page + 3 + 1 < (MAXPOINTS.max_y as usize);

    string(3 + 1, start + (page as u32) - 1, "|");
    if download {
        string(
            3 + 1 + (page as u32),
            0,
            &format!("   {} Downloading {}", page_str, file_name_brief)
        );
    }
    string(3 + 1, start + (page as u32) - 1, "/");

    let mut response = match get_response(image_base_url, c_hash, f_name, &saver).await {
        Ok(res) => res,
        Err(err) => {
            return Err(err);
        }
    };

    let (total_size, final_size) = get_size(&response);

    string(3 + 1, start + (page as u32) - 1, "\\");
    let mut file = match File::create(full_path) {
        Ok(file) => file,
        Err(err) => {
            return Err(MdownError::IoError(err, full_path.to_string()));
        }
    };

    let (mut downloaded, mut last_size) = (0, 0.0);
    let interval = Duration::from_millis(100);
    let mut last_check_time = Instant::now();

    while fs::metadata(format!(".cache\\{}.lock", lock_file)).is_ok() {
        sleep(Duration::from_millis(10));
    }
    let mut lock_file_inst = match
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!(".cache\\{}_{}_final.lock", folder_name, page))
    {
        Ok(lock_file) => lock_file,
        Err(err) => {
            return Err(
                MdownError::IoError(err, format!(".cache\\{}_{}_final.lock", folder_name, page))
            );
        }
    };
    match write!(lock_file_inst, "{:.2}", total_size) {
        Ok(()) => (),
        Err(err) => {
            suspend_error(
                MdownError::IoError(err, format!(".cache\\{}_{}_final.lock", folder_name, page))
            );
        }
    }

    while
        // prettier-ignore
        let Some(chunk) = match response.chunk().await {
            Ok(Some(chunk)) => Some(chunk),
            Ok(None) => None,
            Err(err) => {
                return Err(MdownError::NetworkError(err));
            }
        }
    {
        if *IS_END.lock() {
            return Ok(());
        }
        match file.write_all(&chunk) {
            Ok(()) => (),
            Err(err) => {
                suspend_error(MdownError::IoError(err, full_path.to_string()));
            }
        }
        downloaded += chunk.len() as u64;
        let current_time = Instant::now();
        if current_time.duration_since(last_check_time) >= interval {
            if (downloaded as f32) != last_size {
                let mut lock_file = match
                    OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .open(format!(".cache\\{}_{}.lock", folder_name, page))
                {
                    Ok(file) => file,
                    Err(err) => {
                        return Err(
                            MdownError::IoError(
                                err,
                                format!(".cache\\{}_{}.lock", folder_name, page)
                            )
                        );
                    }
                };
                match
                    lock_file.write(format!("{}", (downloaded as f64) / 1024.0 / 1024.0).as_bytes())
                {
                    Ok(_size) => (),
                    Err(err) => {
                        suspend_error(
                            MdownError::IoError(
                                err,
                                format!(".cache\\{}_{}.lock", folder_name, page)
                            )
                        );
                    }
                }
            }
            last_check_time = current_time;
            let percentage = ((100.0 / (total_size as f32)) * (downloaded as f32)).round() as i64;
            let perc_string = get_perc(percentage);
            let current_mb = get_float((downloaded as f32) / 1024.0 / 1024.0);
            let current_mbs = get_float(
                (((downloaded as f32) - last_size) * 10.0) / 1024.0 / 1024.0
            );
            let message = format!(
                "   {} Downloading {} {}% - {:.2}mb of {:.2}mb [{:.2}mb/s]",
                page_str,
                file_name_brief,
                perc_string,
                current_mb,
                final_size,
                current_mbs
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
            if
                !*args::ARGS_WEB &&
                !*args::ARGS_GUI &&
                !*args::ARGS_CHECK &&
                !*args::ARGS_UPDATE &&
                download
            {
                string(
                    3 + 1 + (page as u32),
                    0,
                    &format!(
                        "{} {}",
                        message,
                        "#".repeat(
                            ((((MAXPOINTS.max_x - (message.len() as u32)) as f32) /
                                (total_size as f32)) *
                                (downloaded as f32)) as usize
                        )
                    )
                );
            }
            last_size = downloaded as f32;
        }
    }

    *CURRENT_PAGE.lock() += 1;

    if !*args::ARGS_WEB && !*args::ARGS_GUI && !*args::ARGS_CHECK && !*args::ARGS_UPDATE {
        if download {
            let current_mb = get_float((downloaded as f32) / 1024.0 / 1024.0);
            let max_mb = get_float((total_size as f32) / 1024.0 / 1024.0);
            let message = format!(
                "   {} Downloading {} {}% - {:.2}mb of {:.2}mb",
                page_str,
                file_name_brief,
                100,
                current_mb,
                max_mb
            );
            string(
                3 + 1 + (page as u32),
                0,
                &format!(
                    "{} {}",
                    message,
                    "#".repeat(
                        ((((MAXPOINTS.max_x - (message.len() as u32)) as f32) /
                            (total_size as f32)) *
                            (downloaded as f32)) as usize
                    )
                )
            );
        }
        string(3 + 1, start + (page as u32) - 1, "#");
    }
    let mut lock_file = match
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!(".cache\\{}_{}.lock", folder_name, page))
    {
        Ok(file) => file,
        Err(err) => {
            return Err(MdownError::IoError(err, format!(".cache\\{}_{}.lock", folder_name, page)));
        }
    };
    match lock_file.write(format!("{}", (downloaded as f64) / 1024.0 / 1024.0).as_bytes()) {
        Ok(_size) => (),
        Err(err) => {
            suspend_error(
                MdownError::IoError(err, format!(".cache\\{}_{}.lock", folder_name, page))
            );
        }
    }

    if
        *args::ARGS_WEB ||
        *args::ARGS_GUI ||
        *args::ARGS_CHECK ||
        *args::ARGS_UPDATE ||
        *args::ARGS_LOG
    {
        log!(&format!("Finished image download {}", page));
    }

    if *args::ARGS_GUI {
        match fs::create_dir_all(".cache\\preview") {
            Ok(()) => (),
            Err(err) => {
                return Err(MdownError::IoError(err, format!(".cache\\preview\\{}", full_path)));
            }
        }
        let target_file = std::path::Path::new(".cache\\preview").join("preview.png");

        if target_file.exists() {
            match fs::remove_file(&target_file) {
                Ok(()) => (),
                Err(err) => {
                    return Err(MdownError::IoError(err, format!(".cache\\preview\\{}", full_path)));
                }
            };
        }
        match fs::copy(full_path, target_file) {
            Ok(_) => (),
            Err(err) => {
                return Err(MdownError::IoError(err, format!(".cache\\preview\\{}", full_path)));
            }
        };
    }
    Ok(())
}

// Returns a valid response object when given a valid URL
#[tokio::test]
async fn test_get_response_client_valid_url() {
    let url = "https://example.com";
    let response = get_response_client(url).await;
    assert!(response.is_ok());
}

// Returns an error when given an invalid URL
#[tokio::test]
async fn test_get_response_client_invalid_url() {
    let url = "invalid_url";
    let response = get_response_client(url).await;
    assert!(response.is_err());
}

// Returns an error when given an empty URL
#[tokio::test]
async fn test_get_response_client_empty_url() {
    let url = "";
    let response = get_response_client(url).await;
    assert!(response.is_err());
}
