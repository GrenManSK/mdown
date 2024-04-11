use serde_json::Value;
use std::{
    fs::{ self, File, OpenOptions },
    io::Write,
    sync::Arc,
    thread::sleep,
    time::{ Duration, Instant },
};
use tracing::info;

use crate::{
    error::mdown::Error,
    getter,
    resolute::{ self, CURRENT_PAGE },
    string,
    utils::{ self, clear_screen, process_filename },
    ARGS,
    IS_END,
    MAXPOINTS,
};

pub(crate) fn get_client() -> Result<reqwest::Client, reqwest::Error> {
    match
        reqwest::Client
            ::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:122.0) Gecko/20100101 Firefox/122.0"
            )
            .build()
    {
        Ok(response) => Ok(response),
        Err(err) => {
            return Err(err);
        }
    }
}

pub(crate) async fn get_response(
    base_url: Arc<str>,
    c_hash: Arc<str>,
    cover_hash: Arc<str>,
    mode: &str
) -> Result<reqwest::Response, Error> {
    let client = match get_client() {
        Ok(client) => client,
        Err(err) => {
            return Err(Error::NetworkError(err));
        }
    };
    let base_url = match url::Url::parse(&base_url.to_string()) {
        Ok(url) => url,
        Err(err) => {
            return Err(Error::ConversionError(err.to_string()));
        }
    };
    let url = format!("\\{}\\{}\\{}", mode, c_hash, cover_hash);

    let full_url = match base_url.join(&url) {
        Ok(url) => url,
        Err(err) => {
            return Err(Error::ConversionError(err.to_string()));
        }
    };

    match client.get(full_url).send().await {
        Ok(response) => {
            return Ok(response);
        }
        Err(err) => {
            return Err(Error::NetworkError(err));
        }
    }
}
pub(crate) fn get_size(response: &reqwest::Response) -> (u64, f32) {
    let total_size: u64 = match response.content_length() {
        Some(value) => value,
        None => 0,
    };
    (total_size, (total_size as f32) / (1024 as f32) / (1024 as f32))
}

pub(crate) fn get_perc(percentage: i64) -> String {
    format!("{:>3}", percentage)
}

pub(crate) async fn get_response_client(full_url: &str) -> Result<reqwest::Response, Error> {
    let client = match get_client() {
        Ok(client) => client,
        Err(err) => {
            return Err(Error::NetworkError(err));
        }
    };

    match client.get(full_url).send().await {
        Ok(response) => Ok(response),
        Err(err) => {
            return Err(Error::NetworkError(err));
        }
    }
}

pub(crate) async fn download_cover(
    image_base_url: Arc<str>,
    c_hash: Arc<str>,
    cover_hash: Arc<str>,
    folder: Arc<str>,
    handle_id: Option<Box<str>>
) -> Result<(), Error> {
    let handle_id = match handle_id {
        Some(id) => id,
        None => String::from("0").into_boxed_str(),
    };
    if ARGS.web || ARGS.gui || ARGS.check || ARGS.update || ARGS.log {
        info!("@{}  Downloading cover", handle_id);
    }
    string(1, 0, "Downloading cover_art");

    let mut response = match get_response(image_base_url, c_hash, cover_hash, "covers").await {
        Ok(res) => res,
        Err(err) => {
            return Err(err);
        }
    };
    let (total_size, _) = get_size(&response);

    let mut file = match File::create(format!("{}\\_cover.png", folder)) {
        Ok(file) => file,
        Err(err) => {
            return Err(Error::IoError(err, Some(format!("{}\\_cover.png", folder))));
        }
    };

    let interval = Duration::from_millis(250);
    let mut last_check_time = Instant::now();
    let mut downloaded = 0;

    while
        // prettier-ignore or #[rustfmt::skip]
        let Some(chunk) = match response.chunk().await {
            Ok(size) => size,
            Err(err) => {
                return Err(Error::NetworkError(err));
            }
        }
    {
        match file.write_all(&chunk) {
            Ok(()) => (),
            Err(err) => {
                (
                    match resolute::SUSPENDED.lock() {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(Error::PoisonError(err.to_string()));
                        }
                    }
                ).push(Error::IoError(err, Some(format!("{}\\_cover.png", folder))));
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
                1,
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
            if ARGS.web || ARGS.gui || ARGS.check || ARGS.update || ARGS.log {
                info!("@{}  {}", handle_id, message);
            }
        }
    }
    clear_screen(1);
    Ok(())
}
pub(crate) async fn download_stat(
    id: &str,
    folder: &str,
    manga_name: &str,
    handle_id: Option<Box<str>>
) -> Result<(), Error> {
    let handle_id = match handle_id {
        Some(id) => id,
        None => String::from("0").into_boxed_str(),
    };
    if ARGS.web || ARGS.gui || ARGS.check || ARGS.update || ARGS.log {
        info!("@{}  Getting statistics", handle_id);
    }
    string(1, 0, "Getting statistics");

    let response = match getter::get_statistic_json(id).await {
        Ok(response) => response,
        Err(err) => {
            return Err(err);
        }
    };

    let mut file = match File::create(format!("{}\\_statistics.md", folder)) {
        Ok(file) => file,
        Err(err) => {
            return Err(Error::IoError(err, Some(format!("{}\\_statistics.md", folder))));
        }
    };

    let mut data = String::from(&("# ".to_owned() + manga_name + "\n\n"));

    let json_value = match utils::get_json(&response) {
        Ok(value) => value,
        Err(err) => {
            (
                match resolute::SUSPENDED.lock() {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(Error::PoisonError(err.to_string()));
                    }
                }
            ).push(Error::JsonError(err.to_string()));
            return Ok(());
        }
    };
    match json_value {
        Value::Object(obj) => {
            let statistics = match obj.get("statistics").and_then(|stat| stat.get(id)) {
                Some(stat) => stat,
                None => {
                    return Err(Error::JsonError(String::from("Didn't find statistics")));
                }
            };
            let comments = match statistics.get("comments") {
                Some(comm) => comm,
                None => {
                    return Err(Error::JsonError(String::from("Didn't find comments")));
                }
            };
            let thread_id = match comments.get("threadId").and_then(Value::as_i64) {
                Some(id) => id,
                None => -1,
            };
            let replies_count = match comments.get("repliesCount").and_then(Value::as_i64) {
                Some(id) => id,
                None => -1,
            };
            let rating = match statistics.get("rating") {
                Some(rating) => rating,
                None => {
                    return Err(Error::JsonError(String::from("Didn't find rating")));
                }
            };
            let average = match rating.get("average").and_then(Value::as_f64) {
                Some(id) => id,
                None => -1.0,
            };
            let bayesian = match rating.get("bayesian").and_then(Value::as_f64) {
                Some(id) => id,
                None => -1.0,
            };
            let distribution = match rating.get("distribution") {
                Some(dist) => dist,
                None => {
                    return Err(Error::JsonError(String::from("Didn't find distribution")));
                }
            };
            let follows = match statistics.get("follows").and_then(Value::as_i64) {
                Some(id) => id,
                None => -1,
            };

            data += &format!("---\n\n## RATING\n\nRating: {}\n\n", average);
            data += &format!("Bayesian: {}\n\n---\n\n", bayesian);
            for i in 1..11 {
                data += &get_dist(distribution.clone(), i);
            }
            data += &format!("## Follows: {}\n\n", follows);
            data += &format!(
                "## Comments\n\nThread: <https://forums.mangadex.org/threads/{}>\n\nNumber of comments in thread: {}\n",
                thread_id,
                replies_count
            );
        }
        _ => todo!(),
    }

    match file.write_all(data.as_bytes()) {
        Ok(()) => (),
        Err(err) => {
            return Err(Error::IoError(err, Some(format!("{}\\_statistics.md", folder))));
        }
    }

    Ok(())
}

fn get_dist(distribution: Value, i: usize) -> String {
    let value = match distribution.get(i.to_string()).and_then(Value::as_i64) {
        Some(value) => value,
        _ => -1,
    };
    format!("{}: {}\n\n", i, value)
}

pub(crate) async fn download_image(
    image_base_url: Arc<str>,
    c_hash: Arc<str>,
    f_name: Arc<str>,
    manga_name: Arc<str>,
    name: Arc<str>,
    vol: Arc<str>,
    chapter: Arc<str>,
    page: usize,
    start: u32,
    iter: usize,
    times: usize,
    handle_id: Box<str>
) -> Result<(), Error> {
    let mut pr_title = "".to_string();
    if name != "".into() {
        pr_title = format!(" - {}", name);
    }
    let page = page + 1;
    if ARGS.web || ARGS.gui || ARGS.check || ARGS.update || ARGS.log {
        info!("@{} Starting image download {}", handle_id, page);
    }
    let page_str = page.to_string() + &" ".repeat(3 - page.to_string().len());
    let folder_name = process_filename(
        &format!("{} - {}Ch.{}{}", manga_name, vol, chapter, pr_title)
    );
    let file_name = process_filename(
        &format!("{} - {}Ch.{}{} - {}.jpg", manga_name, vol, chapter, pr_title, page)
    );
    let file_name_brief = process_filename(&format!("{}Ch.{} - {}.jpg", vol, chapter, page));

    let lock_file = process_filename(&format!(".cache\\{}.lock", folder_name));

    string(3 + 1, start + (page as u32) - 1, "|");
    string(3 + 1 + (page as u32), 0, "   Sleeping");
    sleep(Duration::from_millis(((page - iter * times) * 50) as u64));
    string(3 + 1 + (page as u32), 0, &format!("   {} Downloading {}", page_str, file_name_brief));
    string(3 + 1, start + (page as u32) - 1, "/");
    let full_path = format!(".cache/{}/{}", folder_name, file_name);

    let saver = match resolute::SAVER.lock() {
        Ok(saver) =>
            match *saver {
                true => "data-saver",
                false => "data",
            }
        Err(err) => {
            return Err(Error::PoisonError(err.to_string()));
        }
    };
    let mut response = match get_response(image_base_url, c_hash, f_name, saver).await {
        Ok(res) => res,
        Err(err) => {
            return Err(err);
        }
    };

    let (total_size, final_size) = get_size(&response);

    string(3 + 1, start + (page as u32) - 1, "\\");
    let mut file = match File::create(full_path.clone()) {
        Ok(file) => file,
        Err(err) => {
            return Err(Error::IoError(err, Some(full_path.clone())));
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
                Error::IoError(err, Some(format!(".cache\\{}_{}_final.lock", folder_name, page)))
            );
        }
    };
    match write!(lock_file_inst, "{:.2}", total_size) {
        Ok(()) => (),
        Err(err) => {
            (
                match resolute::SUSPENDED.lock() {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(Error::PoisonError(err.to_string()));
                    }
                }
            ).push(
                Error::IoError(err, Some(format!(".cache\\{}_{}_final.lock", folder_name, page)))
            );
        }
    }

    while
        //prettier-ignore
        let Some(chunk) = match response.chunk().await {
            Ok(Some(chunk)) => Some(chunk),
            Ok(None) => None,
            Err(err) => {
                return Err(Error::NetworkError(err));
            }
        }
    {
        // prettier-ignore
        if match IS_END.lock() {
            Ok(value) => *value,
            Err(err) => {
                return Err(Error::PoisonError(err.to_string()));
            }
        }
        {
            return Ok(());
        }
        match file.write_all(&chunk) {
            Ok(()) => (),
            Err(err) => {
                (
                    match resolute::SUSPENDED.lock() {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(Error::PoisonError(err.to_string()));
                        }
                    }
                ).push(Error::IoError(err, Some(full_path.clone())));
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
                            Error::IoError(
                                err,
                                Some(format!(".cache\\{}_{}.lock", folder_name, page))
                            )
                        );
                    }
                };
                match
                    lock_file.write(format!("{}", (downloaded as f64) / 1024.0 / 1024.0).as_bytes())
                {
                    Ok(_size) => (),
                    Err(err) => {
                        (
                            match resolute::SUSPENDED.lock() {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(Error::PoisonError(err.to_string()));
                                }
                            }
                        ).push(
                            Error::IoError(
                                err,
                                Some(format!(".cache\\{}_{}.lock", folder_name, page))
                            )
                        );
                    }
                }
            }
            last_check_time = current_time;
            let percentage = ((100.0 / (total_size as f32)) * (downloaded as f32)).round() as i64;
            let perc_string = get_perc(percentage);
            let message = format!(
                "   {} Downloading {} {}% - {:.2}mb of {:.2}mb [{:.2}mb/s]",
                page_str,
                file_name_brief,
                perc_string,
                (downloaded as f32) / (1024 as f32) / (1024 as f32),
                final_size,
                (((downloaded as f32) - last_size) * 4.0) / (1024 as f32) / (1024 as f32)
            );
            if ARGS.web || ARGS.gui || ARGS.check || ARGS.update || ARGS.log {
                info!("@{} {}", handle_id, message.to_string());
            }
            if !ARGS.web && !ARGS.gui && !ARGS.check && !ARGS.update {
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

    *(match CURRENT_PAGE.lock() {
        Ok(value) => value,
        Err(err) => {
            return Err(Error::PoisonError(err.to_string()));
        }
    }) += 1;

    if !ARGS.web && !ARGS.gui && !ARGS.check && !ARGS.update {
        let message = format!(
            "   {} Downloading {} {}% - {:.2}mb of {:.2}mb",
            page_str,
            file_name_brief,
            100,
            (downloaded as f32) / (1024 as f32) / (1024 as f32),
            (total_size as f32) / (1024 as f32) / (1024 as f32)
        );
        string(
            3 + 1 + (page as u32),
            0,
            &format!(
                "{} {}",
                message,
                "#".repeat(
                    ((((MAXPOINTS.max_x - (message.len() as u32)) as f32) / (total_size as f32)) *
                        (downloaded as f32)) as usize
                )
            )
        );
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
            return Err(Error::IoError(err, Some(format!(".cache\\{}_{}.lock", folder_name, page))));
        }
    };
    match lock_file.write(format!("{}", (downloaded as f64) / 1024.0 / 1024.0).as_bytes()) {
        Ok(_size) => (),
        Err(err) => {
            (
                match resolute::SUSPENDED.lock() {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(Error::PoisonError(err.to_string()));
                    }
                }
            ).push(Error::IoError(err, Some(format!(".cache\\{}_{}.lock", folder_name, page))));
        }
    }

    if ARGS.web || ARGS.gui || ARGS.check || ARGS.update || ARGS.log {
        info!("@{} Finished image download {}", handle_id, page);
    }
    Ok(())
}

// Returns a valid response object when given a valid URL
#[tokio::test]
async fn test_get_response_client_valid_url() {
    let url = "https://example.com";
    let response = get_response_client(&url.to_string()).await;
    assert!(response.is_ok());
}

// Returns an error when given an invalid URL
#[tokio::test]
async fn test_get_response_client_invalid_url() {
    let url = "invalid_url";
    let response = get_response_client(&url.to_string()).await;
    assert!(response.is_err());
}

// Returns an error when given an empty URL
#[tokio::test]
async fn test_get_response_client_empty_url() {
    let url = "";
    let response = get_response_client(&url.to_string()).await;
    assert!(response.is_err());
}
