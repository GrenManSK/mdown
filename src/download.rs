use std::{ time::{ Duration, Instant }, fs::{ self, File, OpenOptions }, thread::sleep, io::Write };
use serde_json::{ self, Value };
use tracing::info;

use crate::{
    string,
    ARGS,
    MAXPOINTS,
    utils::{ process_filename, clear_screen },
    IS_END,
    getter,
    resolute::CURRENT_PAGE,
};

pub(crate) async fn get_response(c_hash: &str, cover_hash: &str, mode: &str) -> reqwest::Response {
    reqwest
        ::get(format!("https://uploads.mangadex.org\\{}\\{}\\{}", mode, c_hash, cover_hash)).await
        .unwrap()
}
pub(crate) fn get_size(response: &reqwest::Response) -> (u64, f32) {
    let total_size: u64 = response.content_length().unwrap_or(0);
    (total_size, (total_size as f32) / (1024 as f32) / (1024 as f32))
}

pub(crate) fn get_perc(percentage: i64) -> String {
    format!("{:>3}", percentage)
}

pub(crate) async fn get_response_client(
    full_url: String
) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client
        ::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/116.0"
        )
        .build()
        .unwrap();

    client.get(&full_url).send().await
}

pub(crate) async fn download_cover(
    c_hash: &str,
    cover_hash: &str,
    folder: &str,
    handle_id: Option<String>
) {
    let handle_id = handle_id.unwrap_or_default();
    if ARGS.web {
        info!("@{}  Downloading cover", handle_id);
    }
    string(1, 0, "Downloading cover_art");

    let mut response = get_response(c_hash, cover_hash, "covers").await;
    let (total_size, _) = get_size(&response);

    let mut file = File::create(format!("{}\\_cover.png", folder)).unwrap();

    let interval = Duration::from_millis(250);
    let mut last_check_time = Instant::now();
    let mut downloaded = 0;

    while let Some(chunk) = response.chunk().await.unwrap() {
        let _ = file.write_all(&chunk);
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
                        ((((MAXPOINTS.max_x - (message.len() as i32)) as f32) /
                            (total_size as f32)) *
                            (downloaded as f32)) as usize
                    )
                )
            );
            if ARGS.web {
                info!("@{}  {}", handle_id, message);
            }
        }
    }
    clear_screen(1);
}
pub(crate) async fn download_stat(
    id: &str,
    folder: &str,
    manga_name: &str,
    handle_id: Option<String>
) {
    let handle_id = handle_id.unwrap_or_default();
    if ARGS.web {
        info!("@{}  Getting statistics", handle_id);
    }
    string(1, 0, "Getting statistics");

    let response = getter::get_statistic_json(id).await.unwrap();

    let mut file = File::create(format!("{}\\_statistics.md", folder)).unwrap();

    let mut data = String::from(&("# ".to_owned() + manga_name + "\n\n"));

    match serde_json::from_str(&response) {
        Ok(json_value) =>
            match json_value {
                Value::Object(obj) => {
                    let statistics = obj
                        .get("statistics")
                        .and_then(|stat| stat.get(id))
                        .unwrap();
                    let comments = statistics.get("comments").unwrap();
                    let thread_id = comments.get("threadId").and_then(Value::as_u64).unwrap();
                    let replies_count = comments
                        .get("repliesCount")
                        .and_then(Value::as_u64)
                        .unwrap();
                    let rating = statistics.get("rating").unwrap();
                    let average = rating.get("average").and_then(Value::as_f64).unwrap();
                    let bayesian = rating.get("bayesian").and_then(Value::as_f64).unwrap();
                    let distribution = rating.get("distribution").unwrap();
                    let follows = statistics.get("follows").and_then(Value::as_u64).unwrap();

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
        Err(err) => eprintln!("  Error parsing JSON: {}", err),
    }

    file.write_all(data.as_bytes()).unwrap();
}

fn get_dist(distribution: Value, i: usize) -> String {
    let value = distribution.get(i.to_string()).and_then(Value::as_u64).unwrap();
    format!("{}: {}\n\n", i, value)
}

pub(crate) async fn download_image(
    c_hash: &str,
    f_name: &str,
    manga_name: &str,
    name: &str,
    vol: &str,
    chapter: &str,
    page: usize,
    start: i32,
    iter: usize,
    times: usize,
    handle_id: String
) {
    let mut pr_title = "".to_string();
    if name != "" {
        pr_title = format!(" - {}", name);
    }
    let page = page + 1;
    if ARGS.web {
        info!("@{} Starting image download {}", handle_id, page);
    }
    let page_str = page.to_string() + &" ".repeat(3 - page.to_string().len());
    let folder_name = process_filename(
        format!("{} - {}Ch.{}{}", manga_name, vol, chapter, pr_title)
    );
    let file_name = process_filename(
        format!("{} - {}Ch.{}{} - {}.jpg", manga_name, vol, chapter, pr_title, page)
    );
    let file_name_brief = process_filename(format!("{}Ch.{} - {}.jpg", vol, chapter, page));

    let lock_file = process_filename(format!(".cache\\{}.lock", folder_name));

    string(5 + 1, -1 + start + (page as i32), "|");
    string(5 + 1 + (page as i32), 0, "   Sleeping");
    sleep(Duration::from_millis(((page - iter * times) * 50) as u64));
    string(5 + 1 + (page as i32), 0, &format!("   {} Downloading {}", page_str, file_name_brief));
    string(5 + 1, -1 + start + (page as i32), "/");
    let full_path = format!(".cache/{}/{}", folder_name, file_name);

    let mut response;
    if ARGS.saver {
        response = get_response(c_hash, f_name, "data-saver").await;
    } else {
        response = get_response(c_hash, f_name, "data").await;
    }
    let (total_size, final_size) = get_size(&response);

    string(5 + 1, -1 + start + (page as i32), "\\");
    let mut file = File::create(full_path).unwrap();

    let (mut downloaded, mut last_size) = (0, 0.0);
    let interval = Duration::from_millis(250);
    let mut last_check_time = Instant::now();

    while fs::metadata(format!(".cache\\{}.lock", lock_file)).is_ok() {
        sleep(Duration::from_millis(10));
    }
    let mut lock_file_inst = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(format!(".cache\\{}_{}_final.lock", folder_name, page))
        .unwrap();
    let _ = write!(lock_file_inst, "{:.2}", total_size);

    while let Some(chunk) = response.chunk().await.unwrap() {
        if *IS_END.lock().unwrap() || false {
            return;
        }
        let _ = file.write_all(&chunk);
        downloaded += chunk.len() as u64;
        let current_time = Instant::now();
        if current_time.duration_since(last_check_time) >= interval {
            if (downloaded as f32) != last_size {
                let mut lock_file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(format!(".cache\\{}_{}.lock", folder_name, page))
                    .unwrap();
                let _ = lock_file.write(format!("{}", downloaded / 1024 / 1024).as_bytes());
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
            if ARGS.web {
                info!("@{} {}", handle_id, message.to_string());
            }
            string(
                5 + 1 + (page as i32),
                0,
                &format!(
                    "{} {}",
                    message,
                    "#".repeat(
                        ((((MAXPOINTS.max_x - (message.len() as i32)) as f32) /
                            (total_size as f32)) *
                            (downloaded as f32)) as usize
                    )
                )
            );
            last_size = downloaded as f32;
        }
    }

    *CURRENT_PAGE.lock().unwrap() += 1;

    let message = format!(
        "   {} Downloading {} {}% - {:.2}mb of {:.2}mb",
        page_str,
        file_name_brief,
        100,
        (downloaded as f32) / (1024 as f32) / (1024 as f32),
        (total_size as f32) / (1024 as f32) / (1024 as f32)
    );

    string(
        5 + 1 + (page as i32),
        0,
        &format!(
            "{} {}",
            message,
            "#".repeat(
                ((((MAXPOINTS.max_x - (message.len() as i32)) as f32) / (total_size as f32)) *
                    (downloaded as f32)) as usize
            )
        )
    );
    string(5 + 1, -1 + start + (page as i32), "#");
    let mut lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(format!(".cache\\{}_{}.lock", folder_name, page))
        .unwrap();
    let _ = lock_file.write(format!("{}", (downloaded as f64) / 1024.0 / 1024.0).as_bytes());

    if ARGS.web {
        info!("@{} Finished image download {}", handle_id, page);
    }
}

// Returns a valid response object when given a valid URL
#[tokio::test]
async fn test_get_response_client_valid_url() {
    let url = "https://example.com";
    let response = get_response_client(url.to_string()).await;
    assert!(response.is_ok());
}

// Returns an error when given an invalid URL
#[tokio::test]
async fn test_get_response_client_invalid_url() {
    let url = "invalid_url";
    let response = get_response_client(url.to_string()).await;
    assert!(response.is_err());
}

// Returns an error when given an empty URL
#[tokio::test]
async fn test_get_response_client_empty_url() {
    let url = "";
    let response = get_response_client(url.to_string()).await;
    assert!(response.is_err());
}

// Returns an error when the request times out
#[tokio::test]
async fn test_get_response_client_request_timeout() {
    let url = "https://example.com";
    // Set a very short timeout for testing purposes
    let client = reqwest::Client::builder().timeout(Duration::from_millis(1)).build().unwrap();
    let response = client.get(url).send().await;
    assert!(response.is_err());
}
