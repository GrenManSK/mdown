use chrono::DateTime;
use crosscurses::stdscr;
use glob::glob;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use serde_json::Value;
use std::{ env, fs::{ self, File }, io::Write, process::exit, sync::Arc };

mod args;
mod db;
mod download;
mod error;
mod getter;
mod macros;
mod metadata;
mod resolute;
mod utils;
mod zip_func;

#[cfg(feature = "music")]
mod music;

#[cfg(feature = "gui")]
mod gui;

#[cfg(feature = "server")]
mod server;

#[cfg(feature = "web")]
mod web;

fn string(y: u32, x: u32, value: &str) {
    if
        !*args::ARGS_WEB &&
        !*args::ARGS_GUI &&
        !*args::ARGS_CHECK &&
        !*args::ARGS_UPDATE &&
        !*args::ARGS_QUIET
    {
        stdscr().mvaddnstr(y as i32, x as i32, value, (MAXPOINTS.max_x - x) as i32);
        stdscr().refresh();
    }
}

fn log_end(handle_id: Box<str>) {
    resolute::HANDLE_ID_END.lock().push(handle_id);
}

lazy_static! {
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
    match utils::remove_cache() {
        Ok(()) => (),
        Err(_err) => (),
    }
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
        crosscurses::echo();
        crosscurses::cbreak();
    }
    if *resolute::FINAL_END.lock() {
        exit(0);
    }
}

async fn start() -> Result<(), error::MdownError> {
    let folder = match db::setup_settings() {
        Ok(folder) => folder,
        Err(_err) => args::ARGS.lock().folder.clone(),
    };

    args::ARGS.lock().change("folder", args::Value::Str(folder));

    if *args::ARGS_ENCODE != "" {
        debug!("Start web");
        #[cfg(feature = "web")]
        println!("{}", web::encode(&*args::ARGS_ENCODE));
        #[cfg(not(feature = "web"))]
        println!("Encode is not supported; You have to enable web feature");
        return Ok(());
    }

    if *args::ARGS_RESET {
        debug!("args_reset");
        return match utils::reset() {
            Ok(()) => Ok(()),
            Err(err) => Err(err),
        };
    }

    match db::init().await {
        Ok(()) => (),
        Err(err) => {
            return Err(err);
        }
    }

    // cwd
    match env::set_current_dir(args::ARGS_CWD.as_str()) {
        Ok(()) => (),
        Err(err) => {
            return Err(error::MdownError::IoError(err, args::ARGS_CWD.to_string()));
        }
    }

    debug!("cwd set to {}", *args::ARGS_CWD);

    if *args::ARGS_DELETE {
        return match resolute::args_delete() {
            Ok(()) => Ok(()),
            Err(err) => Err(err),
        };
    }

    if *args::ARGS_SHOW_LOG {
        debug!("show_log");
        return match resolute::show_log().await {
            Ok(()) => Ok(()),
            Err(err) => Err(err),
        };
    }

    match utils::create_cache_folder() {
        Ok(()) => (),
        Err(err) => {
            return Err(err);
        }
    }
    debug!("created cache folder");

    if args::ARGS_MUSIC.is_some() {
        debug!("music");
        #[cfg(feature = "music")]
        tokio::spawn(async { music::start() });
        #[cfg(not(feature = "music"))]
        eprintln!("Music feature is not enabled; You have to enable music feature");
    }

    // subscriber
    if
        *args::ARGS_WEB ||
        *args::ARGS_GUI ||
        *args::ARGS_UPDATE ||
        *args::ARGS_LOG ||
        *args::ARGS_SERVER
    {
        tokio::spawn(async { utils::log_handler() });
        match utils::setup_subscriber() {
            Ok(()) => (),
            Err(err) => {
                return Err(err);
            }
        }
        debug!("setup subscriber");
    }

    *resolute::LANGUAGE.lock() = args::ARGS.lock().lang.clone();

    if args::ARGS_SHOW.is_some() || args::ARGS_SHOW_ALL.is_some() {
        debug!("show || show all");
        match resolute::show().await {
            Ok(()) => (),
            Err(err) => {
                return Err(err);
            }
        }

        return Ok(());
    }

    if *args::ARGS_CHECK || *args::ARGS_UPDATE {
        debug!("start resolve_check");
        match resolute::resolve_check().await {
            Ok(()) => (),
            Err(err) => {
                return Err(err);
            }
        }

        return Ok(());
    }

    if *args::ARGS_SERVER {
        debug!("start server");
        #[cfg(feature = "server")]
        return match server::start() {
            Ok(()) => Ok(()),
            Err(err) => Err(err),
        };
        #[cfg(not(feature = "server"))]
        {
            println!("Server is not supported");
            *resolute::ENDED.lock() = true;
            return Ok(());
        }
    }

    //gui
    if *args::ARGS_GUI {
        debug!("start gui");
        #[cfg(feature = "gui")]
        return match gui::start() {
            Ok(()) => Ok(()),
            Err(err) => Err(err),
        };
        #[cfg(not(feature = "gui"))]
        {
            println!("Gui is not supported");
            *resolute::ENDED.lock() = true;
            return Ok(());
        }
    }

    // web
    if *args::ARGS_WEB {
        debug!("start web");
        #[cfg(feature = "web")]
        return match web::start().await {
            Ok(()) => Ok(()),
            Err(err) => Err(err),
        };
        #[cfg(not(feature = "web"))]
        {
            println!("Web is not supported");
            *resolute::ENDED.lock() = true;
            return Ok(());
        }
    }

    let file_path = match utils::resolve_start() {
        Ok(file_path) => file_path,
        Err(err) => {
            return Err(err);
        }
    };

    if !*args::ARGS_QUIET {
        debug!("start crosscurses");
        utils::setup_requirements(file_path.clone());
    }

    let mut manga_name = String::from("!");
    let mut status_code = match reqwest::StatusCode::from_u16(200) {
        Ok(code) => code,
        Err(err) => {
            return Err(
                error::MdownError::CustomError(err.to_string(), String::from("InvalidStatusCode"))
            );
        }
    };

    let url = args::ARGS.lock().url.clone();

    let id;

    if args::ARGS.lock().search != String::from("*") {
        debug!("using search");
        id = match utils::search().await {
            Ok(id) => id,
            Err(err) => {
                return Err(err);
            }
        };
    } else if let Some(id_temp) = utils::resolve_regex(&url) {
        debug!("using whole url");
        if utils::is_valid_uuid(id_temp.as_str()) {
            id = id_temp.as_str().to_string();
        } else {
            string(3, 0, &format!("Wrong format of UUID ({})", id_temp.as_str()));
            string(4, 0, "Should be 8-4-4-4-12 (123e4567-e89b-12d3-a456-426614174000)");
            id = String::from("*");
        }
    } else if utils::is_valid_uuid(&args::ARGS.lock().url) {
        debug!("using uuid");
        id = args::ARGS.lock().url.clone();
    } else if url == "UNSPECIFIED" {
        id = String::from("*");
    } else {
        string(3, 0, &format!("Wrong format of UUID ({})", url));
        string(4, 0, "Should be 8-4-4-4-12 (123e4567-e89b-12d3-a456-426614174000)");
        id = String::from("*");
    }
    if id != String::from("*") {
        debug!("id acquired");
        *resolute::MANGA_ID.lock() = id.clone();
        string(0, 0, &format!("Extracted ID: {}", id));
        string(1, 0, &format!("Getting manga information ..."));
        match getter::get_manga_json(&id).await {
            Ok(manga_name_json) => {
                string(1, 0, &format!("Getting manga information DONE"));
                *resolute::MUSIC_STAGE.lock() = String::from("init");
                let json_value = match utils::get_json(&manga_name_json) {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(err);
                    }
                };
                if let Value::Object(obj) = json_value {
                    debug!("parsed manga information");
                    manga_name = match resolute::resolve(obj, &id).await {
                        Ok(value) => value,
                        Err(err) => {
                            handle_error!(&err, String::from("program"));
                            String::from("!")
                        }
                    };
                } else {
                    return Err(error::MdownError::JsonError(String::from("Unexpected JSON value")));
                }
            }
            Err(code) => {
                string(1, 0, &format!("Getting manga information ERROR"));
                let code = code.into();
                let parts: Vec<&str> = code.split_whitespace().collect();

                if let Some(status_code_tmp) = parts.get(0) {
                    status_code = match
                        reqwest::StatusCode::from_u16(match status_code_tmp.parse::<u16>() {
                            Ok(code) => code,
                            Err(_err) => 0,
                        })
                    {
                        Ok(code) => code,
                        Err(err) => {
                            return Err(
                                error::MdownError::CustomError(
                                    err.to_string(),
                                    String::from("InvalidStatusCode")
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

    match utils::resolve_end(&file_path, &manga_name, status_code) {
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
    *resolute::CURRENT_CHAPTER_PARSED.lock() = 0;
    let folder = getter::get_folder_name(manga_name);
    let volume = args::ARGS.lock().volume.clone();
    let chapter = args::ARGS.lock().chapter.clone();
    let arg_volume = getter::get_arg(&volume);
    let arg_chapter = getter::get_arg(&chapter);
    let arg_offset: u32 = match getter::get_arg(&args::ARGS.lock().offset).parse() {
        Ok(value) => value,
        Err(_err) => 0,
    };
    let (mut downloaded, mut hist) = (vec![], &mut vec![]);
    let (mut times, mut moves) = (0, 0);
    let language = resolute::LANGUAGE.lock().clone();
    let mut filename;
    let json_value = match utils::get_json(&manga_json) {
        Ok(value) => value,
        Err(err) => {
            return Err(err);
        }
    };
    let mut all_ids = vec![];

    debug!("checking for .cbz files");

    if let Ok(value) = glob("*.cbz") {
        for entry in value.filter_map(Result::ok) {
            if let Some(entry) = entry.to_str() {
                debug!("found entry in glob: {}", entry);
                if let Ok(manga_id) = resolute::check_for_metadata(entry) {
                    all_ids.push(manga_id.id.clone());
                }
            }
        }
    }

    match serde_json::from_value::<metadata::MangaResponse>(json_value) {
        Ok(obj) => {
            debug!("parsed manga data");
            let data_array = utils::sort(&obj.data);
            let data_len = data_array.len();
            *resolute::CURRENT_CHAPTER_PARSED_MAX.lock() = data_len as u64;
            for item in 0..data_len {
                debug!("parsing chapter entry {}", item);
                let mut date_change = false;
                let parsed = format!(
                    "   Parsed chapters: {}/{}",
                    resolute::CURRENT_CHAPTER_PARSED.lock(),
                    resolute::CURRENT_CHAPTER_PARSED_MAX.lock()
                );
                if
                    !*args::ARGS_WEB &&
                    !*args::ARGS_GUI &&
                    !*args::ARGS_CHECK &&
                    !*args::ARGS_UPDATE
                {
                    string(0, MAXPOINTS.max_x - (parsed.len() as u32), &parsed);
                }
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

                let (chapter_attr, lang, pages, chapter_num, mut title) =
                    getter::get_metadata(array_item);

                title = resolute::title(title);

                let vol = match
                    (
                        match chapter_attr.volume {
                            Some(value) => value,
                            None => String::new(),
                        }
                    ).as_str()
                {
                    "" => String::new(),
                    value => format!("Vol.{} ", value),
                };

                let con_chap = resolute::resolve_skip(arg_chapter, &chapter_num);
                let con_vol = resolute::resolve_skip(arg_volume, &vol);

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
                    fs::metadata(filename.get_file_w_folder()).is_ok() &&
                    !arg_force &&
                    !(match resolute::check_for_metadata_saver(&filename.get_file_w_folder()) {
                        Ok(metadata) => if !*args::ARGS_CHECK { metadata } else { false } //
                        Err(err) => {
                            return Err(err);
                        }
                    }) &&
                    ({
                        if *args::ARGS_CHECK {
                            let chapter_ids = resolute::CHAPTER_IDS.lock();
                            let data_id = match chapter_ids.get(&chapter_num) {
                                Some(id) => id,
                                None => &String::new(),
                            };
                            if data_id != id && *data_id != String::new() {
                                false
                            } else {
                                true
                            }
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
                                    if datetime_cur < datetime {
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
                                    } else if datetime_cur > datetime {
                                        debug!(
                                            "dates didn't match bu date in local database was ahead of the date in mangadex database"
                                        );
                                        resolute::FIXED_DATES.lock().push(chapter_num.to_string());
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
                                Err(_err) => (),
                            }
                            drop(dates);
                        }
                        Err(_err) => (),
                    }
                    *resolute::CURRENT_CHAPTER_PARSED.lock() += 1;
                    if
                        cont &&
                        (lang == language || language == "*")
                    {
                        resolute::CHAPTERS
                            .lock()
                            .push(metadata::ChapterMetadata::new(&chapter_num, &update_date, id));
                        moves = utils::skip(folder_path, item, moves, &mut hist);
                        continue;
                    }
                }

                if con_vol {
                    debug!("skipping because volume didn't match");
                    moves = utils::skip_didnt_match("volume", item, moves, &mut hist);
                    continue;
                }
                if con_chap {
                    debug!("skipping because chapter didn't match");
                    moves = utils::skip_didnt_match("chapter", item, moves, &mut hist);
                    continue;
                }
                if pages == 0 {
                    debug!(
                        "skipping because variable pages is 0; probably because chapter is not supported on mangadex, third party"
                    );
                    moves = utils::skip_custom("pages is 0", item, moves, &mut hist);
                    continue;
                }
                if
                    (lang == language || language == "*") &&
                    !resolute::CHAPTERS
                        .lock()
                        .iter()
                        .any(|item| item.number == chapter_num) &&
                    !all_ids.contains(&id_string)
                {
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
                        moves = utils::skip_offset(item, moves, hist);
                        times += 1;
                        *resolute::CURRENT_CHAPTER_PARSED.lock() += 1;
                        continue;
                    }
                    utils::clear_screen(2);
                    let folder_path_tmp = &filename.get_folder_w_end();
                    let folder_path = folder_path_tmp.as_str();
                    let message = format!(
                        "  Metadata: Language: {};Pages: {};{};Chapter: {}{}",
                        lang,
                        pages,
                        vol,
                        chapter_num,
                        match title.as_str() {
                            "" => String::new(),
                            _ => format!(";Title: {}", title),
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
                    if
                        !*args::ARGS_CHECK ||
                        !resolute::CHAPTERS
                            .lock()
                            .iter()
                            .any(|chapter| chapter.number == chapter_num.to_string())
                    {
                        if *args::ARGS_CHECK {
                            debug!("was added to to download list because check flag is set");
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
                                handle_error!(&err, String::from("group"));
                                (String::from("null"), String::from("null"))
                            }
                        };
                        debug!("found chapter's scanlation group: {} {}", name, website);
                        let (name, website) = (name.as_str(), website.as_str());
                        match getter::get_chapter(id).await {
                            Ok(json) => {
                                let json_value = match utils::get_json(&json) {
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(err);
                                    }
                                };
                                let obj = match
                                    serde_json::from_value::<metadata::ChapterData>(json_value)
                                {
                                    Ok(value) => value,
                                    Err(err) => {
                                        return Err(error::MdownError::JsonError(err.to_string()));
                                    }
                                };
                                *resolute::MUSIC_STAGE.lock() = String::from("start");
                                debug!("starting to download chapter");
                                match
                                    download_chapter(
                                        id,
                                        obj,
                                        array_item,
                                        &manga_name,
                                        &title,
                                        &vol,
                                        &chapter_num,
                                        &filename,
                                        &update_date,
                                        &name,
                                        &website
                                    ).await
                                {
                                    Ok(()) => (),
                                    Err(err) => {
                                        handle_error!(&err, String::from("chapter"));
                                    }
                                };
                            }
                            Err(err) => {
                                resolute::SUSPENDED.lock().push(err);
                            }
                        }
                        if *IS_END.lock() {
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
                        zip_func::to_zip(folder_path, &file_name);
                        match fs::remove_dir_all(folder_path) {
                            Ok(()) => (),
                            Err(err) => {
                                return Err(
                                    error::MdownError::IoError(err, folder_path.to_string())
                                );
                            }
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
                    string(2, 0, &format!("{}", " ".repeat(MAXPOINTS.max_x as usize)));
                    let message = format!(
                        "Skipping because of wrong language; found '{}', target '{}' ...",
                        lang,
                        language
                    );
                    string(2, 0, &format!("  {}", message));

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
        }
        Err(err) => {
            return Err(error::MdownError::JsonError(err.to_string()));
        }
    }
    if *args::ARGS_DEBUG {
        match utils::debug_print(hist, "hist.txt") {
            Ok(()) => (),
            Err(_err) => (),
        };
    }
    Ok(downloaded)
}

pub(crate) async fn download_chapter(
    id: &str,
    obj: metadata::ChapterData,
    manga_json: &metadata::ChapterResponse,
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
    let image_base_url = obj.baseUrl.clone();
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
            metadata::Saver::dataSaver =>
                match data_array.dataSaver {
                    Some(data) => data,
                    None => Vec::new(),
                }
        };
    }
    let images_length = images.len();

    *resolute::CURRENT_PAGE.lock() = 0;
    *resolute::CURRENT_PAGE_MAX.lock() = images_length.clone() as u64;

    let lock_file = filename.get_lock();
    let mut lock_file_inst = match File::create(&lock_file) {
        Ok(file) => file,
        Err(err) => {
            return Err(error::MdownError::IoError(err, lock_file.clone()));
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
            return Err(error::MdownError::IoError(err, lock_file.clone()));
        }
    };
    let attr = manga_json.attributes.clone();

    let pages = attr.pages.to_string();

    let scanlation = metadata::ScanlationMetadata::new(name, website);
    let response_map = metadata::ChapterMetadataIn::new(
        resolute::MANGA_NAME.lock().to_string(),
        id.to_string(),
        resolute::MANGA_ID.lock().to_string(),
        *resolute::SAVER.lock(),
        title.to_string(),
        pages,
        chapter.to_string(),
        vol.to_string(),
        scanlation
    );

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

    debug!("metadata file created successfully");

    let lock_file_wait = filename.get_folder_name();

    tokio::spawn(async move { utils::wait_for_end(&lock_file_wait, images_length).await });
    let start = if MAXPOINTS.max_x / 3 < (images_length as u32) / 2 {
        1
    } else {
        MAXPOINTS.max_x / 3 - (images_length as u32) / 2
    };

    let iter = match args::ARGS.lock().max_consecutive.parse() {
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
            let image_temp = getter::get_attr_as_same_as_index(&images, item).to_string();
            let chapter_hash = Arc::from(chapter_hash.clone());
            let saver = Arc::from(match saver {
                metadata::Saver::data => "data",
                metadata::Saver::dataSaver => "data-saver",
            });
            let image = Arc::from(image_temp.trim_matches('"'));
            let image_base_url = Arc::from(image_base_url.clone());
            let page = item + 1;
            let page_str = page.to_string() + &" ".repeat(3 - page.to_string().len());

            let pr_title = match title != "" {
                true => format!(" - {}", title),
                false => String::new(),
            };
            let folder_name = utils::process_filename(
                &format!("{} - {}Ch.{}{}", manga_name, vol, chapter, pr_title)
            );
            let file_name = utils::process_filename(
                &format!("{} - {}Ch.{}{} - {}.jpg", manga_name, vol, chapter, pr_title, page)
            );
            let file_name_brief = utils::process_filename(
                &format!("{}Ch.{} - {}.jpg", vol, chapter, page)
            );

            let lock_file = format!(".cache\\{}.lock", folder_name);
            let full_path = format!(".cache/{}/{}", folder_name, file_name);

            tokio::spawn(async move {
                match
                    download::download_image(
                        image_base_url,
                        chapter_hash,
                        image,
                        page,
                        &page_str,
                        &folder_name,
                        &file_name_brief,
                        &lock_file,
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
