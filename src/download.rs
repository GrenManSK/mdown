use std::{ time::{ Duration, Instant }, fs::{ self, File, OpenOptions }, thread::sleep, io::Write };
use crosscurses::*;

use crate::{ string, ARGS };
use crate::utils::process_filename;

async fn get_cover_response(c_hash: &str, cover_hash: &str) -> reqwest::Response {
    reqwest
        ::get(format!("https://uploads.mangadex.org\\covers\\{}\\{}", c_hash, cover_hash)).await
        .unwrap()
}

fn get_size(response: &reqwest::Response) -> (f32, f32) {
    let total_size: f32 = response.content_length().unwrap_or(0) as f32;
    (total_size, (total_size as f32) / (1024 as f32) / (1024 as f32))
}

fn get_perc(percentage: i64) -> String {
    format!("{:>3}", percentage)
}

pub(crate) async fn download_cover(c_hash: &str, cover_hash: &str, folder: &str) {
    string(1, 0, "Downloading cover_art");

    let mut response = get_cover_response(c_hash, cover_hash).await;
    let (total_size, final_size) = get_size(&response);

    let mut file = File::create(format!("{}\\_cover.png", folder)).unwrap();

    let interval = Duration::from_millis(250);
    let mut last_check_time = Instant::now();
    let (mut downloaded, mut last_size) = (0, 0.0);

    while let Some(chunk) = response.chunk().await.unwrap() {
        let _ = file.write_all(&chunk);
        downloaded += chunk.len() as u64;
        let current_time = Instant::now();
        if current_time.duration_since(last_check_time) >= interval {
            last_check_time = current_time;
            let percentage = ((100.0 / (total_size as f32)) * (downloaded as f32)).round() as i64;
            let perc_string = get_perc(percentage);
            let message = format!(
                "Downloading cover_art {}% - {:.2}mb of {:.2}mb [{:.2}mb/s]",
                perc_string,
                (downloaded as f32) / (1024 as f32) / (1024 as f32),
                final_size,
                (((downloaded as f32) - last_size) * 4.0) / (1024 as f32) / (1024 as f32)
            );
            string(
                1,
                0,
                &format!(
                    "{} {}",
                    message,
                    "#".repeat(
                        ((((stdscr().get_max_x() - (message.len() as i32)) as f32) /
                            (total_size as f32)) *
                            (downloaded as f32)) as usize
                    )
                )
            );
            last_size = downloaded as f32;
        }
    }
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
    times: usize
) {
    let mut pr_title = "".to_string();
    if name != "" {
        pr_title = format!(" - {}", name);
    }
    let page = page + 1;
    let page_str = page.to_string() + &" ".repeat(3 - page.to_string().len());
    let base_url;
    if ARGS.saver {
        base_url = "https://uploads.mangadex.org/data-saver/";
    } else {
        base_url = "https://uploads.mangadex.org/data/";
    }
    let full_url = format!("{}{}/{}", base_url, c_hash, f_name);
    let folder_name = process_filename(
        format!("{} - {}Ch.{}{}", manga_name, vol, chapter, pr_title)
    );
    let file_name = process_filename(
        format!("{} - {}Ch.{}{} - {}.jpg", manga_name, vol, chapter, pr_title, page)
    );
    let file_name_brief = process_filename(format!("{}Ch.{} - {}.jpg", vol, chapter, page));

    let lock_file = process_filename(format!("{}.lock", folder_name));

    string(5 + 1, -1 + start + (page as i32), "|");
    string(5 + 1 + (page as i32), 0, "   Sleeping");
    sleep(Duration::from_millis(((page - iter * times) * 50) as u64));
    string(5 + 1 + (page as i32), 0, &format!("   {} Downloading {}", page_str, file_name_brief));
    string(5 + 1, -1 + start + (page as i32), "/");
    let full_path = format!("{}/{}", folder_name, file_name);

    let mut response = reqwest::get(full_url.clone()).await.unwrap();
    let (total_size, final_size) = get_size(&response);

    string(5 + 1, -1 + start + (page as i32), "\\");
    let mut file = File::create(full_path).unwrap();

    let (mut downloaded, mut last_size) = (0, 0.0);
    let interval = Duration::from_millis(250);
    let mut last_check_time = Instant::now();

    while fs::metadata(format!("{}.lock", lock_file)).is_ok() {
        sleep(Duration::from_millis(10));
    }
    let mut lock_file_inst = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(format!("{}_{}_final.lock", folder_name, page))
        .unwrap();
    let _ = write!(lock_file_inst, "{:.2}", total_size);

    while let Some(chunk) = response.chunk().await.unwrap() {
        let _ = file.write_all(&chunk);
        downloaded += chunk.len() as u64;
        let current_time = Instant::now();
        if current_time.duration_since(last_check_time) >= interval {
            if (downloaded as f32) != last_size {
                let mut lock_file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(format!("{}_{}.lock", folder_name, page))
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
            string(
                5 + 1 + (page as i32),
                0,
                &format!(
                    "{} {}",
                    message,
                    "#".repeat(
                        ((((stdscr().get_max_x() - (message.len() as i32)) as f32) /
                            (total_size as f32)) *
                            (downloaded as f32)) as usize
                    )
                )
            );
            last_size = downloaded as f32;
        }
    }

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
                ((((stdscr().get_max_x() - (message.len() as i32)) as f32) / (total_size as f32)) *
                    (downloaded as f32)) as usize
            )
        )
    );
    string(5 + 1, -1 + start + (page as i32), "#");
    let mut lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(format!("{}_{}.lock", folder_name, page))
        .unwrap();
    let _ = lock_file.write(format!("{}", (downloaded as f64) / 1024.0 / 1024.0).as_bytes());
}
