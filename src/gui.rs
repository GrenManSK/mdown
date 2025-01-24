use eframe::egui;
use egui::{ containers::*, * };
use glob::glob;
use image::load_from_memory;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use serde_json::Value;
use std::{
    collections::{ HashMap, HashSet },
    io::BufReader,
    ops::ControlFlow,
    sync::Arc,
    time::Instant,
};
use tracing::{ info, warn };

use crate::{
    args::{ self, ARGS },
    debug,
    error::MdownError,
    getter,
    handle_error,
    metadata,
    resolute,
    utils,
    version_manager::get_current_version,
    zip_func,
};

lazy_static! {
    pub(crate) static ref CURRENT_CHAPTER: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref READER_CURRENT_CHAPTER_ID: Mutex<String> = Mutex::new(String::new());
    pub(crate) static ref READER_CHAPTER_PATHS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
}

include!(concat!(env!("OUT_DIR"), "/loading_gif.rs"));

const NUM_OF_PRELOADS: usize = 10;

pub(crate) fn start() -> Result<(), MdownError> {
    match app() {
        Ok(()) => (),
        Err(err) => eprintln!("Error gui: {}", err),
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

pub(crate) fn app() -> Result<(), eframe::Error> {
    info!("Setting up options");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 600.0]),
        ..Default::default()
    };
    info!("Starting gui");
    eframe::run_native(
        &format!("mdown v{}", get_current_version()),
        options,
        Box::new(|_cc| Ok(Box::new(App::new(_cc))))
    )
}

#[derive(Default)]
struct App {
    exit_show_confirmation_dialog: bool,
    exit_allowed_to_close: bool,
    panel: String,
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
    download_texture_handle: Option<TextureHandle>,
    main_done_downloading: Option<String>,
    panel_show_heading: bool,
    reader_title_animation_state: Option<(Instant, String)>,
    reader_manga_data: Option<metadata::MangaMetadata>,
    reader_id: Option<metadata::ChapterMetadata>,
    reader_page: usize,
    reader_chapter_path: Option<String>,
    reader_chapter_len: Option<usize>,
    reader_chapters: Vec<metadata::ChapterMetadata>,
    reader_texture_cache: Arc<Mutex<HashMap<usize, Option<TextureHandle>>>>,
    reader_loading_pages: Arc<Mutex<HashSet<usize>>>,
    reader_hover_start_time: Option<Instant>,
    reader_click_start_time: Option<Instant>,
    reader_click_page: Option<usize>,
    gif_current_frame: usize,
    gif_last_update: Option<Instant>,
    gif_images: HashMap<String, Vec<(ColorImage, u16)>>,
}

impl App {
    fn new(_: &eframe::CreationContext<'_>) -> Self {
        let setup_url = ARGS.lock().url.clone();
        let setup_lang = ARGS.lock().lang.clone();
        let setup_offset = ARGS.lock().offset.clone();
        let setup_database_offset = ARGS.lock().database_offset.clone();
        let setup_title = ARGS.lock().title.clone();
        let setup_folder = ARGS.lock().folder.clone();
        let setup_volume = ARGS.lock().volume.clone();
        let setup_chapter = ARGS.lock().chapter.clone();
        let setup_max_consecutive = ARGS.lock().max_consecutive.clone();
        let setup_saver = ARGS.lock().saver;
        let setup_stat = ARGS.lock().stat;
        let setup_force = ARGS.lock().force;
        let gif_images = load_all_gifs();
        Self {
            exit_allowed_to_close: false,
            exit_show_confirmation_dialog: false,
            panel: "main".to_owned(),
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
            download_texture_handle: None,
            panel_show_heading: true,
            reader_title_animation_state: None,
            reader_manga_data: None,
            reader_id: None,
            reader_page: 0,
            reader_chapter_path: None,
            reader_chapters: Vec::new(),
            reader_texture_cache: Arc::new(Mutex::new(HashMap::new())),
            reader_loading_pages: Arc::new(Mutex::new(HashSet::new())),
            reader_chapter_len: None,
            reader_hover_start_time: None,
            reader_click_start_time: None,
            reader_click_page: None,
            gif_current_frame: 0,
            gif_last_update: Some(Instant::now()),
            gif_images,
        }
    }

    fn menu(&mut self, ui: &mut Ui) {
        menu::bar(ui, |ui| {
            ui.menu_button("Menu", |ui| {
                if ui.button("Main").clicked() {
                    info!("Selected main");
                    self.panel = String::from("main");
                    self.panel_show_heading = true;
                }
                if ui.button("Help").clicked() {
                    info!("Selected help");
                    self.panel = String::from("help");
                    self.panel_show_heading = true;
                }
                if ui.button("Reader").clicked() {
                    info!("Selected reader");
                    self.panel = String::from("reader");
                    self.reader_manga_data = None;
                    self.panel_show_heading = false;
                    self.reader_full_reset();
                }
            });
        });
    }

    fn reader(&mut self, ctx: &Context, ui: &mut Ui) {
        if let Some(chapter_id) = self.reader_id.clone() {
            self.reader_panel(ctx, ui, chapter_id);
            return;
        }
        if let Some(manga_data) = self.reader_manga_data.clone() {
            self.reader_chapter_selection(ui, manga_data);
            return;
        }
        self.reader_manga_selection(ui);
    }

    fn reader_manga_selection(&mut self, ui: &mut Ui) {
        if ui.button("Back").clicked() {
            self.reader_full_reset();
            self.reader_manga_data = None;
            self.panel = String::from("main");
            return;
        }
        ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
            ui.heading("Current downloaded manga");

            match get_manga_data() {
                Ok(manga_list) => {
                    for manga in manga_list {
                        if ui.button(manga.name.clone()).clicked() {
                            info!("Selected {}", manga.name);
                            self.reader_manga_data = Some(manga.clone());
                        }
                    }
                }
                Err(err) => warn!("Error getting manga data: {}", err),
            }
        });
    }

    fn reader_chapter_selection(&mut self, ui: &mut Ui, manga_data: metadata::MangaMetadata) {
        if ui.button("Back").clicked() {
            self.reader_full_reset();
            self.reader_manga_data = None;
            return;
        }
        ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
            ui.heading(format!("{} ({})", manga_data.name, manga_data.id));
        });
        ui.add_space(5.0);
        ui.horizontal_wrapped(|ui| {
            let mut chapters = manga_data.chapters.clone();
            chapters.sort_by(|a, b| a.parse_number().cmp(&b.parse_number()));
            self.reader_chapters = chapters.clone();
            for chapter in chapters.iter() {
                if ui.button(chapter.number.clone()).clicked() {
                    self.reader_reset();
                    self.reader_id = Some(chapter.clone());
                    self.reader_title_animation_state = None;
                    info!("Selected chapter id: {}", chapter.id.clone());
                    *READER_CURRENT_CHAPTER_ID.lock() = chapter.id.clone();
                }
            }
        });

        if READER_CHAPTER_PATHS.lock().is_none() {
            info!("Reading files ...");
            get_chapter_paths(manga_data);
        }
    }

    fn request_next_chapter(&mut self) -> bool {
        if let Some(current_chapter) = self.reader_id.clone() {
            if let Some(value) = current_chapter.get_next_chapter(&self.reader_chapters.clone()) {
                self.reader_reset();
                self.reader_id = Some(value.clone());
                self.reader_title_animation_state = None;
                self.request_chapter_path(value);
                self.request_chapter_len();
                *READER_CURRENT_CHAPTER_ID.lock() = value.id.clone();
                return true;
            }
        }
        false
    }

    fn request_previous_chapter(&mut self) -> bool {
        if let Some(current_chapter) = self.reader_id.clone() {
            if
                let Some(value) = current_chapter.get_previous_chapter(
                    &self.reader_chapters.clone()
                )
            {
                self.reader_reset();
                self.reader_id = Some(value.clone());
                self.reader_title_animation_state = None;
                self.request_chapter_path(value);
                self.request_chapter_len();
                *READER_CURRENT_CHAPTER_ID.lock() = value.id.clone();
                return true;
            }
        }
        false
    }

    fn reader_progress(&mut self, ui: &mut Ui, ctx: &Context) {
        if let Some(chapter_len) = self.reader_chapter_len {
            let segment_hover_margin = 10.0;
            let default_bar_height = 2.0;
            let expanded_bar_height = 10.0;

            let available_width = ui.available_width();
            let segment_width = available_width / (chapter_len as f32);

            let hover_duration = if let Some(start_time) = self.reader_hover_start_time {
                start_time.elapsed()
            } else {
                std::time::Duration::new(0, 0)
            };
            let click_duration = if let Some(start_time) = self.reader_click_start_time {
                start_time.elapsed()
            } else {
                std::time::Duration::new(0, 0)
            };

            let bar_height = if hover_duration.as_secs_f32() < 0.25 {
                let progress = hover_duration.as_secs_f32() / 0.25;
                default_bar_height + (expanded_bar_height - default_bar_height) * progress
            } else {
                expanded_bar_height
            };

            let bar_rect = Rect::from_min_max(
                Pos2::new(ui.min_rect().left(), ui.max_rect().bottom() - bar_height),
                Pos2::new(ui.min_rect().right(), ui.max_rect().bottom())
            );

            ui.painter().rect_filled(
                bar_rect.shrink((expanded_bar_height - default_bar_height) / 2.0),
                Rounding::same(4.0), // Rounded bar
                Color32::from_gray(200)
            );

            let mut hovered_segment_rect = None;

            for page_index in 0..chapter_len {
                let color = if page_index == self.reader_page {
                    Color32::WHITE
                } else if let Some(Some(_)) = self.reader_texture_cache.lock().get(&page_index) {
                    Color32::GRAY
                } else {
                    Color32::BLACK
                };

                //how many pixels should item be expanded
                let progress_of_expansion = if let Some(clicked_index) = self.reader_click_page {
                    let expansion = 20.0;
                    if page_index == clicked_index {
                        if click_duration.as_secs_f32() < 0.5 {
                            let progress = click_duration.as_secs_f32() / 0.5;
                            progress * expansion
                        } else if
                            click_duration.as_secs_f32() >= 0.5 &&
                            click_duration.as_secs_f32() < 1.0
                        {
                            expansion
                        } else if
                            click_duration.as_secs_f32() > 1.0 &&
                            click_duration.as_secs_f32() < 1.5
                        {
                            let progress = (click_duration.as_secs_f32() - 1.0) / 0.5;

                            (1.0 - progress) * expansion
                        } else {
                            self.reader_click_page = None;
                            0.0
                        }
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                let segment_rect = Rect::from_min_max(
                    Pos2::new(
                        bar_rect.left() + segment_width * (page_index as f32),
                        bar_rect.top() +
                            (expanded_bar_height - default_bar_height - progress_of_expansion) / 2.0
                    ),
                    Pos2::new(
                        bar_rect.left() + segment_width * ((page_index as f32) + 1.0),
                        bar_rect.top() +
                            (expanded_bar_height + default_bar_height + progress_of_expansion) / 2.0
                    )
                );

                let hover_segment_rect = segment_rect.expand(segment_hover_margin);

                let segment_response = ui.interact(
                    hover_segment_rect,
                    ui.make_persistent_id(page_index),
                    Sense::click()
                );

                let hovered_rect = if segment_response.hovered() {
                    hovered_segment_rect = Some(segment_rect);
                    segment_rect.expand(10.0)
                } else {
                    segment_rect
                };

                let segment_color = if segment_response.hovered() {
                    Color32::LIGHT_GRAY
                } else {
                    color
                };

                ui.painter().rect_filled(
                    hovered_rect,
                    Rounding::same(6.0), // Rounded segments
                    segment_color
                );

                if segment_response.hovered() || self.reader_click_page.is_some() {
                    ctx.request_repaint();
                }

                if segment_response.clicked() && self.reader_page != page_index {
                    self.reader_click_start_time = Some(Instant::now());
                    self.reader_click_page = Some(page_index);
                    self.reader_page = page_index;
                    self.download_texture_handle = None;
                    info!("Jumped to page: {}", page_index);
                }
                let in_click_animation = match
                    self.reader_click_page.and_then(|page| Some(page == page_index))
                {
                    Some(page) => page,
                    None => false,
                };

                if segment_response.hovered() || in_click_animation {
                    let tooltip_rect = Rect::from_min_size(
                        hovered_rect.center_top() - Vec2::new(15.0, 30.0),
                        Vec2::new(30.0, 20.0)
                    );
                    ui.painter().rect_filled(
                        tooltip_rect,
                        Rounding::same(4.0), // Rounded tooltip
                        Color32::WHITE
                    );
                    ui.painter().text(
                        tooltip_rect.center(),
                        Align2::CENTER_CENTER,
                        format!("{}", page_index + 1),
                        FontId::proportional(12.0),
                        Color32::BLACK
                    );
                }
            }

            if let Some(hovered_rect) = hovered_segment_rect {
                ui.painter().rect_filled(
                    hovered_rect.expand(10.0),
                    Rounding::same(6.0), // Rounded hovered rectangle
                    Color32::LIGHT_GRAY
                );
            }

            if self.reader_hover_start_time.is_none() && hovered_segment_rect.is_some() {
                self.reader_hover_start_time = Some(Instant::now());
            }

            if hovered_segment_rect.is_none() {
                self.reader_hover_start_time = None;
            }
        }
    }

    fn reader_chap_title(&mut self, ui: &mut Ui, ctx: &Context) {
        if let Some(chapter) = &self.reader_chapter_path {
            let chapter_in = match zip_func::extract_file_from_zip(chapter, "_metadata") {
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

            // Initialize animation state
            if self.reader_title_animation_state.is_none() {
                self.reader_title_animation_state = Some((
                    Instant::now(),
                    String::from("drop_down"),
                ));
            }

            // Handle animation logic
            let y_offset = if
                let Some((start_time, current_state)) = &mut self.reader_title_animation_state
            {
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
            } else {
                0.0
            };

            // Draw the title with animation
            let painter = ui.painter();
            let screen_rect = ui.clip_rect();
            let text_position = screen_rect.center_top() + egui::vec2(0.0, y_offset + 50.0);

            let max_width = screen_rect.width() - 40.0; // Ensure padding on both sides
            let mut wrapped_lines = Vec::new();
            let fonts = ui.fonts(|fonts| fonts.clone());

            let mut current_line = String::new();
            for word in full_text.split_whitespace() {
                let test_line = if current_line.is_empty() {
                    word.to_string()
                } else {
                    format!("{} {}", current_line, word)
                };

                let test_line_size = fonts.layout_no_wrap(
                    test_line.clone(),
                    egui::TextStyle::Heading.resolve(&ui.style()),
                    Color32::BLACK
                );

                if test_line_size.rect.width() > max_width {
                    wrapped_lines.push(current_line.clone());
                    current_line = word.to_string();
                } else {
                    current_line = test_line;
                }
            }

            if !current_line.is_empty() {
                wrapped_lines.push(current_line);
            }

            let text_size = egui::vec2(
                max_width,
                (wrapped_lines.len() as f32) *
                    fonts.row_height(&egui::TextStyle::Heading.resolve(&ui.style()))
            );

            let bg_rect = egui::Rect::from_center_size(
                text_position,
                text_size + egui::vec2(20.0, 10.0) // Add padding
            );

            // Draw background with rounded corners
            painter.rect_filled(
                bg_rect,
                10.0, // Rounded corners radius
                Color32::from_rgba_premultiplied(0, 0, 0, 180) // Semi-transparent black
            );

            // Draw each wrapped line
            let mut current_y = text_position.y - text_size.y / 2.0;
            for line in wrapped_lines {
                painter.text(
                    egui::pos2(text_position.x, current_y),
                    egui::Align2::CENTER_TOP,
                    line,
                    egui::TextStyle::Heading.resolve(&ui.style()),
                    Color32::WHITE
                );
                current_y += fonts.row_height(&egui::TextStyle::Heading.resolve(&ui.style()));
            }

            // Request repaint if animation is ongoing
            if self.reader_title_animation_state.is_some() {
                ctx.request_repaint();
            }
        }
    }

    fn reader_chap_number(&self, ui: &mut Ui) {
        if let Some(chapter) = &self.reader_chapter_path {
            let chapter_len = match zip_func::extract_image_len_from_zip_gui(&chapter) {
                Ok(len) => len,
                Err(_err) => 0,
            };

            let full_text = format!("{}/{}", self.reader_page.clone() + 1, chapter_len);

            let painter = ui.painter();
            let screen_rect = ui.clip_rect();

            // Determine the position and size for the text
            let text_size = ui.fonts(|fonts| {
                fonts
                    .layout_no_wrap(
                        full_text.clone(),
                        egui::TextStyle::Heading.resolve(&ui.style()),
                        Color32::BLACK
                    )
                    .rect.size()
            });

            // Calculate the background rectangle size and position
            let text_position = screen_rect.center_bottom() - egui::vec2(0.0, 50.0); // Adjust position
            let bg_rect = egui::Rect::from_center_size(
                text_position,
                text_size + egui::vec2(20.0, 10.0) // Add padding
            );

            // Draw background with rounded corners
            painter.rect_filled(
                bg_rect,
                10.0, // Rounded corners radius
                Color32::from_rgba_premultiplied(0, 0, 0, 180) // Semi-transparent black
            );

            // Draw the chapter number text
            painter.text(
                text_position,
                egui::Align2::CENTER_CENTER,
                full_text,
                egui::TextStyle::Heading.resolve(&ui.style()),
                Color32::WHITE
            );
        }
    }

    fn reader_panel(&mut self, ctx: &Context, ui: &mut Ui, chapter_id: metadata::ChapterMetadata) {
        if ui.button("Back").clicked() {
            self.reader_reset();
            return;
        }
        if let ControlFlow::Break(_) = self.reader_handle_input(ctx, ui) {
            return;
        }
        if let Some(file_path) = self.reader_chapter_path.clone() {
            let available_width = ui.available_width();
            let available_height = ui.available_height();
            self.reader_preload(ctx, file_path, available_width, available_height);

            ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
                // Display the current page
                let mut loading = false;
                match self.reader_texture_cache.lock().get(&self.reader_page) {
                    Some(Some(texture)) => {
                        ui.image(texture);
                    }
                    Some(None) => {
                        ui.heading("Loading page...");
                        loading = true;
                    }
                    None => {
                        ui.heading("Page not available");
                    }
                }
                if loading {
                    self.show_gif(ctx, "loading");
                }
            });
        }

        self.reader_chap_number(ui);
        self.reader_chap_title(ui, ctx);
        self.reader_progress(ui, ctx);
        self.request_chapter_path(&chapter_id);
        self.request_chapter_len();
    }

    fn request_chapter_len(&mut self) {
        if self.reader_chapter_len.is_none() {
            if let Some(file_path) = self.reader_chapter_path.clone() {
                let chapter_len = match zip_func::extract_image_len_from_zip_gui(&file_path) {
                    Ok(len) => len,
                    Err(_err) => 0,
                };
                self.reader_chapter_len = Some(chapter_len);
            }
        }
    }

    fn request_chapter_path(&mut self, chapter_id: &metadata::ChapterMetadata) {
        if self.reader_chapter_path.is_none() {
            if let Some(paths) = READER_CHAPTER_PATHS.lock().clone() {
                match paths.get(&chapter_id.id) {
                    Some(path) => {
                        self.reader_chapter_path = Some(path.to_string());
                        info!("Chapter path set to: {}", path);
                    }
                    None => (),
                }
            }
        }
    }

    fn reader_preload(
        &mut self,
        ctx: &Context,
        file_path: String,
        available_width: f32,
        available_height: f32
    ) {
        let id = READER_CURRENT_CHAPTER_ID.lock().clone();
        // Preloading logic (same as before)
        for offset in (0..NUM_OF_PRELOADS)
            .flat_map(|n| [n as isize, -(n as isize)].into_iter())
            .filter_map(|off| ((self.reader_page as isize) + off).try_into().ok()) {
            let page_to_load = offset;
            let mut reader_loading_pages = self.reader_loading_pages.lock();
            let reader_texture_cache = self.reader_texture_cache.lock();

            if
                !reader_texture_cache.contains_key(&page_to_load) &&
                !reader_loading_pages.contains(&page_to_load)
            {
                reader_loading_pages.insert(page_to_load);
                let file_path = file_path.clone();
                let ctx_clone = ctx.clone();
                let cache_clone = self.reader_texture_cache.clone();
                let loading_clone = self.reader_loading_pages.clone();
                let id_clone = id.clone();

                tokio::spawn(async move {
                    match zip_func::extract_image_from_zip_gui(&file_path, page_to_load + 1) {
                        Ok(image_data) => {
                            cache_clone.lock().insert(page_to_load, None);
                            let texture = load_and_resize_image(
                                &ctx_clone,
                                &image_data,
                                available_width,
                                available_height
                            );
                            if id_clone != *READER_CURRENT_CHAPTER_ID.lock() {
                                return;
                            }
                            cache_clone.lock().insert(page_to_load, texture);
                            info!("Preloaded page {}", page_to_load);
                            ctx_clone.request_repaint();
                        }
                        Err(err) =>
                            match err {
                                MdownError::NotFoundError(..) => (),
                                err => warn!("Error loading page {}: {}", page_to_load, err),
                            }
                    }
                    loading_clone.lock().remove(&page_to_load);
                });
            }
        }
    }

    fn reader_handle_input(&mut self, ctx: &Context, ui: &mut Ui) -> ControlFlow<()> {
        let input = ctx.input(|i| i.clone());
        if input.key_pressed(egui::Key::ArrowRight) {
            if let Some(chap_len) = self.reader_chapter_len.clone() {
                if input.modifiers.ctrl {
                    // handle ctrl + right arrow
                    if self.request_next_chapter() {
                        return ControlFlow::Break(());
                    } else {
                        self.reader_reset();
                        info!("Manga is finished");
                        return ControlFlow::Break(());
                    }
                } else if input.modifiers.shift {
                    // handle shift + right arrow
                    self.reader_page = chap_len - 1;
                    return ControlFlow::Continue(());
                } else if self.reader_page + 1 >= chap_len && chap_len != 0 {
                    if self.request_next_chapter() {
                        return ControlFlow::Break(());
                    } else {
                        self.reader_reset();
                        info!("Manga is finished");
                        return ControlFlow::Break(());
                    }
                }
            }
            self.reader_page += 1;
            self.download_texture_handle = None;
            info!("Next page: {}", self.reader_page);
        } else if input.key_pressed(egui::Key::ArrowLeft) {
            if input.modifiers.ctrl {
                // Handle ctrl + left arrow

                // Don't handle if previous chapter was found or not
                self.request_previous_chapter();
                self.reader_page = 0;
                return ControlFlow::Break(());
            } else if input.modifiers.shift {
                // Handle shift + left arrow

                self.reader_page = 0;
                return ControlFlow::Continue(());
            } else if self.reader_page <= 0 {
                // handle if page is 0 (or less)

                // If there is previous chapter change chapter length to its last image
                // Note that self.reader_chapter_len is already set to new chapter
                if self.request_previous_chapter() {
                    self.reader_page = match self.reader_chapter_len {
                        Some(len) => len - 1,
                        None => 0,
                    };
                }
            } else {
                // Handle normal

                self.reader_page -= 1;
                self.download_texture_handle = None;
                info!("Previous page: {}", self.reader_page);
            }
        } else if input.key_pressed(egui::Key::ArrowUp) {
            if let Some(chap_len) = self.reader_chapter_len.clone() {
                if self.reader_page + 1 >= chap_len && chap_len != 0 {
                    // Handle if page is already at end

                    if self.request_next_chapter() {
                        return ControlFlow::Continue(());
                    } else {
                        // Handle if there is no next chapter

                        self.reader_reset();
                        ui.heading("Manga is finished");
                    }
                    return ControlFlow::Continue(());
                } else if self.reader_page + 5 >= chap_len && chap_len != 0 {
                    // Handle normal
                    self.reader_page = chap_len - 1;

                    return ControlFlow::Continue(());
                }
            }
            self.reader_page += 5;
            self.download_texture_handle = None;
            info!("Next page: {}", self.reader_page);
        } else if input.key_pressed(egui::Key::ArrowDown) {
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
            self.download_texture_handle = None;
            info!("Previous page: {}", self.reader_page);
        } else if input.key_pressed(egui::Key::R) {
            if input.modifiers.shift {
                self.reader_texture_cache.lock().clear();
                info!("Clearing the entire texture cache");
            } else {
                self.reader_texture_cache.lock().remove(&self.reader_page);
                info!("Resetting page {}", self.reader_page);
            }
        } else if input.key_pressed(egui::Key::Q) {
            self.reader_reset();
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    }

    fn reader_reset(&mut self) {
        self.reader_id = None;
        self.reader_page = 0;
        self.reader_chapter_path = None;
        self.reader_chapter_len = None;
        self.download_texture_handle = None;
        self.reader_texture_cache.lock().clear();
        self.reader_title_animation_state = None;
        self.reader_loading_pages.lock().clear();
        *READER_CURRENT_CHAPTER_ID.lock() = String::new();
    }

    fn reader_full_reset(&mut self) {
        self.reader_reset();
        self.reader_chapters.clear();
        *READER_CHAPTER_PATHS.lock() = None;
    }

    fn main(&mut self, ctx: &Context, ui: &mut Ui) {
        if !*resolute::DOWNLOADING.lock() {
            self.main_config(ui);
        } else {
            self.main_downloading(ctx, ui);
        }
    }

    fn main_downloading(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
            ui.label(format!("Downloading {}", resolute::MANGA_NAME.lock()));
            ui.label(format!("Chapter: {}", resolute::CURRENT_CHAPTER.lock()));
            ui.label(
                format!(
                    "[{:.2}mb/{:.2}mb]",
                    resolute::CURRENT_SIZE.lock(),
                    resolute::CURRENT_SIZE_MAX.lock()
                )
            );
            let current_page = resolute::CURRENT_PAGE.lock();
            let current_page_max = resolute::CURRENT_PAGE_MAX.lock();
            let progress = "#".repeat(*current_page as usize);
            let message = format!("Progress: [{}/{}]", current_page, current_page_max);
            ui.label(message);
            ui.label(progress);
            ui.add_space(5.0);

            self.main_downloading_end_panel(ctx, ui);
            if self.main_done_downloading.is_none() && resolute::MANGA_ID.lock().clone() != "" {
                self.main_done_downloading = Some(resolute::MANGA_ID.lock().clone());
            }
            ctx.request_repaint();
        });
    }

    fn main_downloading_end_panel(&mut self, ctx: &Context, ui: &mut Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            if !resolute::WEB_DOWNLOADED.lock().is_empty() {
                ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
                    ui.label("Downloaded:");
                    for i in resolute::WEB_DOWNLOADED.lock().iter() {
                        ui.label(i);
                    }
                });
            }

            if !resolute::SCANLATION_GROUPS.lock().is_empty() {
                ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
                    ui.label("Scanlation group:");
                    for i in resolute::SCANLATION_GROUPS.lock().iter() {
                        ui.label(i.name.clone());
                    }
                });
            }

            if self.download_texture_handle.is_some() {
                match std::fs::metadata(".cache\\preview\\preview.png") {
                    Ok(_metadata) => {
                        if *resolute::CURRENT_CHAPTER.lock() != *CURRENT_CHAPTER.lock() {
                            *CURRENT_CHAPTER.lock() = resolute::CURRENT_CHAPTER.lock().to_string();
                            self.download_texture_handle = None;
                        }
                    }
                    Err(_err) => (),
                };
            }
            if let Some(download_texture_handle) = &self.download_texture_handle {
                ui.image(download_texture_handle);
            } else {
                match image::open(".cache\\preview\\preview.png") {
                    Ok(img) => {
                        let img_rgba8 = img.to_rgba8();
                        let size = [img_rgba8.width() as usize, img_rgba8.height() as usize];
                        let color_image = ColorImage::from_rgba_unmultiplied(size, &img_rgba8);
                        let download_texture_handle = ctx.load_texture(
                            "my_image",
                            color_image,
                            TextureOptions::default()
                        );
                        self.download_texture_handle = Some(download_texture_handle);
                    }
                    Err(_err) => (),
                }
            }
        });
    }

    fn main_config(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.label("Set url of manga");
                ui.text_edit_singleline(&mut self.setup_url);
                if let Some(id) = utils::resolve_regex(self.setup_url.as_str()) {
                    ui.label(format!("Found id: {}", id.as_str()));
                    self.setup_url = id.as_str().to_string();
                } else if utils::is_valid_uuid(self.setup_url.as_str()) {
                    ui.label(format!("Found id: {}", self.setup_url));
                }
                ui.add_space(1.0);
                ui.label("Set language of manga");
                ui.text_edit_singleline(&mut self.setup_lang);
                ui.label("Set offset");
                ui.text_edit_singleline(&mut self.setup_offset);
                ui.label("Set offset of database");
                ui.text_edit_singleline(&mut self.setup_database_offset);
                ui.label("Set title of manga");
                ui.text_edit_singleline(&mut self.setup_title);
                ui.label("Set folder to put manga in");
                ui.text_edit_singleline(&mut self.setup_folder);
                ui.label("Set volume of manga");
                ui.text_edit_singleline(&mut self.setup_volume);
                ui.label("Set chapter of manga");
                ui.text_edit_singleline(&mut self.setup_chapter);
                ui.label("Set max consecutive of manga");
                ui.text_edit_singleline(&mut self.setup_max_consecutive);
                ui.checkbox(&mut self.setup_saver, "Saver");
                ui.checkbox(&mut self.setup_stat, "Statistics");
                ui.checkbox(&mut self.setup_force, "Force");

                ui.add_space(5.0);
                if ui.button("Download").clicked() {
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
                        self.setup_max_consecutive.clone(),
                        self.setup_force,
                        self.setup_offset.clone(),
                        self.setup_database_offset.clone()
                    );
                    let url = self.setup_url.clone();
                    *resolute::SAVER.lock() = self.setup_saver;
                    resolute::SCANLATION_GROUPS.lock().clear();
                    let _ = tokio::spawn(async move {
                        match resolve_download(&url, handle_id).await {
                            Ok(_) => (),
                            Err(err) => handle_error!(&err, String::from("gui")),
                        };
                    });
                }

                ui.add_space(5.0);

                if let Some(downloaded_manga_id) = self.main_done_downloading.clone() {
                    match get_manga_data() {
                        Ok(manga_list) => {
                            for manga in manga_list {
                                if manga.id == downloaded_manga_id {
                                    if ui.button(format!("{}", manga.name)).clicked() {
                                        self.panel = String::from("reader");
                                        self.reader_manga_data = Some(manga);
                                    }
                                }
                            }
                        }
                        Err(err) => warn!("Error getting manga data: {}", err),
                    }
                }

                if !resolute::WEB_DOWNLOADED.lock().is_empty() {
                    ui.label("Downloaded:");
                    for i in resolute::WEB_DOWNLOADED.lock().iter() {
                        ui.label(i);
                    }
                }

                if !resolute::SCANLATION_GROUPS.lock().is_empty() {
                    ui.label("Scanlation group:");
                    for i in resolute::SCANLATION_GROUPS.lock().iter() {
                        ui.label(i.name.clone());
                    }
                }
            })
        });
    }

    fn help(&self, ui: &mut Ui) {
        ui.heading("Downloader");
        ui.add_space(20.0);
        ui.label("Write url and press download");
        ui.add_space(20.0);
        ui.heading("Reader");
        ui.add_space(20.0);
        ui.label("with left and right arrows you move pages by 1");
        ui.label("with up and down arrows you move pages by 5");
    }

    fn main_panel(&mut self, ctx: &Context, ui: &mut Ui) {
        if self.panel == *"reader" {
            self.reader(ctx, ui)
        } else if self.panel == *"main" {
            self.main(ctx, ui);
        } else if self.panel == *"help" {
            self.help(ui);
        }
    }

    fn show_gif(&mut self, ctx: &Context, path: &str) {
        let gif_frames = match self.gif_images.get(&path.to_string()) {
            Some(frames) => frames,
            None => {
                warn!("Failed to find gif image {}", path);
                return;
            }
        };

        if let Some(last_update) = self.gif_last_update {
            let now = Instant::now();
            let frame_delay = std::time::Duration::from_millis(100);

            if now - last_update >= frame_delay {
                self.gif_current_frame = (self.gif_current_frame + 1) % gif_frames.len();
                self.gif_last_update = Some(now);
            }

            let texture = ctx.load_texture(
                "gif_frame",
                gif_frames[self.gif_current_frame].0.clone(),
                Default::default()
            );

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.image(&texture);
                });
            });

            ctx.request_repaint_after(frame_delay);
        }
    }

    fn exit_dialog(&mut self, ctx: &Context) {
        egui::Window
            ::new("Do you want to quit?")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Yes").clicked() {
                        self.exit_show_confirmation_dialog = false;
                        self.exit_allowed_to_close = true;
                        info!("Closing gui");
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button("No").clicked() {
                        self.exit_show_confirmation_dialog = false;
                        self.exit_allowed_to_close = false;
                    }
                });
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.menu(ui);

            if self.panel_show_heading {
                ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
                    ui.heading(format!("mdown v{}", get_current_version()));
                });
            }

            self.main_panel(ctx, ui)
        });

        if ctx.input(|i| i.viewport().close_requested()) {
            if self.exit_allowed_to_close {
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.exit_show_confirmation_dialog = true;
            }
        }

        if self.exit_show_confirmation_dialog {
            self.exit_dialog(ctx);
        }
    }
}

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

fn load_and_resize_image(
    ctx: &Context,
    image_data: &[u8],
    available_width: f32,
    available_height: f32
) -> Option<TextureHandle> {
    match load_from_memory(image_data) {
        Ok(img) => {
            let img_rgba8 = img.to_rgba8();
            let img_width = img_rgba8.width() as f32;
            let img_height = img_rgba8.height() as f32;

            let scale_x = available_width / img_width;
            let scale_y = available_height / img_height;
            let scale = scale_x.min(scale_y);

            let new_width = (img_width * scale) as u32;
            let new_height = (img_height * scale) as u32;

            let resized_image = image::imageops::resize(
                &img_rgba8,
                new_width,
                new_height,
                image::imageops::FilterType::Triangle
            );

            let color_image = ColorImage::from_rgba_unmultiplied(
                [new_width as usize, new_height as usize],
                &resized_image
            );
            Some(ctx.load_texture("my_image", color_image, TextureOptions::default()))
        }
        Err(e) => {
            warn!("Failed to load image: {}", e);
            None
        }
    }
}

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

fn load_all_gifs() -> HashMap<String, Vec<(ColorImage, u16)>> {
    let mut gif_images = HashMap::new();
    gif_images.insert("loading".to_owned(), load_gif(LOADING_GIF));
    gif_images
}

fn load_gif(file_data: &[u8]) -> Vec<(ColorImage, u16)> {
    let mut frames = Vec::new();
    let mut decoder = gif::Decoder
        ::new(BufReader::new(file_data))
        .expect("Failed to create GIF decoder");

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

        let buffer = frame.buffer.as_ref();

        for &index in buffer {
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

        let color_image = ColorImage::from_rgba_unmultiplied([width, height], &rgba_pixels);
        frames.push((color_image, frame.delay));
    }

    frames
}

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
                    Err(MdownError::JsonError(String::from("Unexpected JSON value"), 11401))
                }
            }
            Err(err) => Err(MdownError::ChainedError(Box::new(err), 11404)),
        }
    } else {
        info!("@{} Didn't find any id", handle_id);
        Err(MdownError::NotFoundError(String::from("ID"), 11402))
    }
}
