use glob::glob;
use image::load_from_memory;
use lazy_static::lazy_static;
use macroquad::{
    color::{Color, BLACK, GRAY, LIGHTGRAY, WHITE},
    hash,
    input::{
        is_key_down, is_key_pressed, is_mouse_button_pressed, is_quit_requested, mouse_position,
        prevent_quit, KeyCode, MouseButton,
    },
    math::{vec2, Rect, Vec2},
    shapes::{draw_rectangle, draw_rectangle_lines},
    text::{draw_text, measure_text},
    texture::{draw_texture_ex, Image, Texture2D},
    ui::{root_ui, widgets::InputText, widgets::Window as UiWindow, Ui},
    window::{clear_background, next_frame, screen_height, screen_width},
};
use parking_lot::Mutex;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    io::BufReader,
    ops::ControlFlow,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{info, warn};

use crate::{
    args::{self, ARGS},
    debug,
    error::{self, MdownError},
    getter, handle_error, metadata, resolute, utils,
    version_manager::get_current_version,
    zip_func,
};

lazy_static! {
    pub(crate) static ref CURRENT_CHAPTER: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref READER_CURRENT_CHAPTER_ID: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref READER_CHAPTER_PATHS: Mutex<Option<HashMap<String, String>>> =
        Mutex::new(None);
}

include!(concat!(env!("OUT_DIR"), "/loading_gif.rs"));

const NUM_OF_PRELOADS: usize = 10;

/// Height of the always on top menu bar, tall enough for one small button.
const MENU_BAR_HEIGHT: f32 = 32.0;
/// Height of a single entry inside the menu dropdown.
const MENU_ENTRY_HEIGHT: f32 = 24.0;
/// Number of entries in the menu dropdown, used to keep panels clear of it.
const MENU_ENTRY_COUNT: f32 = 3.0;
/// Panels are laid out below the menu bar and its dropdown.
const PANEL_TOP: f32 = MENU_BAR_HEIGHT + MENU_ENTRY_COUNT * MENU_ENTRY_HEIGHT;

const FONT_SMALL: f32 = 14.0;
const FONT_NORMAL: f32 = 18.0;
const FONT_HEADING: f32 = 24.0;
/// `measure_text` takes a separate scale factor which `draw_text` leaves at 1.0.
const FONT_SCALE: f32 = 1.0;

const BACKGROUND: Color = Color::from_rgba(32, 32, 36, 255);
const MENU_BAR_COLOR: Color = Color::from_rgba(46, 46, 52, 255);
const BUTTON_COLOR: Color = Color::from_rgba(62, 62, 70, 255);
const BUTTON_HOVER_COLOR: Color = Color::from_rgba(86, 86, 100, 255);
const BUTTON_BORDER: Color = Color::from_rgba(110, 110, 125, 255);
const OVERLAY: Color = Color::from_rgba(0, 0, 0, 153);

pub(crate) fn start() -> Result<(), MdownError> {
    if let Err(err) = app() {
        eprintln!("Error gui: {}", err);
    }

    match utils::remove_cache() {
        Ok(()) => (),
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 14002));
        }
    }
    *resolute::FINAL_END.lock() = true;
    Ok(())
}

/// Initializes and runs the GUI application using `macroquad`.
///
/// Creates the window, hands control to the frame loop owned by macroquad and blocks
/// until the window is closed.
///
/// # Returns
/// - `Ok(())` if the GUI ran and was closed successfully.
pub(crate) fn app() -> Result<(), MdownError> {
    info!("Setting up options");
    info!("Starting gui");

    let config = macroquad::conf::Conf {
        miniquad_conf: macroquad::window::Conf {
            window_title: format!("mdown v{}", get_current_version()),
            window_width: 500,
            window_height: 600,
            high_dpi: false,
            ..Default::default()
        },
        ..Default::default()
    };

    macroquad::Window::from_config(config, async {
        // Handle window close requests ourselves so a confirmation can be shown.
        prevent_quit();

        let mut app = App::new();

        loop {
            clear_background(BACKGROUND);

            if !app.frame() {
                break;
            }

            next_frame().await;
        }
    });

    Ok(())
}

#[derive(Default)]
struct App {
    exit_show_confirmation_dialog: bool,
    exit_allowed_to_close: bool,
    panel: String,
    menu_open: bool,
    setup_url: String,
    setup_lang: String,
    setup_offset: String,
    setup_database_offset: String,
    setup_title: String,
    setup_folder: String,
    setup_volume: String,
    setup_chapter: String,
    setup_max_consecutive: String,
    setup_saver: bool,
    setup_stat: bool,
    setup_force: bool,
    download_texture: Option<Texture2D>,
    main_done_downloading: Option<String>,
    panel_show_heading: bool,
    reader_title_animation_state: Option<(Instant, String)>,
    reader_manga_data: Option<metadata::MangaMetadata>,
    reader_id: Option<metadata::ChapterMetadata>,
    reader_page: usize,
    reader_chapter_path: Option<String>,
    reader_chapter_len: Option<usize>,
    reader_chapters: Vec<metadata::ChapterMetadata>,
    /// Uploaded page textures, only ever touched on the gui thread.
    reader_texture_cache: HashMap<usize, Texture2D>,
    /// Decoded pages waiting to be uploaded to the gpu by the gui thread.
    reader_pending_pages: Arc<Mutex<HashMap<usize, Image>>>,
    /// Pages currently being extracted and decoded by background tasks.
    reader_loading_pages: Arc<Mutex<HashSet<usize>>>,
    reader_hover_start_time: Option<Instant>,
    reader_click_start_time: Option<Instant>,
    reader_click_page: Option<usize>,
    gif_current_frame: usize,
    gif_last_update: Option<Instant>,
    gif_textures: HashMap<String, Vec<Texture2D>>,
}

impl App {
    fn new() -> Self {
        let setup_url = ARGS.lock().url.clone();
        let setup_lang = ARGS.lock().lang.clone();
        let setup_offset = ARGS.lock().offset.clone();
        let setup_database_offset = ARGS.lock().database_offset.clone();
        let setup_title = ARGS.lock().title.clone();
        let setup_folder = ARGS.lock().folder.clone();
        let setup_volume = ARGS.lock().volume.clone();
        let setup_chapter = ARGS.lock().chapter.clone();
        let setup_max_consecutive = ARGS.lock().max_consecutive.clone().to_string();
        let setup_saver = ARGS.lock().saver;
        let setup_stat = ARGS.lock().stat;
        let setup_force = ARGS.lock().force;
        // Textures may only be created on the thread that owns the rendering context.
        let gif_textures = load_all_gifs();
        Self {
            exit_allowed_to_close: false,
            exit_show_confirmation_dialog: false,
            panel: "main".to_owned(),
            menu_open: false,
            setup_url: match setup_url.as_str() {
                "UNSPECIFIED" => String::new(),
                value => value.to_owned(),
            },
            setup_lang,
            setup_offset,
            setup_database_offset,
            setup_title,
            setup_folder,
            setup_volume,
            setup_chapter,
            setup_max_consecutive,
            setup_saver,
            setup_stat,
            setup_force,
            main_done_downloading: None,
            download_texture: None,
            panel_show_heading: true,
            reader_title_animation_state: None,
            reader_manga_data: None,
            reader_id: None,
            reader_page: 0,
            reader_chapter_path: None,
            reader_chapters: Vec::new(),
            reader_texture_cache: HashMap::new(),
            reader_pending_pages: Arc::new(Mutex::new(HashMap::new())),
            reader_loading_pages: Arc::new(Mutex::new(HashSet::new())),
            reader_chapter_len: None,
            reader_hover_start_time: None,
            reader_click_start_time: None,
            reader_click_page: None,
            gif_current_frame: 0,
            gif_last_update: Some(Instant::now()),
            gif_textures,
        }
    }

    /// Renders a single frame and updates all interactive state.
    ///
    /// # Returns
    /// - `false` when the gui should be closed.
    fn frame(&mut self) -> bool {
        self.upload_pending_pages();

        if is_quit_requested() {
            if self.exit_allowed_to_close {
                return false;
            }
            self.exit_show_confirmation_dialog = true;
        }

        let width = screen_width();
        let height = screen_height();

        if self.exit_show_confirmation_dialog {
            draw_rectangle(0.0, 0.0, width, height, OVERLAY);
            if self.exit_dialog(width, height) {
                info!("Closing gui");
                return false;
            }
            return true;
        }

        {
            let mut ui = root_ui();

            if self.panel_show_heading {
                draw_text_centered(
                    &format!("mdown v{}", get_current_version()),
                    vec2(width / 2.0, MENU_BAR_HEIGHT + 20.0),
                    FONT_HEADING,
                    WHITE,
                );
            }

            self.main_panel(&mut ui, width, height);
        }

        // Drawn last so the menu stays usable on top of every panel.
        self.menu_bar(width);
        self.menu_dropdown();

        true
    }

    /// Switches to the given panel, resetting reader state when needed.
    fn select_panel(&mut self, panel: &str) {
        info!("Selected {}", panel);
        self.panel = panel.to_owned();
        self.reader_manga_data = None;
        self.panel_show_heading = panel != "reader";
        self.reader_full_reset();
    }

    /// Draws the menu bar and handles opening the dropdown.
    fn menu_bar(&mut self, width: f32) {
        draw_rectangle(0.0, 0.0, width, MENU_BAR_HEIGHT, MENU_BAR_COLOR);

        if draw_button(vec2(4.0, 3.0), "Menu", FONT_SMALL) {
            self.menu_open = !self.menu_open;
        }
    }

    /// Draws the menu dropdown. Called after all panels so it is never covered.
    fn menu_dropdown(&mut self) {
        if !self.menu_open {
            return;
        }

        let entries = [("Main", "main"), ("Help", "help"), ("Reader", "reader")];
        let entry_width = 96.0;
        let clicked = is_mouse_button_pressed(MouseButton::Left);

        for (index, (label, panel)) in entries.iter().enumerate() {
            let rect = Rect::new(
                4.0,
                MENU_BAR_HEIGHT + index as f32 * MENU_ENTRY_HEIGHT,
                entry_width,
                MENU_ENTRY_HEIGHT,
            );
            let hovered = rect.contains(mouse_pos());

            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                if hovered {
                    BUTTON_HOVER_COLOR
                } else {
                    BUTTON_COLOR
                },
            );
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, BUTTON_BORDER);
            draw_text_centered(label, rect.center(), FONT_SMALL, WHITE);

            if hovered && clicked {
                self.menu_open = false;
                self.select_panel(panel);
                return;
            }
        }
    }

    /// Decides which panel should be rendered.
    fn main_panel(&mut self, ui: &mut Ui, width: f32, height: f32) {
        match self.panel.as_str() {
            "reader" => self.reader(ui, width, height),
            "main" => self.main(ui, width, height),
            "help" => self.help(width),
            _ => (),
        }
    }

    /// Shows either the configuration form or the progress of an ongoing download.
    fn main(&mut self, ui: &mut Ui, width: f32, height: f32) {
        if !*resolute::DOWNLOADING.lock() {
            self.main_config(ui, width, height);
        } else {
            self.main_downloading(ui, width, height);
        }
    }

    /// Displays the current status of the ongoing download.
    fn main_downloading(&mut self, ui: &mut Ui, width: f32, height: f32) {
        let title = format!("Downloading {}", resolute::MANGA_NAME.lock());
        let chapter = format!("Chapter: {}", resolute::CURRENT_CHAPTER.lock());
        let size = format!(
            "[{:.2}mb/{:.2}mb]",
            resolute::CURRENT_SIZE.lock(),
            resolute::CURRENT_SIZE_MAX.lock()
        );
        let current_page = *resolute::CURRENT_PAGE.lock();
        let current_page_max = *resolute::CURRENT_PAGE_MAX.lock();
        let message = format!("Progress: [{}/{}]", current_page, current_page_max);
        let progress = "#".repeat(current_page as usize);
        let web_downloaded = resolute::WEB_DOWNLOADED.lock().clone();
        let scanlation_groups: Vec<String> = resolute::SCANLATION_GROUPS
            .lock()
            .iter()
            .map(|entry| entry.name.clone())
            .collect();

        if self.main_done_downloading.is_none() && !resolute::MANGA_ID.lock().is_empty() {
            self.main_done_downloading = Some(resolute::MANGA_ID.lock().clone());
        }

        let preview = self.refresh_preview_texture();
        let preview_size = preview.as_ref().and_then(|texture| {
            let size = texture.size();
            if size.x <= 0.0 {
                return None;
            }
            let scale = ((width - 40.0) / size.x).min(1.0);
            Some(vec2(size.x, size.y * scale))
        });

        UiWindow::new(
            hash!("downloading"),
            vec2(8.0, PANEL_TOP),
            panel_size(width, height),
        )
        .movable(false)
        .titlebar(false)
        .ui(ui, |ui| {
            ui.label(None, &title);
            ui.label(None, &chapter);
            ui.label(None, &size);
            ui.label(None, &message);
            ui.label(None, &progress);

            if !web_downloaded.is_empty() {
                ui.label(None, "Downloaded:");
                for entry in &web_downloaded {
                    ui.label(None, entry.as_str());
                }
            }

            if !scanlation_groups.is_empty() {
                ui.label(None, "Scanlation group:");
                for entry in &scanlation_groups {
                    ui.label(None, entry.as_str());
                }
            }

            if let (Some(texture), Some(dest)) = (preview.as_ref(), preview_size) {
                let mut canvas = ui.canvas();
                // `request_space` hands back coordinates relative to the window cursor.
                let relative = canvas.request_space(dest);
                let position = canvas.cursor() + relative;
                canvas.image(Rect::new(position.x, position.y, dest.x, dest.y), texture);
            }
        });
    }

    /// Refreshes the preview texture of the chapter that is currently downloading.
    fn refresh_preview_texture(&mut self) -> Option<Texture2D> {
        if self.download_texture.is_some()
            && std::fs::metadata(".cache\\preview\\preview.png").is_ok()
            && *resolute::CURRENT_CHAPTER.lock() != *CURRENT_CHAPTER.lock()
        {
            *CURRENT_CHAPTER.lock() = resolute::CURRENT_CHAPTER.lock().to_string();
            self.download_texture = None;
        }

        if self.download_texture.is_none() {
            if let Ok(img) = image::open(".cache\\preview\\preview.png") {
                let rgba = img.to_rgba8();
                self.download_texture = Some(Texture2D::from_rgba8(
                    rgba.width() as u16,
                    rgba.height() as u16,
                    rgba.as_raw(),
                ));
            }
        }

        self.download_texture.clone()
    }

    /// Displays the configuration panel used to start a download.
    fn main_config(&mut self, ui: &mut Ui, width: f32, height: f32) {
        UiWindow::new(
            hash!("config"),
            vec2(8.0, PANEL_TOP),
            panel_size(width, height),
        )
        .movable(false)
        .titlebar(false)
        .ui(ui, |ui| {
            InputText::new(hash!("config url"))
                .label("Set url of manga")
                .ui(ui, &mut self.setup_url);

            if let Some(id) = utils::resolve_regex(self.setup_url.as_str()) {
                ui.label(None, &format!("Found id: {}", id.as_str()));
                self.setup_url = id.as_str().to_string();
            } else if utils::is_valid_uuid(self.setup_url.as_str()) {
                let url = self.setup_url.clone();
                ui.label(None, &format!("Found id: {}", url));
            }

            InputText::new(hash!("config lang"))
                .label("Set language of manga")
                .ui(ui, &mut self.setup_lang);
            InputText::new(hash!("config offset"))
                .label("Set offset")
                .ui(ui, &mut self.setup_offset);
            InputText::new(hash!("config database offset"))
                .label("Set offset of database")
                .ui(ui, &mut self.setup_database_offset);
            InputText::new(hash!("config title"))
                .label("Set title of manga")
                .ui(ui, &mut self.setup_title);
            InputText::new(hash!("config folder"))
                .label("Set folder to put manga in")
                .ui(ui, &mut self.setup_folder);
            InputText::new(hash!("config volume"))
                .label("Set volume of manga")
                .ui(ui, &mut self.setup_volume);
            InputText::new(hash!("config chapter"))
                .label("Set chapter of manga")
                .ui(ui, &mut self.setup_chapter);
            InputText::new(hash!("config max consecutive"))
                .label("Set max consecutive of manga")
                .ui(ui, &mut self.setup_max_consecutive);

            ui.checkbox(hash!("config saver"), "Saver", &mut self.setup_saver);
            ui.checkbox(hash!("config stat"), "Statistics", &mut self.setup_stat);
            ui.checkbox(hash!("config force"), "Force", &mut self.setup_force);

            if ui.button(None, "Download") {
                self.start_download();
            }

            if let Some(downloaded_manga_id) = self.main_done_downloading.clone() {
                match get_manga_data() {
                    Ok(manga_list) => {
                        for manga in manga_list {
                            if manga.id == downloaded_manga_id
                                && ui.button(None, manga.name.as_str())
                            {
                                self.panel = String::from("reader");
                                self.reader_manga_data = Some(manga);
                                self.panel_show_heading = false;
                                self.reader_full_reset();
                            }
                        }
                    }
                    Err(err) => warn!("Error getting manga data: {}", err),
                }
            }

            if !resolute::WEB_DOWNLOADED.lock().is_empty() {
                ui.label(None, "Downloaded:");
                for entry in resolute::WEB_DOWNLOADED.lock().iter() {
                    ui.label(None, entry.as_str());
                }
            }

            if !resolute::SCANLATION_GROUPS.lock().is_empty() {
                ui.label(None, "Scanlation group:");
                for entry in resolute::SCANLATION_GROUPS.lock().iter() {
                    ui.label(None, entry.name.as_str());
                }
            }
        });
    }

    /// Reads the configuration form and spawns the download task.
    fn start_download(&mut self) {
        self.main_done_downloading = None;
        let handle_id = utils::generate_random_id(12);
        *ARGS.lock() = args::Args::from(
            self.setup_url.clone(),
            self.setup_lang.clone(),
            self.setup_title.clone(),
            self.setup_folder.clone(),
            self.setup_volume.clone(),
            self.setup_chapter.clone(),
            self.setup_saver,
            self.setup_stat,
            match self.setup_max_continuous() {
                Ok(max_consecutive) => max_consecutive,
                Err(()) => {
                    error::suspend_error(MdownError::ConversionError(
                        String::from("Failed to parse max_consecutive"),
                        14004,
                    ));
                    40
                }
            },
            self.setup_force,
            self.setup_offset.clone(),
            self.setup_database_offset.clone(),
        );
        let url = self.setup_url.clone();
        *resolute::SAVER.lock() = self.setup_saver;
        resolute::SCANLATION_GROUPS.lock().clear();
        drop(tokio::spawn(async move {
            match resolve_download(&url, handle_id).await {
                Ok(_) => (),
                Err(err) => handle_error!(&err, String::from("gui")),
            };
        }));
    }

    /// Parses the maximum consecutive pages setting.
    fn setup_max_continuous(&self) -> Result<usize, ()> {
        self.setup_max_consecutive.parse().map_err(|_| ())
    }

    /// Displays a short usage guide.
    fn help(&self, width: f32) {
        let mut y = PANEL_TOP;

        y += draw_heading("Downloader", width, y) + 20.0;
        y += draw_line_text("Write url and press download", width, y) + 20.0;
        y += draw_heading("Reader", width, y) + 20.0;
        y += draw_line_text("with left and right arrows you move pages by 1", width, y);
        draw_line_text("with up and down arrows you move pages by 5", width, y);
    }

    /// Handles the manga reader, either showing selection lists or the reader itself.
    fn reader(&mut self, ui: &mut Ui, width: f32, height: f32) {
        if let Some(chapter_id) = self.reader_id.clone() {
            self.reader_panel(width, height, chapter_id);
            return;
        }
        if let Some(manga_data) = self.reader_manga_data.clone() {
            self.reader_chapter_selection(ui, width, height, manga_data);
            return;
        }
        self.reader_manga_selection(ui, width, height);
    }

    /// Lists every downloaded manga so one of them can be picked for reading.
    fn reader_manga_selection(&mut self, ui: &mut Ui, width: f32, height: f32) {
        let mut back = false;

        UiWindow::new(
            hash!("manga selection"),
            vec2(8.0, PANEL_TOP),
            panel_size(width, height),
        )
        .movable(false)
        .titlebar(false)
        .ui(ui, |ui| {
            back = ui.button(None, "Back");
            ui.label(None, "Current downloaded manga");

            match get_manga_data() {
                Ok(manga_list) => {
                    for manga in manga_list {
                        if ui.button(None, manga.name.as_str()) {
                            info!("Selected {}", manga.name);
                            self.reader_manga_data = Some(manga.clone());
                        }
                    }
                }
                Err(err) => warn!("Error getting manga data: {}", err),
            }
        });

        if back {
            self.reader_full_reset();
            self.reader_manga_data = None;
            self.panel = String::from("main");
            self.panel_show_heading = true;
        }
    }

    /// Lists every chapter of the selected manga so one of them can be picked.
    fn reader_chapter_selection(
        &mut self,
        ui: &mut Ui,
        width: f32,
        height: f32,
        manga_data: metadata::MangaMetadata,
    ) {
        let mut back = false;
        let mut selected: Option<metadata::ChapterMetadata> = None;

        UiWindow::new(
            hash!("chapter selection"),
            vec2(8.0, PANEL_TOP),
            panel_size(width, height),
        )
        .movable(false)
        .titlebar(false)
        .ui(ui, |ui| {
            back = ui.button(None, "Back");
            ui.label(None, &format!("{} ({})", manga_data.name, manga_data.id));

            let mut chapters = manga_data.chapters.clone();
            chapters.sort_by_key(|chapter| chapter.parse_number());
            self.reader_chapters = chapters.clone();

            for chapter in chapters.iter() {
                if ui.button(None, chapter.number.as_str()) {
                    selected = Some(chapter.clone());
                }
            }
        });

        if back {
            self.reader_full_reset();
            self.reader_manga_data = None;
            return;
        }

        if let Some(chapter) = selected {
            self.reader_reset();
            self.reader_id = Some(chapter.clone());
            self.reader_title_animation_state = None;
            info!("Selected chapter id: {}", chapter.id.clone());
            *READER_CURRENT_CHAPTER_ID.lock() = chapter.id.clone();
        }

        if READER_CHAPTER_PATHS.lock().is_none() {
            info!("Reading files ...");
            get_chapter_paths(manga_data);
        }
    }

    /// Renders the currently selected chapter.
    fn reader_panel(&mut self, width: f32, height: f32, chapter_id: metadata::ChapterMetadata) {
        if draw_button(vec2(8.0, PANEL_TOP), "Back", FONT_SMALL) {
            self.reader_reset();
            return;
        }

        if let ControlFlow::Break(_) = self.reader_handle_input() {
            return;
        }

        let page_top = PANEL_TOP + MENU_ENTRY_HEIGHT;
        let page_bottom = height - 30.0;

        if let Some(file_path) = self.reader_chapter_path.clone() {
            self.reader_preload(file_path, width - 20.0, page_bottom - page_top);

            let center = vec2(width / 2.0, (page_top + page_bottom) / 2.0);

            if let Some(texture) = self.reader_texture_cache.get(&self.reader_page) {
                let size = texture.size();
                let position = vec2(center.x - size.x / 2.0, center.y - size.y / 2.0);
                draw_texture_ex(texture, position.x, position.y, WHITE, Default::default());
            } else if self.reader_is_loading(self.reader_page) {
                draw_text_centered("Loading page...", center, FONT_NORMAL, WHITE);
                self.show_gif(center);
            } else {
                draw_text_centered("Page not available", center, FONT_NORMAL, WHITE);
            }
        }

        self.reader_chap_number(width, height);
        self.reader_chap_title(width);
        self.reader_progress(width, height);
        self.request_chapter_path(&chapter_id);
        self.request_chapter_len();
    }

    /// Returns whether the given page is currently being fetched or decoded.
    fn reader_is_loading(&self, page: usize) -> bool {
        self.reader_pending_pages.lock().contains_key(&page)
            || self.reader_loading_pages.lock().contains(&page)
    }

    /// Uploads every decoded page into a gpu texture.
    ///
    /// Decoding happens on background tasks while texture creation has to happen on
    /// the gui thread, so finished pages are collected in `reader_pending_pages`.
    fn upload_pending_pages(&mut self) {
        let ready: Vec<(usize, Image)> = {
            let mut pending = self.reader_pending_pages.lock();
            pending.drain().collect()
        };

        for (page, image) in ready {
            let texture = Texture2D::from_rgba8(image.width, image.height, &image.bytes);
            self.reader_texture_cache.insert(page, texture);
        }
    }

    /// Requests the length (number of pages) of the current chapter.
    fn request_chapter_len(&mut self) {
        if self.reader_chapter_len.is_none() {
            if let Some(file_path) = self.reader_chapter_path.clone() {
                let chapter_len =
                    zip_func::extract_image_len_from_zip_gui(&file_path).unwrap_or_default();
                self.reader_chapter_len = Some(chapter_len);
            }
        }
    }

    /// Looks up the file path of the current chapter.
    fn request_chapter_path(&mut self, chapter_id: &metadata::ChapterMetadata) {
        if self.reader_chapter_path.is_none() {
            if let Some(paths) = READER_CHAPTER_PATHS.lock().clone() {
                if let Some(path) = paths.get(&chapter_id.id) {
                    self.reader_chapter_path = Some(path.to_string());
                    info!("Chapter path set to: {}", path);
                }
            }
        }
    }

    /// Loads the next chapter, if there is one.
    fn request_next_chapter(&mut self) -> bool {
        if let Some(current_chapter) = self.reader_id.clone() {
            if let Some(value) = current_chapter.get_next_chapter(&self.reader_chapters.clone()) {
                self.open_chapter(value.clone());
                return true;
            }
        }
        false
    }

    /// Loads the previous chapter, if there is one.
    fn request_previous_chapter(&mut self) -> bool {
        if let Some(current_chapter) = self.reader_id.clone() {
            if let Some(value) = current_chapter.get_previous_chapter(&self.reader_chapters.clone())
            {
                self.open_chapter(value.clone());
                return true;
            }
        }
        false
    }

    /// Makes the given chapter the one currently being read.
    fn open_chapter(&mut self, chapter: metadata::ChapterMetadata) {
        self.reader_reset();
        self.reader_id = Some(chapter.clone());
        self.reader_title_animation_state = None;
        self.request_chapter_path(&chapter);
        self.request_chapter_len();
        *READER_CURRENT_CHAPTER_ID.lock() = chapter.id.clone();
    }

    /// Spawns background tasks decoding the pages around the current one.
    ///
    /// Extraction and resizing run on tokio workers, only the decoded pixel data is
    /// handed back so the gui thread can upload it as a texture.
    fn reader_preload(&mut self, file_path: String, available_width: f32, available_height: f32) {
        let id = READER_CURRENT_CHAPTER_ID.lock().clone();

        for offset in (0..NUM_OF_PRELOADS)
            .flat_map(|n| [n as isize, -(n as isize)].into_iter())
            .filter_map(|off| ((self.reader_page as isize) + off).try_into().ok())
        {
            let page_to_load = offset;

            if self.reader_texture_cache.contains_key(&page_to_load)
                || self.reader_is_loading(page_to_load)
            {
                continue;
            }

            self.reader_loading_pages.lock().insert(page_to_load);

            let pending = self.reader_pending_pages.clone();
            let loading = self.reader_loading_pages.clone();
            let file_path = file_path.clone();
            let id = id.clone();

            tokio::spawn(async move {
                match zip_func::extract_image_from_zip_gui(&file_path, page_to_load + 1) {
                    Ok(image_data) => {
                        let image =
                            decode_and_resize_image(&image_data, available_width, available_height);
                        if id != *READER_CURRENT_CHAPTER_ID.lock() {
                            return;
                        }
                        if let Some(image) = image {
                            pending.lock().insert(page_to_load, image);
                            info!("Preloaded page {}", page_to_load);
                        }
                    }
                    Err(err) => match err {
                        MdownError::NotFoundError(..) => (),
                        err => warn!("Error loading page {}: {}", page_to_load, err),
                    },
                }
                loading.lock().remove(&page_to_load);
            });
        }
    }

    /// Handles keyboard input for navigating and controlling the reader.
    ///
    /// # Returns
    /// - `ControlFlow::Continue(())` when the reader should keep rendering.
    /// - `ControlFlow::Break(())` when the current chapter was left.
    fn reader_handle_input(&mut self) -> ControlFlow<()> {
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);

        if is_key_pressed(KeyCode::Right) {
            if let Some(chap_len) = self.reader_chapter_len {
                if ctrl {
                    if self.request_next_chapter() {
                        return ControlFlow::Break(());
                    }
                    self.reader_reset();
                    info!("Manga is finished");
                    return ControlFlow::Break(());
                } else if shift {
                    self.reader_page = chap_len - 1;
                    return ControlFlow::Continue(());
                } else if self.reader_page + 1 >= chap_len && chap_len != 0 {
                    if self.request_next_chapter() {
                        return ControlFlow::Break(());
                    }
                    self.reader_reset();
                    info!("Manga is finished");
                    return ControlFlow::Break(());
                }
            }
            self.reader_page += 1;
            self.download_texture = None;
            info!("Next page: {}", self.reader_page);
        } else if is_key_pressed(KeyCode::Left) {
            if ctrl {
                self.request_previous_chapter();
                self.reader_page = 0;
                return ControlFlow::Break(());
            } else if shift {
                self.reader_page = 0;
                return ControlFlow::Continue(());
            } else if self.reader_page == 0 {
                if self.request_previous_chapter() {
                    self.reader_page = self.reader_chapter_len.map(|len| len - 1).unwrap_or(0);
                }
            } else {
                self.reader_page -= 1;
                self.download_texture = None;
                info!("Previous page: {}", self.reader_page);
            }
        } else if is_key_pressed(KeyCode::Up) {
            if let Some(chap_len) = self.reader_chapter_len {
                if self.reader_page + 1 >= chap_len && chap_len != 0 {
                    if !self.request_next_chapter() {
                        self.reader_reset();
                        info!("Manga is finished");
                    }
                    return ControlFlow::Continue(());
                } else if self.reader_page + 5 >= chap_len && chap_len != 0 {
                    self.reader_page = chap_len - 1;
                    return ControlFlow::Continue(());
                }
            }
            self.reader_page += 5;
            self.download_texture = None;
            info!("Next page: {}", self.reader_page);
        } else if is_key_pressed(KeyCode::Down) {
            if self.reader_page == 0 {
                if self.request_previous_chapter() {
                    if let Some(chap_len) = self.reader_chapter_len {
                        self.reader_page = chap_len - 1;
                    }
                    return ControlFlow::Break(());
                }
                return ControlFlow::Continue(());
            } else if (self.reader_page as i32) - 5 < 0 {
                self.reader_page = 0;
                return ControlFlow::Continue(());
            }
            self.reader_page -= 5;
            self.download_texture = None;
            info!("Previous page: {}", self.reader_page);
        } else if is_key_pressed(KeyCode::R) {
            if shift {
                self.reader_texture_cache.clear();
                info!("Clearing the entire texture cache");
            } else {
                self.reader_texture_cache.remove(&self.reader_page);
                info!("Resetting page {}", self.reader_page);
            }
        } else if is_key_pressed(KeyCode::Q) {
            self.reader_reset();
            return ControlFlow::Break(());
        }

        ControlFlow::Continue(())
    }

    /// Clears every piece of reader state.
    fn reader_reset(&mut self) {
        self.reader_id = None;
        self.reader_page = 0;
        self.reader_chapter_path = None;
        self.reader_chapter_len = None;
        self.download_texture = None;
        self.reader_texture_cache.clear();
        self.reader_pending_pages.lock().clear();
        self.reader_loading_pages.lock().clear();
        self.reader_title_animation_state = None;
        self.reader_click_page = None;
        self.reader_click_start_time = None;
        *READER_CURRENT_CHAPTER_ID.lock() = String::new();
    }

    /// Clears the reader state including the chapter list and the chapter paths.
    fn reader_full_reset(&mut self) {
        self.reader_reset();
        self.reader_chapters.clear();
        *READER_CHAPTER_PATHS.lock() = None;
    }

    /// Draws the page progress bar and allows jumping between pages.
    fn reader_progress(&mut self, width: f32, height: f32) {
        let chapter_len = match self.reader_chapter_len {
            Some(len) if len != 0 => len,
            _ => return,
        };

        let segment_hover_margin = 10.0;
        let default_bar_height = 2.0;
        let expanded_bar_height = 10.0;
        let segment_width = width / chapter_len as f32;
        let mouse = mouse_pos();
        let clicked = is_mouse_button_pressed(MouseButton::Left);

        let hover_duration = self
            .reader_hover_start_time
            .map(|start| start.elapsed())
            .unwrap_or_default();
        let click_duration = self
            .reader_click_start_time
            .map(|start| start.elapsed())
            .unwrap_or_default();

        let bar_height = if hover_duration.as_secs_f32() < 0.25 {
            let progress = hover_duration.as_secs_f32() / 0.25;
            default_bar_height + (expanded_bar_height - default_bar_height) * progress
        } else {
            expanded_bar_height
        };

        let bar_top = height - bar_height;
        let inset = (expanded_bar_height - default_bar_height) / 2.0;
        draw_rectangle(
            inset,
            bar_top + inset,
            width - inset * 2.0,
            bar_height - inset * 2.0,
            Color::from_rgba(200, 200, 200, 255),
        );

        let clicked_page = self.reader_click_page;
        let click_elapsed = click_duration.as_secs_f32();
        let mut hovered_rect: Option<Rect> = None;

        for page_index in 0..chapter_len {
            let color = if page_index == self.reader_page {
                WHITE
            } else if self.reader_texture_cache.contains_key(&page_index) {
                GRAY
            } else {
                BLACK
            };

            // How many pixels should the segment be expanded by.
            let expansion = match clicked_page {
                Some(page) if page == page_index => {
                    let expansion = 20.0;
                    if click_elapsed < 0.5 {
                        click_elapsed / 0.5 * expansion
                    } else if click_elapsed < 1.0 {
                        expansion
                    } else if click_elapsed < 1.5 {
                        (1.0 - (click_elapsed - 1.0) / 0.5) * expansion
                    } else {
                        0.0
                    }
                }
                _ => 0.0,
            };

            let segment_rect = Rect::new(
                segment_width * page_index as f32,
                bar_top + (expanded_bar_height - default_bar_height - expansion) / 2.0,
                segment_width,
                (expanded_bar_height + default_bar_height + expansion) / 2.0,
            );
            let hover_rect = expand(segment_rect, segment_hover_margin);
            let hovered = hover_rect.contains(mouse);

            let painted_rect = if hovered {
                expand(segment_rect, 10.0)
            } else {
                segment_rect
            };

            draw_rectangle(
                painted_rect.x,
                painted_rect.y,
                painted_rect.w,
                painted_rect.h,
                if hovered { LIGHTGRAY } else { color },
            );

            if hovered && clicked && self.reader_page != page_index {
                self.reader_click_start_time = Some(Instant::now());
                self.reader_click_page = Some(page_index);
                self.reader_page = page_index;
                self.download_texture = None;
                info!("Jumped to page: {}", page_index);
            }

            if hovered {
                hovered_rect = Some(painted_rect);
            }

            if hovered || clicked_page == Some(page_index) {
                let tooltip = Rect::new(
                    painted_rect.center().x - 15.0,
                    painted_rect.y - 30.0,
                    30.0,
                    20.0,
                );
                draw_rectangle(tooltip.x, tooltip.y, tooltip.w, tooltip.h, WHITE);
                draw_text_centered(
                    &format!("{}", page_index + 1),
                    tooltip.center(),
                    FONT_SMALL,
                    BLACK,
                );
            }
        }

        if let Some(rect) = hovered_rect {
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, LIGHTGRAY);
        }

        if click_elapsed >= 1.5 {
            self.reader_click_page = None;
        }

        if hovered_rect.is_some() && self.reader_hover_start_time.is_none() {
            self.reader_hover_start_time = Some(Instant::now());
        }

        if hovered_rect.is_none() {
            self.reader_hover_start_time = None;
        }
    }

    /// Draws the chapter title with the drop down / wait / go up animation.
    fn reader_chap_title(&mut self, width: f32) {
        let chapter = match &self.reader_chapter_path {
            Some(chapter) => chapter.clone(),
            None => return,
        };

        let chapter_in = match zip_func::extract_file_from_zip(&chapter, "_metadata") {
            Ok(path) => path,
            Err(err) => {
                warn!("Error extracting file from zip: {}", err);
                metadata::ChapterMetadataIn::default()
            }
        };

        let title = if chapter_in.title.is_empty() {
            String::new()
        } else {
            format!(" - {}", chapter_in.title)
        };
        let vol = format!(" - {}", chapter_in.volume);
        let chap_num = if chapter_in.chapter.is_empty() {
            String::new()
        } else {
            format!(" Ch.{}", chapter_in.chapter)
        };
        let full_text = format!("{}{}{}{}", chapter_in.name, title, vol, chap_num);

        if self.reader_title_animation_state.is_none() {
            self.reader_title_animation_state = Some((Instant::now(), String::from("drop_down")));
        }

        let y_offset = match &mut self.reader_title_animation_state {
            Some((start_time, current_state)) => {
                let elapsed = start_time.elapsed().as_secs_f32();

                match current_state.as_str() {
                    "drop_down" => {
                        let progress = (elapsed / 0.5).min(0.5);
                        let offset = (1.0 - progress * 2.0) * -100.0;
                        if progress * 2.0 >= 1.0 {
                            *start_time = Instant::now();
                            *current_state = "wait".to_string();
                        }
                        offset
                    }
                    "wait" => {
                        if elapsed >= 1.5 {
                            *start_time = Instant::now();
                            *current_state = "go_up".to_string();
                        }
                        0.0
                    }
                    "go_up" => {
                        let progress = (elapsed / 0.5).min(0.5);
                        let offset = progress * 2.0 * -100.0;
                        if progress * 2.0 >= 1.0 {
                            *current_state = "end".to_string();
                        }
                        offset
                    }
                    "end" => -100.0,
                    _ => 0.0,
                }
            }
            None => 0.0,
        };

        let max_width = width - 40.0;
        let lines = wrap_text(&full_text, max_width, FONT_HEADING);
        let line_height = line_height(FONT_HEADING);
        let text_width = lines
            .iter()
            .map(|line| text_width(line, FONT_HEADING))
            .fold(0.0f32, f32::max);
        let text_height = lines.len() as f32 * line_height;
        let center = vec2(width / 2.0, PANEL_TOP / 2.0 + 50.0 + y_offset);

        draw_rectangle(
            center.x - text_width / 2.0 - 10.0,
            center.y - text_height / 2.0 - 5.0,
            text_width + 20.0,
            text_height + 10.0,
            OVERLAY,
        );

        let mut current_y = center.y - text_height / 2.0;
        for line in lines {
            draw_text_centered(
                &line,
                vec2(center.x, current_y + line_height / 2.0),
                FONT_HEADING,
                WHITE,
            );
            current_y += line_height;
        }
    }

    /// Draws the current page number out of the total amount of pages.
    fn reader_chap_number(&self, width: f32, height: f32) {
        let chapter = match &self.reader_chapter_path {
            Some(chapter) => chapter.clone(),
            None => return,
        };

        let chapter_len = zip_func::extract_image_len_from_zip_gui(&chapter).unwrap_or_default();
        let full_text = format!("{}/{}", self.reader_page + 1, chapter_len);
        let center = vec2(width / 2.0, height - 50.0);

        let text_size = text_width(&full_text, FONT_HEADING);
        let text_height = line_height(FONT_HEADING);

        draw_rectangle(
            center.x - text_size / 2.0 - 10.0,
            center.y - text_height / 2.0 - 5.0,
            text_size + 20.0,
            text_height + 10.0,
            OVERLAY,
        );
        draw_text_centered(&full_text, center, FONT_HEADING, WHITE);
    }

    /// Draws the loading animation centered on `center`.
    fn show_gif(&mut self, center: Vec2) {
        let texture = match self.next_gif_frame("loading") {
            Some(texture) => texture,
            None => {
                warn!("Failed to find gif image loading");
                return;
            }
        };

        let size = texture.size();
        draw_texture_ex(
            &texture,
            center.x - size.x / 2.0,
            center.y - size.y / 2.0 + 30.0,
            WHITE,
            Default::default(),
        );
    }

    /// Advances the gif animation if enough time passed and returns the frame to show.
    fn next_gif_frame(&mut self, name: &str) -> Option<Texture2D> {
        let frame_count = self.gif_textures.get(name)?.len();
        if frame_count == 0 {
            return None;
        }

        let now = Instant::now();
        match self.gif_last_update {
            Some(last_update) if now - last_update >= Duration::from_millis(100) => {
                self.gif_current_frame = (self.gif_current_frame + 1) % frame_count;
                self.gif_last_update = Some(now);
            }
            Some(_) => (),
            None => self.gif_last_update = Some(now),
        }

        self.gif_textures
            .get(name)?
            .get(self.gif_current_frame)
            .cloned()
    }

    /// Draws the quit confirmation dialog.
    ///
    /// # Returns
    /// - `true` when the user confirmed the quit.
    fn exit_dialog(&mut self, width: f32, height: f32) -> bool {
        let size = vec2(260.0, 90.0);
        let position = vec2(width / 2.0 - size.x / 2.0, height / 2.0 - size.y / 2.0);
        let rect = Rect::new(position.x, position.y, size.x, size.y);

        draw_rectangle(rect.x, rect.y, rect.w, rect.h, BUTTON_COLOR);
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, BUTTON_BORDER);
        draw_text_centered(
            "Do you want to quit?",
            vec2(width / 2.0, position.y + 30.0),
            FONT_NORMAL,
            WHITE,
        );

        let yes = draw_button(
            vec2(position.x + 30.0, position.y + 50.0),
            "Yes",
            FONT_NORMAL,
        );
        let no = draw_button(
            vec2(position.x + 140.0, position.y + 50.0),
            "No",
            FONT_NORMAL,
        );

        if yes {
            self.exit_show_confirmation_dialog = false;
            self.exit_allowed_to_close = true;
            return true;
        }
        if no {
            self.exit_show_confirmation_dialog = false;
            self.exit_allowed_to_close = false;
        }

        false
    }
}

/// Returns the size of a panel window for the given screen dimensions.
fn panel_size(width: f32, height: f32) -> Vec2 {
    vec2(
        (width - 16.0).max(120.0),
        (height - PANEL_TOP - 8.0).max(120.0),
    )
}

/// Grows a rect by `amount` on every side.
fn expand(rect: Rect, amount: f32) -> Rect {
    Rect::new(
        rect.x - amount,
        rect.y - amount,
        rect.w + amount * 2.0,
        rect.h + amount * 2.0,
    )
}

/// Returns the mouse position in screen pixels.
///
/// `mouse_position_local` normalizes the position to `-1..1`, which cannot be used
/// for hit testing pixel based rects.
fn mouse_pos() -> Vec2 {
    let (x, y) = mouse_position();
    Vec2::new(x, y)
}

/// Measures the width of a single line of text.
fn text_width(text: &str, font_size: f32) -> f32 {
    measure_text(text, None, font_size as u16, FONT_SCALE).width
}

/// Measures the height of a single line of text.
fn line_height(font_size: f32) -> f32 {
    measure_text("Ag", None, font_size as u16, FONT_SCALE).height
}

/// Draws text horizontally centered on `center` and vertically centered on it too.
fn draw_text_centered(text: &str, center: Vec2, font_size: f32, color: Color) {
    let dimensions = measure_text(text, None, font_size as u16, FONT_SCALE);
    draw_text(
        text,
        center.x - dimensions.width / 2.0,
        center.y + dimensions.offset_y - dimensions.height / 2.0,
        font_size,
        color,
    );
}

/// Draws a clickable button and returns whether it was pressed this frame.
fn draw_button(position: Vec2, label: &str, font_size: f32) -> bool {
    let dimensions = measure_text(label, None, font_size as u16, FONT_SCALE);
    let rect = Rect::new(
        position.x,
        position.y,
        dimensions.width + 16.0,
        dimensions.height + 10.0,
    );
    let hovered = rect.contains(mouse_pos());

    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if hovered {
            BUTTON_HOVER_COLOR
        } else {
            BUTTON_COLOR
        },
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, BUTTON_BORDER);
    draw_text(
        label,
        rect.x + 8.0,
        rect.y + 5.0 + dimensions.offset_y,
        font_size,
        WHITE,
    );

    hovered && is_mouse_button_pressed(MouseButton::Left)
}

/// Draws a centered heading and returns how much vertical space it used.
fn draw_heading(text: &str, width: f32, y: f32) -> f32 {
    let height = line_height(FONT_HEADING);
    draw_text_centered(
        text,
        vec2(width / 2.0, y + height / 2.0),
        FONT_HEADING,
        WHITE,
    );
    height
}

/// Draws a centered line of text and returns how much vertical space it used.
fn draw_line_text(text: &str, width: f32, y: f32) -> f32 {
    let height = line_height(FONT_NORMAL);
    draw_text_centered(
        text,
        vec2(width / 2.0, y + height / 2.0),
        FONT_NORMAL,
        WHITE,
    );
    height
}

/// Splits text into lines that fit into `max_width`.
fn wrap_text(text: &str, max_width: f32, font_size: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        let candidate = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };

        if !current_line.is_empty() && text_width(&candidate, font_size) > max_width {
            lines.push(std::mem::take(&mut current_line));
            current_line = word.to_string();
        } else {
            current_line = candidate;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Scans the manga folder for `.cbz` files and stores their paths globally.
fn get_chapter_paths(manga_data: metadata::MangaMetadata) {
    *READER_CHAPTER_PATHS.lock() = Some(HashMap::new());
    if let Ok(glob_results) = glob(&format!("{}\\*.cbz", &manga_data.mwd[4..])) {
        tokio::spawn(async move {
            for entry in glob_results.filter_map(Result::ok) {
                if let Some(entry_str) = entry.to_str() {
                    info!("Found entry: {}", entry_str);
                    if let Ok(manga) = resolute::check_for_metadata(entry_str) {
                        if let Some(ref mut value) = *READER_CHAPTER_PATHS.lock() {
                            value.insert(manga.id, entry_str.to_string());
                        }
                    }
                }
            }
        });
    }
}

/// Decodes image data and resizes it to fit the given area while keeping its ratio.
///
/// Returns the raw rgba8 pixels so the texture can be created on the gui thread.
fn decode_and_resize_image(
    image_data: &[u8],
    available_width: f32,
    available_height: f32,
) -> Option<Image> {
    let img = match load_from_memory(image_data) {
        Ok(img) => img,
        Err(err) => {
            warn!("Failed to load image: {}", err);
            return None;
        }
    };

    let img_rgba8 = img.to_rgba8();
    let img_width = img_rgba8.width() as f32;
    let img_height = img_rgba8.height() as f32;
    let scale = (available_width / img_width).min(available_height / img_height);
    let new_width = ((img_width * scale) as u32).max(1);
    let new_height = ((img_height * scale) as u32).max(1);

    let resized_image = image::imageops::resize(
        &img_rgba8,
        new_width,
        new_height,
        image::imageops::FilterType::Triangle,
    );

    Some(Image {
        bytes: resized_image.into_raw(),
        width: new_width as u16,
        height: new_height as u16,
    })
}

/// Retrieves manga data by reading and parsing `dat.json`.
fn get_manga_data() -> Result<Vec<metadata::MangaMetadata>, MdownError> {
    let dat_path = match getter::get_dat_path() {
        Ok(path) => path,
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 14003));
        }
    };
    if let Err(err) = std::fs::metadata(&dat_path) {
        debug!("dat.json not found: {}", err.to_string());
        return Err(MdownError::IoError(err, dat_path, 14000));
    }

    let json = match resolute::get_dat_content(dat_path.as_str()) {
        Ok(value) => value,
        Err(error) => {
            return Err(error);
        }
    };

    match serde_json::from_value::<metadata::Dat>(json) {
        Ok(dat) => Ok(dat.data),
        Err(err) => Err(MdownError::JsonError(err.to_string(), 14001)),
    }
}

/// Loads every gif used by the gui into gpu textures.
fn load_all_gifs() -> HashMap<String, Vec<Texture2D>> {
    let mut gif_textures = HashMap::new();

    gif_textures.insert(
        "loading".to_owned(),
        load_gif(LOADING_GIF)
            .into_iter()
            .map(|(image, _delay)| Texture2D::from_rgba8(image.width, image.height, &image.bytes))
            .collect(),
    );

    gif_textures
}

/// Decodes gif data into rgba8 frames with transparency handling.
///
/// Pixels using the green color `(0, 255, 0)` as their palette entry are treated as
/// fully transparent.
fn load_gif(file_data: &[u8]) -> Vec<(Image, u16)> {
    let mut frames = Vec::new();
    let mut decoder =
        gif::Decoder::new(BufReader::new(file_data)).expect("Failed to create GIF decoder");

    let mut all_frames = Vec::new();
    while let Ok(Some(frame)) = decoder.read_next_frame() {
        all_frames.push(frame.clone());
    }
    let palette = decoder.palette().expect("Failed to get palette");

    let transparent_color = (0, 255, 0);

    for frame in all_frames {
        let width = frame.width as usize;
        let height = frame.height as usize;

        let mut rgba_pixels = Vec::with_capacity(width * height * 4);

        for &index in frame.buffer.as_ref() {
            let base = (index as usize) * 3;
            let r = palette[base];
            let g = palette[base + 1];
            let b = palette[base + 2];

            if (r, g, b) == transparent_color {
                rgba_pixels.push(r);
                rgba_pixels.push(g);
                rgba_pixels.push(b);
                rgba_pixels.push(0);
            } else {
                rgba_pixels.push(r);
                rgba_pixels.push(g);
                rgba_pixels.push(b);
                rgba_pixels.push(255);
            }
        }

        frames.push((
            Image {
                bytes: rgba_pixels,
                width: width as u16,
                height: height as u16,
            },
            frame.delay,
        ));
    }

    frames
}

/// Resolves a manga from a url or id and starts the download process.
async fn resolve_download(url: &str, handle_id: Box<str>) -> Result<String, MdownError> {
    let id;

    if let Some(id_temp) = utils::resolve_regex(url) {
        id = id_temp.as_str().to_string();
    } else if utils::is_valid_uuid(url) {
        id = url.to_string();
    } else {
        id = String::from("*");
    }

    if id != "*" {
        let id = id.as_str();
        *resolute::MANGA_ID.lock() = id.to_string();
        info!("@{} Found {}", handle_id, id);
        match getter::get_manga_json(id).await {
            Ok(manga_name_json) => {
                let json_value = match serde_json::from_str(&manga_name_json) {
                    Ok(value) => value,
                    Err(_) => {
                        return Err(MdownError::JsonError(String::from("Invalid JSON"), 11400));
                    }
                };
                if let Value::Object(obj) = json_value {
                    resolute::resolve(obj, id).await
                } else {
                    Err(MdownError::JsonError(
                        String::from("Unexpected JSON value"),
                        11401,
                    ))
                }
            }
            Err(err) => Err(MdownError::ChainedError(Box::new(err), 11404)),
        }
    } else {
        info!("@{} Didn't find any id", handle_id);
        Err(MdownError::NotFoundError(String::from("ID"), 11402))
    }
}
