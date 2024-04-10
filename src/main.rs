use clap::Parser;
use crosscurses::stdscr;
use lazy_static::lazy_static;
use chrono::DateTime;
use serde_json::Value;
use std::{
    collections::HashMap,
    env,
    fs::{ self, File },
    io::Write,
    process::exit,
    sync::{ Arc, Mutex },
};
use tracing::info;

mod download;
mod error;
mod getter;
mod gui;
mod resolute;
mod utils;
mod web;
mod zip_func;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
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
    #[arg(long, next_line_help = true, help = "database will not be sorted")]
    unsorted: bool,
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
        exclusive = true,
        help = "force to delete *.lock file which is stopping from running another instance of program;\nNOTE that if you already have one instance running it will fail to delete the original file and thus it will crash"
    )]
    force_delete: bool,
    #[arg(
        long,
        default_value_t = String::from("./"),
        next_line_help = true,
        help = "change current working directory\n"
    )]
    cwd: String,
    #[arg(long, next_line_help = true, help = "add txt file which contains status information")]
    stat: bool,
    #[arg(
        short,
        long,
        next_line_help = true,
        help = "enter web mode and will open browser on port 8080, core lock file will not be initialized; result will be printed gradually during download process"
    )]
    web: bool,
    #[arg(
        short,
        long,
        next_line_help = true,
        default_value_t = String::from(""),
        help = "print url in program readable format\n"
    )]
    encode: String,
    #[arg(
        long,
        next_line_help = true,
        requires = "web",
        help = "print progress requests when received, \"--web\" flag need to be set for this to work"
    )]
    log: bool,
    #[arg(
        long,
        next_line_help = true,
        exclusive = true,
        help = "Check downloaded files for errors"
    )]
    check: bool,
    #[arg(
        long,
        next_line_help = true,
        exclusive = true,
        help = "Check downloaded files for errors"
    )]
    update: bool,
    #[arg(long, next_line_help = true, exclusive = true, help = "Delete database")]
    delete: bool,
    #[arg(
        long,
        default_value_t = String::from("*"),
        next_line_help = true,
        help = "download manga by manga title\n"
    )]
    search: String,
    #[arg(long, next_line_help = true, help = "Shows current manga in database")]
    show: bool,
    #[arg(long, next_line_help = true, help = "Gui version of mdown, does nothing for now")]
    gui: bool,
}

fn string(y: i32, x: i32, value: &str) {
    if !ARGS.web || !ARGS.gui || !ARGS.check || !ARGS.update {
        stdscr().mvaddnstr(y, x, value, MAXPOINTS.max_x - x);
        stdscr().refresh();
    }
}
pub(crate) struct MaxPoints {
    max_x: i32,
    max_y: i32,
}

lazy_static! {
    pub(crate) static ref ARGS: Args = Args::parse();
    pub(crate) static ref MAXPOINTS: MaxPoints = MaxPoints {
        max_x: stdscr().get_max_x(),
        max_y: stdscr().get_max_y(),
    };
    pub(crate) static ref IS_END: Mutex<bool> = Mutex::new(false);
    pub(crate) static ref MANGA_ID: Mutex<String> = Mutex::new(String::new());
}

#[tokio::main]
async fn main() {
    if ARGS.encode != "" {
        println!("{}", web::encode(&ARGS.encode));
        return ();
    }
    match start().await {
        Ok(()) => error::handle_suspended(),
        Err(err) => error::handle_final(err),
    };
}
async fn start() -> Result<(), error::mdown::Final> {
    // cwd
    match env::set_current_dir(ARGS.cwd.as_str()) {
        Ok(()) => (),
        Err(err) => {
            let err = error::mdown::Error::IoError(err, Some(ARGS.cwd.to_string()));
            error::handle_error(&err, String::from("program"));
            return Err(error::mdown::Final::Final(err));
        }
    }

    if ARGS.delete {
        return match resolute::args_delete() {
            Ok(()) => Ok(()),
            Err(err) => Err(error::mdown::Final::Final(err)),
        };
    }

    match utils::create_cache_folder() {
        Ok(()) => (),
        Err(err) => {
            return Err(error::mdown::Final::Final(err));
        }
    }

    // subscriber
    if ARGS.web || ARGS.gui || ARGS.update || ARGS.gui {
        match utils::setup_subscriber() {
            Ok(()) => (),
            Err(err) => {
                return Err(error::mdown::Final::Final(err));
            }
        }
    }

    //gui
    if ARGS.gui {
        return match gui::start() {
            Ok(()) => Ok(()),
            Err(err) => Err(error::mdown::Final::Final(err)),
        };
    }

    if ARGS.show {
        match resolute::show().await {
            Ok(()) => (),
            Err(err) => {
                return Err(error::mdown::Final::Final(err));
            }
        }
        match utils::remove_cache() {
            Ok(()) => (),
            Err(err) => {
                return Err(error::mdown::Final::Final(err));
            }
        }
        return Ok(());
    }

    if ARGS.check || ARGS.update {
        match resolute::resolve_check().await {
            Ok(()) => (),
            Err(err) => {
                return Err(error::mdown::Final::Final(err));
            }
        }

        match utils::remove_cache() {
            Ok(()) => (),
            Err(err) => {
                return Err(error::mdown::Final::Final(err));
            }
        }
        return Ok(());
    }

    // web
    if ARGS.web {
        return match web::start().await {
            Ok(()) => Ok(()),
            Err(err) => Err(error::mdown::Final::Final(err)),
        };
    }
    let (file_path, file_path_tm) = match utils::resolve_start() {
        Ok((file_path, file_path_tm)) => (file_path, file_path_tm),
        Err(err) => {
            return Err(error::mdown::Final::Final(err));
        }
    };

    utils::setup_requirements(file_path_tm);

    let mut manga_name: String = String::from("!");
    let mut status_code = match reqwest::StatusCode::from_u16(200) {
        Ok(code) => code,
        Err(err) => {
            return Err(
                error::mdown::Final::Final(
                    error::mdown::Error::CustomError(
                        err.to_string(),
                        String::from("InvalidStatusCode")
                    )
                )
            );
        }
    };

    *(match resolute::LANGUAGE.lock() {
        Ok(value) => value,
        Err(err) => {
            return Err(
                error::mdown::Final::Final(error::mdown::Error::PoisonError(err.to_string()))
            );
        }
    }) = ARGS.lang.clone();

    let id;

    if ARGS.search != String::from("*") {
        id = match utils::search().await {
            Ok(id) => id,
            Err(err) => {
                return Err(error::mdown::Final::Final(err));
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
        *(match MANGA_ID.lock() {
            Ok(value) => value,
            Err(err) => {
                error::handle_error(
                    &error::mdown::Error::PoisonError(err.to_string()),
                    String::from("program")
                );
                exit(1);
            }
        }) = id.to_string();
        string(0, 0, &format!("Extracted ID: {}", id));
        match getter::get_manga_json(&id).await {
            Ok(manga_name_json) => {
                let json_value = match utils::get_json(&manga_name_json) {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(error::mdown::Final::Final(err));
                    }
                };
                if let Value::Object(obj) = json_value {
                    manga_name = match
                        resolute::resolve(obj, &id, Some(String::new().into_boxed_str())).await
                    {
                        Ok(value) => value,
                        Err(err) => {
                            error::handle_error(&err, String::from("program"));
                            String::from("!")
                        }
                    };
                } else {
                    return Err(
                        error::mdown::Final::Final(
                            error::mdown::Error::JsonError(String::from("Unexpected JSON value"))
                        )
                    );
                }
            }
            Err(code) => {
                let code = code.into();
                let parts: Vec<&str> = code.split_whitespace().collect();

                if let Some(status_code_tmp) = parts.get(0) {
                    status_code = match
                        reqwest::StatusCode::from_u16(match status_code_tmp.parse::<u16>() {
                            Ok(code) => code,
                            Err(err) => {
                                return Err(
                                    error::mdown::Final::Final(
                                        error::mdown::Error::ConversionError(
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
                                error::mdown::Final::Final(
                                    error::mdown::Error::CustomError(
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

    *(match resolute::ENDED.lock() {
        Ok(value) => value,
        Err(err) => {
            return Err(
                error::mdown::Final::Final(error::mdown::Error::PoisonError(err.to_string()))
            );
        }
    }) = true;
    // Final key input is in utils::ctrl_handler
    Ok(())
}

pub(crate) async fn download_manga(
    manga_json: String,
    manga_name: &str,
    arg_force: bool,
    handle_id: Option<Box<str>>
) -> Result<Vec<String>, error::mdown::Error> {
    let handle_id = match handle_id {
        Some(id) => id,
        None => String::from("0").into_boxed_str(),
    };
    let folder = getter::get_folder_name(manga_name);
    let arg_volume = getter::get_arg(ARGS.volume.to_string());
    let arg_chapter = getter::get_arg(ARGS.chapter.to_string());
    let arg_offset: i32 = match getter::get_arg(ARGS.offset.to_string()).parse() {
        Ok(value) => value,
        Err(_err) => 0,
    };
    let (mut downloaded, mut hist) = (vec![], vec![]);
    let (mut times, mut moves) = (0, 0);
    let language = match resolute::LANGUAGE.lock() {
        Ok(value) => value.to_string(),
        Err(err) => {
            return Err(error::mdown::Error::PoisonError(err.to_string()));
        }
    };
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
                    return Err(error::mdown::Error::NotFoundError(String::from("download_manga")));
                }
            });
            let data_len = data_array.len();
            *(match resolute::CURRENT_CHAPTER_PARSED_MAX.lock() {
                Ok(value) => value,
                Err(err) => {
                    return Err(error::mdown::Error::PoisonError(err.to_string()));
                }
            }) = data_len as u64;
            for item in 0..data_len {
                let mut date_change = false;
                let parsed = format!(
                    "   Parsed chapters: {}/{}",
                    match resolute::CURRENT_CHAPTER_PARSED.lock() {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(error::mdown::Error::PoisonError(err.to_string()));
                        }
                    },
                    match resolute::CURRENT_CHAPTER_PARSED_MAX.lock() {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(error::mdown::Error::PoisonError(err.to_string()));
                        }
                    }
                );
                string(0, MAXPOINTS.max_x - (parsed.len() as i32), &parsed);
                let array_item = getter::get_attr_as_same_from_vec(&data_array, item);
                let value = getter::get_attr_as_same(array_item, "id").to_string();
                let id = value.trim_matches('"');

                let message = format!(" ({}) Found chapter with id: {}", item as i32, id);
                if ARGS.web || ARGS.gui || ARGS.check || ARGS.update {
                    info!("@{} {}", handle_id, message);
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
                            let mut dates = match resolute::CHAPTER_DATES.lock() {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(error::mdown::Error::PoisonError(err.to_string()));
                                }
                            };
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
                                            (
                                                match resolute::CHAPTERS_TO_REMOVE.lock() {
                                                    Ok(value) => value,
                                                    Err(err) => {
                                                        return Err(
                                                            error::mdown::Error::PoisonError(
                                                                err.to_string()
                                                            )
                                                        );
                                                    }
                                                }
                                            ).push(
                                                resolute::ChapterMetadata::new(
                                                    chapter_num,
                                                    &cur_date,
                                                    id
                                                )
                                            );
                                        }
                                    } else if datetime_cur > datetime {
                                        (
                                            match resolute::FIXED_DATES.lock() {
                                                Ok(value) => value,
                                                Err(err) => {
                                                    return Err(
                                                        error::mdown::Error::PoisonError(
                                                            err.to_string()
                                                        )
                                                    );
                                                }
                                            }
                                        ).push(chapter_num.to_string());
                                        (
                                            match resolute::CHAPTERS_TO_REMOVE.lock() {
                                                Ok(value) => value,
                                                Err(err) => {
                                                    return Err(
                                                        error::mdown::Error::PoisonError(
                                                            err.to_string()
                                                        )
                                                    );
                                                }
                                            }
                                        ).push(
                                            resolute::ChapterMetadata::new(
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
                    *(match resolute::CURRENT_CHAPTER_PARSED.lock() {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(error::mdown::Error::PoisonError(err.to_string()));
                        }
                    }) += 1;
                    if
                        cont &&
                        (lang == language || language == "*") &&
                        chapter_num != "This is test"
                    {
                        (
                            match resolute::CHAPTERS.lock() {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(error::mdown::Error::PoisonError(err.to_string()));
                                }
                            }
                        ).push(resolute::ChapterMetadata::new(&chapter_num, update_date, id));
                        (moves, hist) = utils::skip(
                            utils::process_filename(&folder_path),
                            item,
                            moves,
                            hist.clone(),
                            handle_id.clone()
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
                if (lang == language || language == "*") && chapter_num != "This is test" {
                    if ARGS.check {
                        let dates = match resolute::CHAPTER_DATES.lock() {
                            Ok(value) => value,
                            Err(err) => {
                                return Err(error::mdown::Error::PoisonError(err.to_string()));
                            }
                        };
                        let empty = String::new();

                        let cur_date = match dates.get(chapter_num) {
                            Some(date) => date.to_owned(),
                            None => empty,
                        };
                        (
                            match resolute::CHAPTERS_TO_REMOVE.lock() {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(error::mdown::Error::PoisonError(err.to_string()));
                                }
                            }
                        ).push(resolute::ChapterMetadata::new(chapter_num, &cur_date, id));
                        drop(dates);
                    }
                    let update_date = getter::get_attr_as_str(chapter_attr, "updatedAt");
                    *(match resolute::CURRENT_CHAPTER_PARSED.lock() {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(error::mdown::Error::PoisonError(err.to_string()));
                        }
                    }) += 1;
                    if arg_offset > (times as i32) {
                        (moves, hist) = utils::skip_offset(item, moves, hist, handle_id.clone());
                        times += 1;
                        *(match resolute::CURRENT_CHAPTER_PARSED.lock() {
                            Ok(value) => value,
                            Err(err) => {
                                return Err(error::mdown::Error::PoisonError(err.to_string()));
                            }
                        }) += 1;
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
                    if ARGS.web || ARGS.gui || ARGS.check || ARGS.update {
                        info!("@{} {}", handle_id, message);
                    }
                    string(2, 0, &message);
                    if
                        !ARGS.check ||
                        !(
                            match resolute::CHAPTERS.lock() {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(error::mdown::Error::PoisonError(err.to_string()));
                                }
                            }
                        )
                            .iter()
                            .any(|chapter| chapter.number == chapter_num.to_string())
                    {
                        if ARGS.check {
                            match date_change {
                                true => {
                                    (
                                        match resolute::TO_DOWNLOAD_DATE.lock() {
                                            Ok(value) => value,
                                            Err(err) => {
                                                return Err(
                                                    error::mdown::Error::PoisonError(
                                                        err.to_string()
                                                    )
                                                );
                                            }
                                        }
                                    ).push(chapter_num.to_string());
                                }
                                false => {
                                    (
                                        match resolute::TO_DOWNLOAD.lock() {
                                            Ok(value) => value,
                                            Err(err) => {
                                                return Err(
                                                    error::mdown::Error::PoisonError(
                                                        err.to_string()
                                                    )
                                                );
                                            }
                                        }
                                    ).push(chapter_num.to_string());
                                }
                            }
                            continue;
                        }
                        match getter::get_chapter(id).await {
                            Ok(json) => {
                                match
                                    download_chapter(
                                        id,
                                        json,
                                        &manga_name,
                                        title,
                                        &vol,
                                        chapter_num,
                                        &filename,
                                        update_date,
                                        handle_id.clone()
                                    ).await
                                {
                                    Ok(()) => (),
                                    Err(err) => {
                                        error::handle_error(&err, String::from("chapter"));
                                    }
                                };
                            }
                            Err(err) => {
                                (
                                    match resolute::SUSPENDED.lock() {
                                        Ok(value) => value,
                                        Err(err) => {
                                            return Err(
                                                error::mdown::Error::PoisonError(err.to_string())
                                            );
                                        }
                                    }
                                ).push(err);
                            }
                        }
                        match resolute::resolve_group(array_item, manga_name).await {
                            Ok(()) => (),
                            Err(err) => {
                                error::handle_error(&err, String::from("group"));
                            }
                        }
                        utils::clear_screen(5);
                        string(
                            7,
                            0,
                            &format!("  Converting images to cbz files: {}.cbz", folder_path)
                        );
                        let file_name = filename.get_file_w_folder();
                        zip_func::to_zip(&folder_path, &file_name, handle_id.clone()).await;
                        match fs::remove_dir_all(folder_path.clone()) {
                            Ok(()) => (),
                            Err(err) => {
                                return Err(
                                    error::mdown::Error::IoError(err, Some(folder_path.clone()))
                                );
                            }
                        }

                        utils::clear_screen(2);
                        if ARGS.web || ARGS.gui || ARGS.check || ARGS.update {
                            (
                                match resolute::DOWNLOADED.lock() {
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(
                                            error::mdown::Error::PoisonError(err.to_string())
                                        );
                                    }
                                }
                            ).push(file_name);
                        } else {
                            downloaded.push(filename.get_file_w_folder_w_cwd());
                        }
                    }
                } else {
                    let message = format!(
                        "Skipping because of wrong language; found '{}', target '{}' ...",
                        lang,
                        language
                    );
                    string(2, 0, &format!("  {}", message));

                    if ARGS.web || ARGS.gui || ARGS.check || ARGS.update {
                        info!("@{}  ({}) {}", handle_id, item, message);
                    }

                    *(match resolute::CURRENT_CHAPTER_PARSED_MAX.lock() {
                        Ok(value) => value,
                        Err(err) => {
                            return Err(error::mdown::Error::PoisonError(err.to_string()));
                        }
                    }) -= 1;
                }
            }
        }
        _ => {
            eprintln!("JSON is not an object.");
        }
    }
    Ok(downloaded)
}

pub(crate) async fn download_chapter(
    id: &str,
    manga_json: String,
    manga_name: &str,
    title: &str,
    vol: &str,
    chapter: &str,
    filename: &utils::FileName,
    update_date: &str,
    handle_id: Box<str>
) -> Result<(), error::mdown::Error> {
    string(3, 0, &format!("  Downloading images in folder: {}:", filename.get_folder_name()));
    if ARGS.web || ARGS.gui || ARGS.check || ARGS.update {
        info!("@{} Downloading images in folder: {}", handle_id, filename.get_folder_name());
        let mut current_chapter = match resolute::CURRENT_CHAPTER.lock() {
            Ok(value) => value,
            Err(err) => {
                return Err(error::mdown::Error::PoisonError(err.to_string()));
            }
        };
        current_chapter.clear();
        current_chapter.push_str(&&filename.get_folder_name());
    }
    let json_value = match utils::get_json(&manga_json) {
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
                    let saver = getter::get_saver();
                    if let Some(images1) = data_array.get(saver.clone()).and_then(Value::as_array) {
                        let images_length = images1.len();

                        *(match resolute::CURRENT_PAGE_MAX.lock() {
                            Ok(value) => value,
                            Err(err) => {
                                return Err(error::mdown::Error::PoisonError(err.to_string()));
                            }
                        }) = images_length.clone() as u64;

                        if let Some(images) = data_array.get(saver) {
                            let lock_file = filename.get_lock();
                            let mut lock_file_inst = match File::create(lock_file.clone()) {
                                Ok(file) => file,
                                Err(err) => {
                                    return Err(
                                        error::mdown::Error::IoError(err, Some(lock_file.clone()))
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
                                        error::mdown::Error::IoError(err, Some(lock_file.clone()))
                                    );
                                }
                            };
                            let response_map: HashMap<&str, serde_json::Value> = [
                                (
                                    "name",
                                    serde_json::Value::String(
                                        (
                                            match resolute::MANGA_NAME.lock() {
                                                Ok(value) => value,
                                                Err(err) => {
                                                    return Err(
                                                        error::mdown::Error::PoisonError(
                                                            err.to_string()
                                                        )
                                                    );
                                                }
                                            }
                                        ).to_string()
                                    ),
                                ),
                                (
                                    "id",
                                    serde_json::Value::String(
                                        (
                                            match MANGA_ID.lock() {
                                                Ok(value) => value,
                                                Err(err) => {
                                                    return Err(
                                                        error::mdown::Error::PoisonError(
                                                            err.to_string()
                                                        )
                                                    );
                                                }
                                            }
                                        ).to_string()
                                    ),
                                ),
                                (
                                    "saver",
                                    serde_json::Value::String(
                                        (
                                            match resolute::SAVER.lock() {
                                                Ok(value) => value,
                                                Err(err) => {
                                                    return Err(
                                                        error::mdown::Error::PoisonError(
                                                            err.to_string()
                                                        )
                                                    );
                                                }
                                            }
                                        ).to_string()
                                    ),
                                ),
                            ]
                                .iter()
                                .cloned()
                                .collect();
                            let json = match serde_json::to_string(&response_map) {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(error::mdown::Error::JsonError(err.to_string()));
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
                            let start = MAXPOINTS.max_x / 3 - (images_length as i32) / 2;

                            let iter = match ARGS.max_consecutive.parse() {
                                Ok(x) => x,
                                Err(_err) => {
                                    (
                                        match resolute::SUSPENDED.lock() {
                                            Ok(value) => value,
                                            Err(err) => {
                                                return Err(
                                                    error::mdown::Error::PoisonError(
                                                        err.to_string()
                                                    )
                                                );
                                            }
                                        }
                                    ).push(
                                        error::mdown::Error::ConversionError(
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
                                    let manga_name = Arc::from(manga_name);
                                    let title = Arc::from(title);
                                    let vol = Arc::from(vol);
                                    let chapter = Arc::from(chapter);
                                    let image = Arc::from(image_temp.trim_matches('"'));
                                    let handle_id_tmp = handle_id.clone();
                                    let image_base_url = Arc::from(image_base_url);

                                    tokio::spawn(async move {
                                        match
                                            download::download_image(
                                                image_base_url,
                                                chapter_hash,
                                                image,
                                                manga_name,
                                                title,
                                                vol,
                                                chapter,
                                                item,
                                                start,
                                                iter,
                                                i,
                                                handle_id_tmp
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
                                if match IS_END.lock() {
                                            Ok(value) => *value,
                                            Err(err) => {
                                                return Err(
                                                    error::mdown::Error::PoisonError(
                                                        err.to_string()
                                                    )
                                                );
                                            }
                                        }
                                        {
                                            *(match IS_END.lock() {
                                                Ok(value) => value,
                                                Err(err) => {
                                                    return Err(
                                                        error::mdown::Error::PoisonError(
                                                            err.to_string()
                                                        )
                                                    );
                                                }
                                            }) = false;
                                            return Ok(());
                                        }
                            }

                            *(match resolute::CURRENT_PAGE.lock() {
                                Ok(value) => value,
                                Err(err) => {
                                    return Err(error::mdown::Error::PoisonError(err.to_string()));
                                }
                            }) = 0;

                            let chapter_met = resolute::ChapterMetadata::new(
                                chapter,
                                update_date,
                                id
                            );
                            (
                                match resolute::CHAPTERS.lock() {
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(
                                            error::mdown::Error::PoisonError(err.to_string())
                                        );
                                    }
                                }
                            ).push(chapter_met);

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
