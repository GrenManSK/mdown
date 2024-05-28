use clap::{ Parser, ArgGroup };
use crosscurses::stdscr;
use lazy_static::lazy_static;
use chrono::DateTime;
use serde_json::Value;
use std::{ collections::HashMap, env, fs::{ self, File }, io::Write, process::exit, sync::Arc };
use parking_lot::Mutex;

mod db;
mod download;
mod error;
mod getter;
mod gui;
mod macros;
mod metadata;
mod resolute;
mod server;
mod utils;
mod web;
mod zip_func;

/// Mangadex Manga downloader
#[derive(Parser)]
#[command(
    author = "GrenManSK",
    version,
    about,
    help_template = "{before-help}{name} ({version}) - {author}

{about}

{usage-heading} {usage}

{all-args}
{after-help}",
    help_expected = true,
    long_about = None,
    after_help = "Thanks for using Mdown"
)]
#[clap(group = ArgGroup::new("Search-Options").args(&["url", "search"]))]
#[clap(
    group = ArgGroup::new("Mod-Options").args(
        &[
            "web",
            "server",
            "gui",
            "encode",
            "check",
            "update",
            "show",
            "show_all",
            "force_delete",
            "delete",
            "reset",
        ]
    )
)]
struct Args {
    #[arg(
        short,
        long,
        value_name = "SITE",
        default_value_t = String::from("UNSPECIFIED"),
        next_line_help = true,
        help = format!(
            "url of manga, supply in the format of https:/{}",
            "/mangadex.org/title/[id]/\nor UUID\n"
        )
        // Reason for this format!() is because highlighting error in VS Code;
        // precisely "//" this will break it "url of manga, supply in the format of https://mangadex.org/title/[id]/"
    )]
    url: String,
    #[arg(
        short,
        long,
        value_name = "LANGUAGE",
        default_value_t = String::from("en"),
        next_line_help = true,
        help = "language of manga to download; \"*\" is for all languages\n"
    )]
    lang: String,
    #[arg(
        short,
        long,
        default_value_t = String::from("*"),
        next_line_help = true,
        help = "name of the manga\n"
    )]
    title: String,
    #[arg(
        short,
        long,
        default_value_t = String::from("."),
        next_line_help = true,
        help = "put all chapters in folder specified,\n- if folder name is name it will put in folder same as manga name\n- if folder name is name and title is specified it will make folder same as title\n"
    )]
    folder: String,
    #[arg(
        short,
        long,
        default_value_t = String::from("*"),
        next_line_help = true,
        help = "download only specified volume\n"
    )]
    volume: String,
    #[arg(
        short,
        long,
        default_value_t = String::from("*"),
        next_line_help = true,
        help = "download only specified chapter\n"
    )]
    chapter: String,
    ///
    #[arg(
        short,
        long,
        next_line_help = true,
        help = "download images of lower quality and lower download size; will save network resources and reduce download time"
    )]
    saver: bool,
    #[arg(
        long,
        next_line_help = true,
        help = "add markdown file which contains status information"
    )]
    stat: bool,
    #[arg(long, next_line_help = true, help = "Won't use curses window")]
    quiet: bool,
    #[arg(
        short,
        long,
        default_value_t = String::from("40"),
        next_line_help = true,
        help = "download manga images by supplied number at once;\nit is highly recommended to use MAX 50 because of lack of performance and non complete manga downloading,\nmeaning chapter will not download correctly, meaning missing or corrupt pages\n"
    )]
    max_consecutive: String,
    #[arg(long, next_line_help = true, help = "download manga even if it already exists")]
    force: bool,
    #[arg(
        short,
        long,
        default_value_t = String::from("0"),
        next_line_help = true,
        help = "changes start offset e.g. 50 starts from chapter 50,\nalthough if manga contains chapter like 3.1, 3.2 starting chapter will be moved by number of these chapters\n"
    )]
    offset: String,
    #[arg(
        short,
        long,
        default_value_t = String::from("0"),
        next_line_help = true,
        help = "changes start offset e.g. 50 starts from 50 item in database;\nthis occurs before manga is sorted, which result in some weird behavior like missing chapters\n"
    )]
    database_offset: String,
    #[arg(long, next_line_help = true, help = "database will not be sorted")]
    unsorted: bool,
    #[arg(
        long,
        default_value_t = String::from("./"),
        next_line_help = true,
        help = "change current working directory\n"
    )]
    cwd: String,
    #[arg(
        short,
        long,
        next_line_help = true,
        default_value_t = String::from(""),
        help = "print url in program readable format\n"
    )]
    encode: String,
    #[arg(long, next_line_help = true, help = "print log")]
    log: bool,
    #[arg(long, next_line_help = true, help = "Check downloaded files for errors")]
    check: bool,
    #[arg(long, next_line_help = true, help = "Check downloaded files for errors")]
    update: bool,
    #[arg(
        long,
        default_value_t = String::from("*"),
        next_line_help = true,
        help = "download manga by manga title\n"
    )]
    search: String,
    #[arg(long, next_line_help = true, help = "Shows current manga in database")]
    show: bool,
    #[arg(long, next_line_help = true, help = "Shows current chapters in database")]
    show_all: bool,
    #[arg(
        short,
        long,
        next_line_help = true,
        help = "enter web mode and will open browser on port 8080, core lock file will not be initialized; result will be printed gradually during download process"
    )]
    web: bool,
    #[arg(long, next_line_help = true, help = "Starts server")]
    server: bool,
    /// Reset-Options
    #[arg(
        long,
        next_line_help = true,
        help = "force to delete *.lock file which is stopping from running another instance of program;\nNOTE that if you already have one instance running it will fail to delete the original file and thus it will crash"
    )]
    force_delete: bool,
    #[arg(long, next_line_help = true, help = "Delete database")]
    delete: bool,
    #[arg(long, next_line_help = true, help = "Delete database")]
    reset: bool,
    /// dev
    #[arg(long, next_line_help = true, help = "Gui version of mdown, does nothing for now")]
    gui: bool,
    #[arg(long, next_line_help = true, help = "debug")]
    debug: bool,
    #[arg(long, next_line_help = true, help = "dev")]
    dev: bool,
}

fn string(y: u32, x: u32, value: &str) {
    if !ARGS.web && !ARGS.gui && !ARGS.check && !ARGS.update && !ARGS.quiet {
        stdscr().mvaddnstr(y as i32, x as i32, value, (MAXPOINTS.max_x - x) as i32);
        stdscr().refresh();
    }
}

fn log_end(handle_id: Box<str>) {
    resolute::HANDLE_ID_END.lock().push(handle_id);
}

lazy_static! {
    pub(crate) static ref ARGS: Args = Args::parse();
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
    pub(crate) static ref IS_END: Mutex<bool> = Mutex::new(false);
}

#[tokio::main]
async fn main() {
    match start().await {
        Ok(()) => error::handle_suspended(),
        Err(err) => {
            error::handle_final(&err);
            exit(1);
        }
    }
    if !ARGS.web && !ARGS.gui && !ARGS.check && !ARGS.update && !ARGS.quiet && !ARGS.reset {
        crosscurses::echo();
        crosscurses::cbreak();
    }
    if *resolute::FINAL_END.lock() {
        exit(0);
    }
}
async fn start() -> Result<(), error::Final> {
    if ARGS.encode != "" {
        println!("{}", web::encode(&ARGS.encode));
        return Ok(());
    }

    if ARGS.reset {
        return match utils::reset() {
            Ok(()) => Ok(()),
            Err(err) => Err(error::Final::Final(err)),
        };
    }

    match db::init().await {
        Ok(()) => (),
        Err(err) => {
            error::handle_final(&error::Final::Final(err));
            exit(1);
        }
    }

    // cwd
    match env::set_current_dir(ARGS.cwd.as_str()) {
        Ok(()) => (),
        Err(err) => {
            let err = error::MdownError::IoError(err, Some(ARGS.cwd.to_string()));
            error::handle_error(&err, String::from("program"));
            return Err(error::Final::Final(err));
        }
    }

    if ARGS.delete {
        return match resolute::args_delete() {
            Ok(()) => Ok(()),
            Err(err) => Err(error::Final::Final(err)),
        };
    }

    match utils::create_cache_folder() {
        Ok(()) => (),
        Err(err) => {
            return Err(error::Final::Final(err));
        }
    }

    // subscriber
    if ARGS.web || ARGS.gui || ARGS.update || ARGS.log || ARGS.server {
        match utils::setup_subscriber() {
            Ok(()) => (),
            Err(err) => {
                return Err(error::Final::Final(err));
            }
        }
    }

    *resolute::LANGUAGE.lock() = ARGS.lang.clone();

    if ARGS.show || ARGS.show_all {
        match resolute::show().await {
            Ok(()) => (),
            Err(err) => {
                return Err(error::Final::Final(err));
            }
        }
        match utils::remove_cache() {
            Ok(()) => (),
            Err(err) => {
                return Err(error::Final::Final(err));
            }
        }
        return Ok(());
    }

    if ARGS.check || ARGS.update {
        match resolute::resolve_check().await {
            Ok(()) => (),
            Err(err) => {
                return Err(error::Final::Final(err));
            }
        }

        match utils::remove_cache() {
            Ok(()) => (),
            Err(err) => {
                return Err(error::Final::Final(err));
            }
        }
        return Ok(());
    }

    tokio::spawn(async { utils::log_handler() });

    if ARGS.server {
        return match server::start() {
            Ok(()) => Ok(()),
            Err(err) => Err(error::Final::Final(err)),
        };
    }

    //gui
    if ARGS.gui {
        return match gui::start() {
            Ok(()) => Ok(()),
            Err(err) => Err(error::Final::Final(err)),
        };
    }

    // web
    if ARGS.web {
        return match web::start().await {
            Ok(()) => Ok(()),
            Err(err) => Err(error::Final::Final(err)),
        };
    }
    let (file_path, file_path_tm) = match utils::resolve_start() {
        Ok((file_path, file_path_tm)) => (file_path, file_path_tm),
        Err(err) => {
            return Err(error::Final::Final(err));
        }
    };

    if !ARGS.quiet {
        utils::setup_requirements(file_path_tm);
    }

    let mut manga_name: String = String::from("!");
    let mut status_code = match reqwest::StatusCode::from_u16(200) {
        Ok(code) => code,
        Err(err) => {
            return Err(
                error::Final::Final(
                    error::MdownError::CustomError(
                        err.to_string(),
                        String::from("InvalidStatusCode")
                    )
                )
            );
        }
    };

    let id;

    if ARGS.search != String::from("*") {
        id = match utils::search().await {
            Ok(id) => id,
            Err(err) => {
                return Err(error::Final::Final(err));
            }
        };
    } else if let Some(id_temp) = utils::resolve_regex(&ARGS.url) {
        id = id_temp.as_str().to_string();
    } else if utils::is_valid_uuid(&ARGS.url) {
        id = ARGS.url.clone();
    } else {
        id = String::from("*");
    }
    if id != String::from("*") {
        *resolute::MANGA_ID.lock() = id.to_string();
        string(0, 0, &format!("Extracted ID: {}", id));
        string(1, 0, &format!("Getting manga information ..."));
        match getter::get_manga_json(&id).await {
            Ok(manga_name_json) => {
                string(1, 0, &format!("Getting manga information DONE"));
                let json_value = match utils::get_json(&manga_name_json) {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(error::Final::Final(err));
                    }
                };
                if let Value::Object(obj) = json_value {
                    manga_name = match resolute::resolve(obj, &id).await {
                        Ok(value) => value,
                        Err(err) => {
                            error::handle_error(&err, String::from("program"));
                            String::from("!")
                        }
                    };
                } else {
                    return Err(
                        error::Final::Final(
                            error::MdownError::JsonError(String::from("Unexpected JSON value"))
                        )
                    );
                }
            }
            Err(code) => {
                string(2, 0, &format!("Getting manga information ERROR"));
                let code = code.into();
                let parts: Vec<&str> = code.split_whitespace().collect();

                if let Some(status_code_tmp) = parts.get(0) {
                    status_code = match
                        reqwest::StatusCode::from_u16(match status_code_tmp.parse::<u16>() {
                            Ok(code) => code,
                            Err(err) => {
                                return Err(
                                    error::Final::Final(
                                        error::MdownError::ConversionError(
                                            format!("status_code {}", err.to_string())
                                        )
                                    )
                                );
                            }
                        })
                    {
                        Ok(code) => code,
                        Err(err) => {
                            return Err(
                                error::Final::Final(
                                    error::MdownError::CustomError(
                                        err.to_string(),
                                        String::from("InvalidStatusCode")
                                    )
                                )
                            );
                        }
                    };
                } else {
                    println!("Invalid status string");
                }
            }
        }
    }

    match utils::resolve_end(file_path, manga_name, status_code) {
        Ok(()) => (),
        Err(err) => eprintln!("Error: {}", err),
    }

    utils::resolve_final_end();

    *resolute::ENDED.lock() = true;

    // Final key input is in utils::ctrl_handler
    Ok(())
}

pub(crate) async fn download_manga(
    manga_json: String,
    manga_name: &str,
    arg_force: bool
) -> Result<Vec<String>, error::MdownError> {
    let folder = getter::get_folder_name(manga_name);
    let arg_volume = getter::get_arg(ARGS.volume.to_string());
    let arg_chapter = getter::get_arg(ARGS.chapter.to_string());
    let arg_offset: u32 = match getter::get_arg(ARGS.offset.to_string()).parse() {
        Ok(value) => value,
        Err(_err) => 0,
    };
    let (mut downloaded, mut hist) = (vec![], vec![]);
    let (mut times, mut moves) = (0, 0);
    let language_inst = resolute::LANGUAGE.lock().clone();
    let language = language_inst.clone();
    drop(language_inst);
    let mut filename;
    let json_value = match utils::get_json(&manga_json) {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };
    match json_value {
        Value::Object(obj) => {
            let data_array = utils::sort(match obj.get("data").and_then(Value::as_array) {
                Some(value) => value,
                None => {
                    return Err(error::MdownError::NotFoundError(String::from("download_manga")));
                }
            });
            let data_len = data_array.len();
            *resolute::CURRENT_CHAPTER_PARSED_MAX.lock() = data_len as u64;
            for item in 0..data_len {
                let mut date_change = false;
                let parsed = format!(
                    "   Parsed chapters: {}/{}",
                    resolute::CURRENT_CHAPTER_PARSED.lock(),
                    resolute::CURRENT_CHAPTER_PARSED_MAX.lock()
                );
                if !ARGS.web && !ARGS.gui && !ARGS.check && !ARGS.update {
                    string(0, MAXPOINTS.max_x - (parsed.len() as u32), &parsed);
                }
                let array_item = getter::get_attr_as_same_from_vec(&data_array, item);
                let value = getter::get_attr_as_same(array_item, "id").to_string();
                let id = value.trim_matches('"');

                let message = format!(" ({}) Found chapter with id: {}", item as u32, id);
                if ARGS.web || ARGS.gui || ARGS.check || ARGS.update || ARGS.log {
                    log!(&message);
                }
                string(1, 0, &message);

                let (chapter_attr, lang, pages, chapter_num, mut title) =
                    getter::get_metadata(array_item);

                title = resolute::title(title);

                let con_chap = resolute::resolve_skip(arg_chapter.clone(), chapter_num);
                let vol;
                let vol_temp = getter::get_attr_as_str(chapter_attr, "volume");
                if vol_temp == "" {
                    vol = String::from("");
                } else {
                    vol = format!("Vol.{} ", &vol_temp);
                }

                let con_vol = resolute::resolve_skip(arg_volume.clone(), vol_temp);

                filename = utils::FileName {
                    manga_name: manga_name.to_string(),
                    vol: vol.to_string(),
                    chapter_num: chapter_num.to_string(),
                    title: title.to_string(),
                    folder: folder.to_string(),
                };
                let folder_path = filename.get_folder_name();
                if
                    (lang == language || language == "*") &&
                    chapter_num != "This is test" &&
                    fs::metadata(filename.get_file_w_folder()).is_ok() &&
                    !arg_force &&
                    !(match resolute::check_for_metadata_saver(&filename.get_file_w_folder()) {
                        Ok(metadata) => if !ARGS.check { metadata } else { false }
                        Err(err) => {
                            return Err(err);
                        }
                    })
                {
                    let mut cont = true;
                    let update_date = getter::get_attr_as_str(chapter_attr, "updatedAt");
                    match DateTime::parse_from_rfc3339(update_date) {
                        Ok(datetime) => {
                            let mut dates = resolute::CHAPTER_DATES.lock();
                            let empty = String::new();

                            let cur_date = match dates.get(chapter_num) {
                                Some(date) => date.to_owned(),
                                None => empty,
                            };

                            match DateTime::parse_from_rfc3339(&cur_date) {
                                Ok(datetime_cur) => {
                                    if datetime_cur < datetime {
                                        date_change = true;
                                        cont = false;
                                        dates.remove(chapter_num);
                                        if ARGS.update {
                                            resolute::CHAPTERS_TO_REMOVE
                                                .lock()
                                                .push(
                                                    metadata::ChapterMetadata::new(
                                                        chapter_num,
                                                        &cur_date,
                                                        id
                                                    )
                                                );
                                        }
                                    } else if datetime_cur > datetime {
                                        resolute::FIXED_DATES.lock().push(chapter_num.to_string());
                                        resolute::CHAPTERS_TO_REMOVE
                                            .lock()
                                            .push(
                                                metadata::ChapterMetadata::new(
                                                    chapter_num,
                                                    &cur_date,
                                                    id
                                                )
                                            );
                                    }
                                }
                                Err(_err) => (),
                            }
                            drop(dates);
                        }
                        Err(_err) => (),
                    }
                    *resolute::CURRENT_CHAPTER_PARSED.lock() += 1;
                    if
                        cont &&
                        (lang == language || language == "*") &&
                        chapter_num != "This is test"
                    {
                        resolute::CHAPTERS
                            .lock()
                            .push(metadata::ChapterMetadata::new(&chapter_num, update_date, id));
                        (moves, hist) = utils::skip(
                            utils::process_filename(&folder_path),
                            item,
                            moves,
                            hist.clone()
                        );
                        continue;
                    }
                }

                if con_vol {
                    (moves, hist) = utils::skip_didnt_match("volume", item, moves, hist.clone());
                    continue;
                }
                if con_chap {
                    (moves, hist) = utils::skip_didnt_match("chapter", item, moves, hist.clone());
                    continue;
                }
                if pages == 0 {
                    (moves, hist) = utils::skip_custom("pages is 0", item, moves, hist.clone());
                    continue;
                }
                if
                    (lang == language || language == "*") &&
                    chapter_num != "This is test" &&
                    !resolute::CHAPTERS
                        .lock()
                        .iter()
                        .any(|item| item.number == chapter_num)
                {
                    if ARGS.check {
                        let dates = resolute::CHAPTER_DATES.lock();
                        let empty = String::new();

                        let cur_date = match dates.get(chapter_num) {
                            Some(date) => date.to_owned(),
                            None => empty,
                        };
                        resolute::CHAPTERS_TO_REMOVE
                            .lock()
                            .push(metadata::ChapterMetadata::new(chapter_num, &cur_date, id));
                        drop(dates);
                    }
                    let update_date = getter::get_attr_as_str(chapter_attr, "updatedAt");
                    *resolute::CURRENT_CHAPTER_PARSED.lock() += 1;
                    if arg_offset > times {
                        (moves, hist) = utils::skip_offset(item, moves, hist);
                        times += 1;
                        *resolute::CURRENT_CHAPTER_PARSED.lock() += 1;
                        continue;
                    }
                    utils::clear_screen(2);
                    let folder_path = filename.get_folder_w_end();
                    let mut pr_title_full = "".to_string();
                    if title != "" {
                        pr_title_full = format!(";Title: {}", title);
                    }
                    let message = format!(
                        "  Metadata: Language: {};Pages: {};{};Chapter: {}{}",
                        lang,
                        pages,
                        vol,
                        chapter_num,
                        pr_title_full
                    );
                    if ARGS.web || ARGS.gui || ARGS.check || ARGS.update || ARGS.log {
                        log!(&message);
                    }
                    string(2, 0, &message);
                    if
                        !ARGS.check ||
                        !resolute::CHAPTERS
                            .lock()
                            .iter()
                            .any(|chapter| chapter.number == chapter_num.to_string())
                    {
                        if ARGS.check {
                            match date_change {
                                true => {
                                    resolute::TO_DOWNLOAD_DATE.lock().push(chapter_num.to_string());
                                }
                                false => {
                                    resolute::TO_DOWNLOAD.lock().push(chapter_num.to_string());
                                }
                            }
                            continue;
                        }
                        let (name, website) = match resolute::resolve_group(array_item).await {
                            Ok((name, website)) => (name, website),
                            Err(err) => {
                                error::handle_error(&err, String::from("group"));
                                (String::from("null"), String::from("null"))
                            }
                        };
                        match getter::get_chapter(id).await {
                            Ok(json) => {
                                match
                                    download_chapter(
                                        id,
                                        json,
                                        array_item,
                                        &manga_name,
                                        title,
                                        &vol,
                                        chapter_num,
                                        &filename,
                                        update_date,
                                        &name,
                                        &website
                                    ).await
                                {
                                    Ok(()) => (),
                                    Err(err) => {
                                        error::handle_error(&err, String::from("chapter"));
                                    }
                                };
                            }
                            Err(err) => {
                                resolute::SUSPENDED.lock().push(err);
                            }
                        }
                        // prettier-ignore or #[rustfmt::skip]
                        if  *IS_END.lock() {
                            return Ok(downloaded);
                        }
                        match resolute::get_scanlation_group_to_file(manga_name, name, website) {
                            Ok(()) => (),
                            Err(err) => {
                                return Err(err);
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
                        zip_func::to_zip(&folder_path, &file_name);
                        match fs::remove_dir_all(folder_path.clone()) {
                            Ok(()) => (),
                            Err(err) => {
                                return Err(
                                    error::MdownError::IoError(err, Some(folder_path.clone()))
                                );
                            }
                        }

                        utils::clear_screen(2);
                        if ARGS.web || ARGS.gui || ARGS.check || ARGS.update {
                            resolute::WEB_DOWNLOADED.lock().push(file_name);
                        } else {
                            downloaded.push(filename.get_file_w_folder_w_cwd());
                        }
                        let mut current_chapter = resolute::CURRENT_CHAPTER.lock();
                        current_chapter.clear();
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                } else {
                    string(2, 0, &format!("{}", " ".repeat(MAXPOINTS.max_x as usize)));
                    let message = format!(
                        "Skipping because of wrong language; found '{}', target '{}' ...",
                        lang,
                        language
                    );
                    string(2, 0, &format!("  {}", message));

                    if ARGS.web || ARGS.gui || ARGS.check || ARGS.update || ARGS.log {
                        log!(&format!("({}) {}", item, message));
                    }

                    *resolute::CURRENT_CHAPTER_PARSED_MAX.lock() -= 1;
                }
            }
        }
        _ => {
            eprintln!("JSON is not an object.");
        }
    }
    if ARGS.debug {
        match utils::debug_print(hist, "hist.txt") {
            Ok(()) => (),
            Err(_err) => (),
        };
    }
    Ok(downloaded)
}

pub(crate) async fn download_chapter(
    id: &str,
    manga_chapter_json: String,
    manga_json: &Value,
    manga_name: &str,
    title: &str,
    vol: &str,
    chapter: &str,
    filename: &utils::FileName,
    update_date: &str,
    name: &str,
    website: &str
) -> Result<(), error::MdownError> {
    string(3, 0, &format!("  Downloading images in folder: {}:", filename.get_folder_name()));
    if ARGS.web || ARGS.gui || ARGS.check || ARGS.update || ARGS.log {
        let mut current_chapter = resolute::CURRENT_CHAPTER.lock();
        current_chapter.clear();
        current_chapter.push_str(&filename.get_folder_name());
        drop(current_chapter);
        log!(&format!("Downloading images in folder: {}", filename.get_folder_name()));
    }
    let json_value = match utils::get_json(&manga_chapter_json) {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };
    match json_value {
        Value::Object(obj) => {
            let image_base_url = match obj.get("baseUrl").and_then(Value::as_str) {
                Some(value) => value,
                None => "https://uploads.mangadex.org",
            };
            if let Some(data_array) = obj.get("chapter") {
                if let Some(chapter_hash) = data_array.get("hash").and_then(Value::as_str) {
                    let saver = get_saver!();
                    let mut images1 = data_array.get(saver.clone()).and_then(Value::as_array);
                    if images1.is_none() {
                        images1 = data_array.get(get_saver!(true)).and_then(Value::as_array);
                    }
                    if let Some(images1) = images1 {
                        let images_length = images1.len();

                        *resolute::CURRENT_PAGE_MAX.lock() = images_length.clone() as u64;

                        if let Some(images) = data_array.get(saver.clone()) {
                            let lock_file = filename.get_lock();
                            let mut lock_file_inst = match File::create(lock_file.clone()) {
                                Ok(file) => file,
                                Err(err) => {
                                    return Err(
                                        error::MdownError::IoError(err, Some(lock_file.clone()))
                                    );
                                }
                            };
                            match write!(lock_file_inst, "0") {
                                Ok(()) => (),
                                Err(err) => {
                                    eprintln!("Error: writing in chapter lock file {}", err);
                                }
                            }
                            match fs::create_dir_all(filename.get_folder_w_end()) {
                                Ok(()) => (),
                                Err(err) =>
                                    eprintln!(
                                        "Error: creating directory {} {}",
                                        filename.get_folder_w_end(),
                                        err
                                    ),
                            }

                            let mut metadata_file = match
                                File::create(format!("{}_metadata", filename.get_folder_w_end()))
                            {
                                Ok(file) => file,
                                Err(err) => {
                                    return Err(
                                        error::MdownError::IoError(err, Some(lock_file.clone()))
                                    );
                                }
                            };
                            let attr = match manga_json.get("attributes") {
                                Some(attr) => attr,
                                None => {
                                    return Err(
                                        error::MdownError::NotFoundError(
                                            String::from("attributes not found")
                                        )
                                    );
                                }
                            };

                            let pages = match
                                serde_json::to_string(match attr.get("pages") {
                                    Some(pages) => pages,
                                    None => {
                                        return Err(
                                            error::MdownError::JsonError(
                                                String::from("pages not found")
                                            )
                                        );
                                    }
                                })
                            {
                                Ok(pages) => pages,
                                Err(_err) => "null".to_string(),
                            };

                            let mut scanlation = serde_json::Map::new();
                            scanlation.insert(
                                String::from("name"),
                                serde_json::Value::String(name.to_string())
                            );
                            scanlation.insert(
                                String::from("website"),
                                serde_json::Value::String(website.to_string())
                            );

                            let response_map: HashMap<&str, serde_json::Value> = [
                                (
                                    "name",
                                    serde_json::Value::String(
                                        resolute::MANGA_NAME.lock().to_string()
                                    ),
                                ),
                                (
                                    "id",
                                    serde_json::Value::String(
                                        resolute::MANGA_ID.lock().to_string()
                                    ),
                                ),
                                (
                                    "saver",
                                    serde_json::Value::String(resolute::SAVER.lock().to_string()),
                                ),
                                (
                                    "title",
                                    serde_json::Value::String(match title {
                                        "" => "null".to_string(),
                                        x => x.to_string(),
                                    }),
                                ),
                                ("pages", serde_json::Value::String(pages.to_string())),
                                (
                                    "chapter",
                                    serde_json::Value::String(match chapter {
                                        "" => "null".to_string(),
                                        x => x.to_string(),
                                    }),
                                ),
                                (
                                    "volume",
                                    serde_json::Value::String(match vol {
                                        "" => "null".to_string(),
                                        x => x.to_string(),
                                    }),
                                ),
                                ("scanlation", serde_json::Value::Object(scanlation)),
                            ]
                                .iter()
                                .cloned()
                                .collect();
                            let json = match serde_json::to_string_pretty(&response_map) {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(error::MdownError::JsonError(err.to_string()));
                                }
                            };
                            match write!(metadata_file, "{}", json) {
                                Ok(()) => (),
                                Err(err) => {
                                    eprintln!("Error: writing in chapter metadata file {}", err);
                                }
                            }

                            let lock_file_wait = filename.get_folder_name();

                            tokio::spawn(async move {
                                utils::wait_for_end(lock_file_wait, images_length).await
                            });
                            match fs::create_dir_all(filename.get_folder_w_end()) {
                                Ok(()) => (),
                                Err(err) =>
                                    eprintln!(
                                        "Error: creating directory {} {}",
                                        filename.get_folder_w_end(),
                                        err
                                    ),
                            }
                            let start = MAXPOINTS.max_x / 3 - (images_length as u32) / 2;

                            let iter = match ARGS.max_consecutive.parse() {
                                Ok(x) => x,
                                Err(_err) => {
                                    resolute::SUSPENDED
                                        .lock()
                                        .push(
                                            error::MdownError::ConversionError(
                                                String::from("Failed to parse max_consecutive")
                                            )
                                        );
                                    40 as usize
                                }
                            };

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

                                let tasks = (start_task..end_task).map(|item| {
                                    let image_temp = getter
                                        ::get_attr_as_same_as_index(images, item)
                                        .to_string();
                                    let chapter_hash = Arc::from(chapter_hash);
                                    let saver = Arc::from(match saver.as_str() {
                                        "data" => "data",
                                        "dataSaver" => "data-saver",
                                        _ => "data",
                                    });
                                    let image = Arc::from(image_temp.trim_matches('"'));
                                    let image_base_url = Arc::from(image_base_url);
                                    let page = item + 1;
                                    let page_str =
                                        page.to_string() + &" ".repeat(3 - page.to_string().len());

                                    let pr_title = match title != "" {
                                        true => format!(" - {}", title),
                                        false => "".to_string(),
                                    };
                                    let folder_name = utils::process_filename(
                                        &format!(
                                            "{} - {}Ch.{}{}",
                                            manga_name,
                                            vol,
                                            chapter,
                                            pr_title
                                        )
                                    );
                                    let file_name = utils::process_filename(
                                        &format!(
                                            "{} - {}Ch.{}{} - {}.jpg",
                                            manga_name,
                                            vol,
                                            chapter,
                                            pr_title,
                                            page
                                        )
                                    );
                                    let file_name_brief = utils::process_filename(
                                        &format!("{}Ch.{} - {}.jpg", vol, chapter, page)
                                    );

                                    let lock_file = utils::process_filename(
                                        &format!(".cache\\{}.lock", folder_name)
                                    );
                                    let full_path = format!(".cache/{}/{}", folder_name, file_name);

                                    string(3 + 1 + (page as u32), 0, " Pending");

                                    tokio::spawn(async move {
                                        match
                                            download::download_image(
                                                image_base_url,
                                                chapter_hash,
                                                image,
                                                page,
                                                page_str,
                                                folder_name,
                                                file_name_brief,
                                                lock_file,
                                                full_path,
                                                saver,
                                                start,
                                                iter,
                                                i
                                            ).await
                                        {
                                            Ok(()) => (),
                                            Err(err) => {
                                                error::handle_error(&err, String::from("image"));
                                            }
                                        };
                                    })
                                });

                                utils::progress_bar_preparation(start, images_length, 4);

                                let _: Vec<_> = futures::future
                                    ::join_all(tasks).await
                                    .into_iter()
                                    .collect();
                                // prettier-ignore
                                if  *IS_END.lock() 
                                        {
                                            std::thread::sleep(std::time::Duration::from_millis(1000));
                                            *( IS_END.lock() 
                                            ) = false;
                                            return Ok(());
                                        }
                            }

                            *resolute::CURRENT_PAGE.lock() = 0;

                            let chapter_met = metadata::ChapterMetadata::new(
                                chapter,
                                update_date,
                                id
                            );
                            resolute::CHAPTERS.lock().push(chapter_met);

                            match resolute::resolve_dat() {
                                Ok(()) => (),
                                Err(err) =>
                                    eprintln!("resolute::resolve_dat() in download_chapter() Error: {}", err),
                            }
                            match fs::remove_file(lock_file.clone()) {
                                Ok(()) => (),
                                Err(_err) => (), // Removing .cache/NAME - CH.X.lock file will result in error
                            };
                        }
                    } else {
                        eprintln!("Missing data for chapter");
                    }
                } else {
                    eprintln!("Chapter number missing");
                }
            } else {
                eprintln!("JSON does not contain a 'chapter' array.");
            }
        }
        _ => {
            eprintln!("JSON is not an object.");
        }
    }

    Ok(())
}
