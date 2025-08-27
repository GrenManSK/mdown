use eframe::egui;
use egui::{ containers::*, * };
use glob::glob;
use image::load_from_memory;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use serde_json::Value;
use smallvec::{ smallvec, SmallVec };
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
    error::{ self, MdownError },
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

/// Initializes and runs the GUI application using `eframe`.
///
/// This function sets up the `eframe::NativeOptions` with a predefined viewport size
/// and starts the graphical user interface (GUI) for the `mdown` application.
///
/// # Errors
/// - Returns `eframe::Error` if initializing or running the GUI fails.
///
/// # Returns
/// - `Ok(())` if the GUI runs successfully.
/// - `Err(eframe::Error)` if an error occurs during initialization.
///
/// # Example
/// ```
/// if let Err(e) = app() {
///     eprintln!("Failed to start the GUI: {:?}", e);
/// }
/// ```
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
        let setup_max_consecutive = ARGS.lock().max_consecutive.clone().to_string();
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

    /// Creates the application's menu bar.
    ///
    /// This function defines a menu bar with a "Menu" button that contains three options:
    /// - "Main": Switches the panel to "main" and enables the heading.
    /// - "Help": Switches the panel to "help" and enables the heading.
    /// - "Reader": Switches the panel to "reader", resets the reader data, and disables the heading.
    ///
    /// # Parameters
    /// - `ui: &mut Ui` – The egui UI context used for rendering the menu.
    ///
    /// # Behavior
    /// - Logs the selected menu option.
    /// - Updates the `panel` field based on the user's selection.
    /// - Resets reader-related data when switching to the "Reader" panel.
    fn menu(&mut self, ui: &mut Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
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

    /// Handles the manga reader panel.
    ///
    /// This function determines what should be displayed in the reader panel based on the state:
    /// - If `reader_id` is set, it calls `reader_panel` to display the selected chapter.
    /// - If `reader_manga_data` is available, it calls `reader_chapter_selection` to show the chapter selection.
    /// - Otherwise, it calls `reader_manga_selection` to allow the user to choose a manga.
    ///
    /// # Parameters
    /// - `ctx: &Context` – The egui context used for rendering.
    /// - `ui: &mut Ui` – The UI context for drawing elements.
    ///
    /// # Behavior
    /// - If a chapter is selected, it renders the reader panel.
    /// - If manga data is available, it allows the user to select a chapter.
    /// - Otherwise, it presents a manga selection interface.
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

    /// Displays a list of downloaded manga for the user to select from.
    ///
    /// This function shows a list of currently downloaded manga, allowing the user to choose one for reading.
    /// If the "Back" button is clicked, it resets the reader state and returns to the main panel.
    ///
    /// # Parameters
    /// - `ui: &mut Ui` – The UI context used for rendering.
    ///
    /// # Behavior
    /// - Displays a "Back" button that, when clicked, resets the reader state and switches to the main panel.
    /// - Displays a list of downloaded manga, and when a manga name is clicked, it updates the `reader_manga_data` with the selected manga.
    ///
    /// # Errors
    /// - If there is an issue retrieving manga data, a warning is logged.
    ///
    /// # Example
    /// ```
    /// // Inside the UI loop
    /// self.reader_manga_selection(ui);
    /// ```
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

    /// Displays the chapter selection interface for a selected manga.
    ///
    /// This function allows the user to select a chapter from the available chapters of a manga. It provides the following functionality:
    /// - Displays the manga name and ID as a heading.
    /// - Displays a list of chapters, sorted by their chapter number.
    /// - When a chapter is selected, it resets the reader state and loads the selected chapter's data.
    /// - Displays a "Back" button that, when clicked, resets the reader state and returns to the manga selection.
    ///
    /// # Parameters
    /// - `ui: &mut Ui` – The UI context used for rendering.
    /// - `manga_data: metadata::MangaMetadata` – The metadata for the selected manga, including the list of available chapters.
    ///
    /// # Behavior
    /// - Displays the "Back" button to reset the reader state and go back to manga selection.
    /// - Renders the manga name and ID as a heading.
    /// - Displays a list of chapters for the selected manga, and when a chapter is clicked, it updates the reader state with the selected chapter.
    ///
    /// # Example
    /// ```
    /// // Inside the UI loop
    /// self.reader_chapter_selection(ui, manga_data);
    /// ```
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

    /// Attempts to load the next chapter in the reader.
    ///
    /// This function checks if there is a current chapter selected. If a next chapter exists, it updates the reader state to load the next chapter and requests the necessary data for that chapter.
    ///
    /// # Returns
    /// - `true` if a next chapter was successfully found and loaded.
    /// - `false` if no next chapter exists or the current chapter is not selected.
    ///
    /// # Behavior
    /// - If there is a current chapter, it tries to find the next chapter from the list of available chapters (`reader_chapters`).
    /// - If a next chapter is found, the function updates the reader state, requests the chapter path and length, and sets the new chapter as the current one.
    /// - If no next chapter is found, the function returns `false`.
    ///
    /// # Example
    /// ```
    /// if self.request_next_chapter() {
    ///     // Next chapter successfully loaded
    /// } else {
    ///     // No next chapter available
    /// }
    /// ```
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

    /// Attempts to load the previous chapter in the reader.
    ///
    /// This function checks if there is a current chapter selected. If a previous chapter exists, it updates the reader state to load the previous chapter and requests the necessary data for that chapter.
    ///
    /// # Returns
    /// - `true` if a previous chapter was successfully found and loaded.
    /// - `false` if no previous chapter exists or the current chapter is not selected.
    ///
    /// # Behavior
    /// - If there is a current chapter, it tries to find the previous chapter from the list of available chapters (`reader_chapters`).
    /// - If a previous chapter is found, the function updates the reader state, requests the chapter path and length, and sets the new chapter as the current one.
    /// - If no previous chapter is found, the function returns `false`.
    ///
    /// # Example
    /// ```
    /// if self.request_previous_chapter() {
    ///     // Previous chapter successfully loaded
    /// } else {
    ///     // No previous chapter available
    /// }
    /// ```
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

    /// Displays the reader progress bar and allows interaction for chapter navigation.
    ///
    /// This function displays a horizontal progress bar representing the pages of a chapter. Each segment of the progress bar represents a page, and the user can hover or click on a segment to navigate to the corresponding page. The bar also visually reacts to user interactions like hovering and clicking, with animations for the click state.
    ///
    /// # Parameters
    /// - `ui`: The UI context to draw the progress bar.
    /// - `ctx`: The application context used to request a repaint for the progress bar when hovered or clicked.
    ///
    /// # Behavior
    /// - Displays a progress bar where each segment represents a page of the current chapter.
    /// - Highlights the currently viewed page and shows the progress of page loading if not yet loaded.
    /// - Allows users to click on a page segment to jump to that page with animations for clicked segments.
    /// - If a segment is hovered, it expands and shows a tooltip with the page number.
    /// - The click animation adjusts the segment size over time to create a visual effect when the user clicks on a page.
    ///
    /// # Animation
    /// - Hovering over a segment causes it to expand slightly, and the segment color changes.
    /// - Clicking on a segment will animate the segment's expansion and then shrink it back after a short duration.
    ///
    /// # Example
    /// ```
    /// self.reader_progress(ui, ctx);
    /// ```
    /// This function is typically called within the UI update cycle to render and update the progress bar as the user interacts with it.
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
                CornerRadius::same(4), // Rounded bar
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
                    CornerRadius::same(6), // Rounded segments
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
                        CornerRadius::same(4), // Rounded tooltip
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
                    CornerRadius::same(6), // Rounded hovered rectangle
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

    /// Displays the chapter title with an animated effect.
    ///
    /// This function displays the chapter title at the top of the screen, with an animation that drops down the title initially, waits for a short period, and then moves the title upward. The chapter title is dynamically fetched from the chapter metadata, including the chapter name, volume, and number, with proper formatting. The animation state and the title position are adjusted over time to create a smooth transition.
    ///
    /// # Parameters
    /// - `ui`: The UI context to draw the chapter title.
    /// - `ctx`: The application context used to request a repaint when the animation is ongoing.
    ///
    /// # Behavior
    /// - Retrieves the chapter metadata, such as the name, volume, chapter number, and title.
    /// - Displays the formatted chapter title with an animation effect. The animation involves:
    ///     - The title "drops down" at first (animation state: "drop_down").
    ///     - It stays in place for a short time (animation state: "wait").
    ///     - Finally, it moves up and out of view (animation state: "go_up").
    /// - The animation states are handled with time-based transitions to create a smooth effect.
    ///
    /// # Title Formatting
    /// - The full title is constructed by combining the chapter name, title, volume, and chapter number (if available).
    /// - The title is wrapped to fit within the available screen width and displayed with appropriate padding and alignment.
    ///
    /// # Example
    /// ```
    /// self.reader_chap_title(ui, ctx);
    /// ```
    /// This function is typically called within the UI update cycle to render and update the chapter title as the user navigates through chapters.
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
            let mut wrapped_lines: SmallVec<[String; 2]> = smallvec![];
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

    /// Displays the current chapter number with the total number of pages in the chapter.
    ///
    /// This function displays the current page number out of the total number of pages for the current chapter. It is placed at the bottom center of the screen with a background that has rounded corners and some padding. The chapter number is updated as the user navigates through the pages of the chapter.
    ///
    /// # Parameters
    /// - `ui`: The UI context to draw the chapter number.
    ///
    /// # Behavior
    /// - Displays the current page number in the format `current_page/total_pages` at the bottom center of the screen.
    /// - The background of the text has rounded corners with a semi-transparent black color to make it stand out against the rest of the UI.
    /// - The text is drawn with white color and centered in the background rectangle.
    ///
    /// # Title Formatting
    /// - The current page and total page numbers are formatted as `current_page/total_pages`.
    /// - The text is centered and drawn with a padding around it to ensure visibility and clarity.
    ///
    /// # Example
    /// ```
    /// self.reader_chap_number(ui);
    /// ```
    /// This function is typically called within the UI update cycle to render and update the chapter number as the user navigates through pages in the chapter.
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

    /// Handles the UI rendering and interaction for the manga reader panel, including page rendering, navigation, and chapter details.
    ///
    /// This function controls the overall layout and UI flow of the manga reader panel. It manages the display of pages, navigation buttons, chapter information (like title, chapter number), and handles user input for interacting with the manga content. It also takes care of loading the manga page images and showing loading indicators while the content is being prepared.
    ///
    /// # Parameters
    /// - `ctx`: The UI context used to request redrawing and handle input events.
    /// - `ui`: The UI context to render the various elements of the reader panel, such as images, buttons, and text.
    /// - `chapter_id`: The metadata for the current chapter, used to request the chapter's content and length.
    ///
    /// # Behavior
    /// - Displays the "Back" button to allow users to return to the previous screen.
    /// - Handles user input for controlling the reader panel using `reader_handle_input`.
    /// - Loads and displays the page image if available. If the image is not loaded yet, it shows a loading message and animates a "loading" GIF.
    /// - Displays the current chapter number and title at the top of the reader panel.
    /// - Renders the progress bar at the bottom of the screen to show the user's position in the chapter.
    /// - Requests the next chapter path and chapter length for reading navigation.
    ///
    /// # Title and Progress Handling
    /// - The function uses `reader_chap_title` and `reader_chap_number` to show the current chapter title and the page number out of the total page count.
    /// - The progress bar and animation are updated using `reader_progress` based on the current page and total pages in the chapter.
    ///
    /// # Example
    /// ```
    /// self.reader_panel(ctx, ui, chapter_id);
    /// ```
    /// This function is usually called in the UI rendering cycle to update the manga reader's state with each interaction or page change.
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

    /// Requests the length (number of pages) of the current chapter by extracting it from the zip file containing the images.
    ///
    /// This function checks whether the chapter length has already been determined. If not, it attempts to extract the length (total number of pages) from the zip file of the current chapter. The extracted length is stored in `reader_chapter_len` for later use.
    ///
    /// # Behavior
    /// - If `reader_chapter_len` is `None`, it attempts to retrieve the length of the chapter by calling `zip_func::extract_image_len_from_zip_gui` with the current `reader_chapter_path`.
    /// - The length of the chapter (number of pages) is stored in `reader_chapter_len`.
    /// - If an error occurs while extracting the chapter length, it defaults to `0` pages.
    ///
    /// # Example
    /// ```
    /// self.request_chapter_len();
    /// ```
    /// This function is typically called when preparing to render a chapter, ensuring that the length of the chapter (number of pages) is known before rendering.
    ///
    /// # Notes
    /// - This function is useful when the chapter images are contained in a zip archive, as it provides the total number of pages based on the zip file contents.
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

    /// Requests the file path of the chapter based on its `chapter_id` and stores it in `reader_chapter_path`.
    ///
    /// This function checks whether the `reader_chapter_path` is already set. If not, it attempts to retrieve the file path for the given `chapter_id` from a locked global storage of chapter paths (`READER_CHAPTER_PATHS`). If the path is found, it is assigned to `reader_chapter_path` for future use.
    ///
    /// # Behavior
    /// - If `reader_chapter_path` is `None`, it attempts to fetch the file path associated with the given `chapter_id` from `READER_CHAPTER_PATHS`.
    /// - If the path is found in `READER_CHAPTER_PATHS`, it is stored in `reader_chapter_path` and a log entry is created.
    /// - If no path is found for the chapter, no action is taken.
    ///
    /// # Example
    /// ```
    /// self.request_chapter_path(&chapter_id);
    /// ```
    /// This function is typically used to load or set the path of the chapter’s content (usually from a database or cache) so that the chapter can be processed further.
    ///
    /// # Notes
    /// - This function assumes that `READER_CHAPTER_PATHS` contains paths mapped by `chapter_id.id`.
    /// - The function logs an informational message when the path is successfully set.
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

    /// Preloads adjacent pages for the current chapter, asynchronously loading images into the texture cache.
    ///
    /// This function attempts to preload images for pages adjacent to the current one (based on `NUM_OF_PRELOADS`) to ensure smoother navigation in the manga reader. It uses asynchronous tasks to load and cache images in the background, minimizing the delay when transitioning between pages.
    ///
    /// The function iterates over a range of page offsets and attempts to load the pages, checking that they are neither already loaded nor currently being loaded. When an image is successfully loaded, it is inserted into the `reader_texture_cache` for future access. If the chapter has changed during the preload process, it cancels the operation for the old chapter.
    ///
    /// # Arguments
    /// - `ctx`: The context of the current UI session used for updating the UI and requesting re-renders.
    /// - `file_path`: The file path to the chapter archive that contains the image data.
    /// - `available_width`: The available width to scale the image when it is loaded.
    /// - `available_height`: The available height to scale the image when it is loaded.
    ///
    /// # Behavior
    /// - Preloads pages from the current page and pages before and after it, within the range defined by `NUM_OF_PRELOADS`.
    /// - Each page is loaded in a background task using `tokio::spawn` to avoid blocking the UI thread.
    ///
    /// # Example
    /// ```
    /// self.reader_preload(ctx, file_path, available_width, available_height);
    /// ```
    ///
    /// # Notes
    /// - The function relies on `tokio` for asynchronous execution and `zip_func::extract_image_from_zip_gui` for extracting images from a ZIP archive.
    /// - It ensures that images are only loaded if they are not already present in `reader_texture_cache` and are not currently being loaded.
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

    /// Handles keyboard input for navigating and controlling the manga reader.
    ///
    /// This function processes various key presses for navigating pages, managing chapters, and interacting with the manga reader. It supports normal navigation (next/previous page), jumping to the start/end of chapters, and controlling cache behavior. The function handles key combinations such as `Ctrl`, `Shift`, and standalone key presses for specific actions.
    ///
    /// # Key Actions:
    /// - **Right Arrow**:
    ///     - Moves to the next page, or next chapter if `Ctrl` is pressed.
    ///     - Jumps to the last page of the chapter if `Shift` is pressed.
    ///     - If at the last page, tries to load the next chapter.
    /// - **Left Arrow**:
    ///     - Moves to the previous page, or previous chapter if `Ctrl` is pressed.
    ///     - Jumps to the first page of the chapter if `Shift` is pressed.
    ///     - If at the first page, tries to load the previous chapter.
    /// - **Up Arrow**:
    ///     - Jumps forward by 5 pages, or moves to the last page if near the chapter's end.
    /// - **Down Arrow**:
    ///     - Jumps backward by 5 pages, or moves to the first page if at the beginning.
    /// - **R**:
    ///     - Clears the current page texture or the entire texture cache depending on whether `Shift` is pressed.
    /// - **Q**:
    ///     - Resets the reader and stops the current chapter.
    ///
    /// # Arguments
    /// - `ctx`: The context of the current UI session used to check input events and modify the UI state.
    /// - `ui`: The UI object used for rendering the interface and reacting to input events.
    ///
    /// # Returns
    /// - `ControlFlow::Continue(())`: Continues running the application.
    /// - `ControlFlow::Break(())`: Breaks the loop and ends the current operation, typically for chapter transitions or reset actions.
    ///
    /// # Example
    /// ```
    /// self.reader_handle_input(ctx, ui);
    /// ```
    ///
    /// # Notes
    /// - This function checks for key presses in a variety of contexts and modifies the page number or chapter accordingly.
    /// - It uses `tokio::spawn` for async tasks and locks for managing shared states (such as page numbers and chapter data).
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

    /// Resets the manga reader to its initial state.
    ///
    /// This function clears all internal state related to the current chapter, page, and any cached textures. It resets variables like the current chapter ID, page number, and texture cache, effectively returning the reader to its starting condition. This is useful when transitioning between chapters or when the user exits the reader.
    ///
    /// # Actions Performed:
    /// - Resets the current reader ID (`reader_id`).
    /// - Resets the current page (`reader_page`) to the first page.
    /// - Clears the path to the current chapter (`reader_chapter_path`) and chapter length (`reader_chapter_len`).
    /// - Removes any textures that have been downloaded but not yet displayed.
    /// - Clears the texture cache and any pages marked as loading.
    /// - Resets the title animation state (`reader_title_animation_state`).
    /// - Clears the `READER_CURRENT_CHAPTER_ID` lock.
    ///
    /// # Example
    /// ```
    /// self.reader_reset();
    /// ```
    ///
    /// # Notes
    /// - This function should be called when resetting the reader after a chapter has been read, or if the reader state needs to be cleared for any reason (e.g., exiting the reader).
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

    /// Performs a full reset of the manga reader, clearing all state and data.
    ///
    /// This function is an extension of the `reader_reset` function, and in addition to resetting the reader's page, chapter, and texture cache, it also clears the list of chapters and removes the stored chapter paths. This is useful when resetting the entire reader, such as when loading a completely new set of manga or after exiting the reader entirely.
    ///
    /// # Actions Performed:
    /// - Calls `reader_reset` to reset the reader's state (page, chapter, texture cache, etc.).
    /// - Clears the list of chapters (`reader_chapters`).
    /// - Clears the stored chapter paths (`READER_CHAPTER_PATHS`).

    /// # Example
    /// ```
    /// self.reader_full_reset();
    /// ```
    ///
    /// # Notes
    /// - This function is typically used when the reader should be completely reset, including clearing chapter information.
    fn reader_full_reset(&mut self) {
        self.reader_reset();
        self.reader_chapters.clear();
        *READER_CHAPTER_PATHS.lock() = None;
    }

    /// Main entry point for rendering the user interface in gui version of Mdown.
    ///
    /// Depending on the current state of downloading, this function either:
    /// - Displays the main configuration interface (`main_config`) if no download is in progress, or
    /// - Displays the downloading interface (`main_downloading`) if a download is active.
    ///
    /// # Actions Performed:
    /// - If the `DOWNLOADING` lock is not active (no download in progress), it calls `main_config` to render the configuration UI.
    /// - If the `DOWNLOADING` lock is active (indicating a download is in progress), it calls `main_downloading` to render the downloading UI.
    ///
    /// # Example
    /// ```
    /// self.main(ctx, ui);
    /// ```
    ///
    /// # Notes
    /// - This function acts as a switch, rendering either the configuration or downloading UI based on the current download status.
    fn main(&mut self, ctx: &Context, ui: &mut Ui) {
        if !*resolute::DOWNLOADING.lock() {
            self.main_config(ui);
        } else {
            self.main_downloading(ctx, ui);
        }
    }

    /// Renders the downloading interface of the manga reader.
    ///
    /// This function displays the current status of the ongoing manga download process,
    /// including the manga title, chapter name, current download size, and progress indicators.
    ///
    /// # Actions Performed:
    /// - Displays the manga title and chapter being downloaded (`MANGA_NAME` and `CURRENT_CHAPTER`).
    /// - Shows the current download size and the maximum size of the file in megabytes.
    /// - Displays a progress bar represented by a series of `#` characters, reflecting the number of pages downloaded (`CURRENT_PAGE` and `CURRENT_PAGE_MAX`).
    /// - Calls `main_downloading_end_panel` to show any additional UI elements related to download completion.
    /// - If the download is still ongoing and the manga ID is set, it saves the current manga ID in the `main_done_downloading` variable.
    ///
    /// # Example:
    /// ```
    /// self.main_downloading(ctx, ui);
    /// ```
    ///
    /// # Notes:
    /// - The function periodically requests repainting of the UI to reflect the progress of the download.
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

    /// Renders the end panel during the downloading process.
    ///
    /// This function displays additional information and content related to the manga chapter download.
    /// It includes the list of downloaded files, scanlation group details, and a preview image of the manga chapter.
    /// If a preview image is available, it is loaded and displayed.
    ///
    /// # Actions Performed:
    /// - Displays a list of downloaded files stored in `WEB_DOWNLOADED` lock.
    /// - Shows the scanlation group(s) involved in the download, stored in `SCANLATION_GROUPS` lock.
    /// - Checks for the presence of a preview image in the cache (`.cache\\preview\\preview.png`).
    /// - If the preview image exists and is valid, it is displayed as a texture in the UI.
    /// - If no preview image exists, attempts to load one from the cache and display it.
    ///
    /// # Example:
    /// ```
    /// self.main_downloading_end_panel(ctx, ui);
    /// ```
    ///
    /// # Notes:
    /// - The function uses a scrollable area to show all content, ensuring the UI remains scrollable when the content exceeds the available space.
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

    /// Displays the configuration panel for setting manga download parameters.
    ///
    /// This function presents an interactive UI to configure various settings for manga downloading,
    /// including manga URL, language, chapter, volume, folder path, and other parameters.
    /// Users can also toggle options like saving the manga or displaying statistics during the download.
    ///
    /// # UI Elements:
    /// - **Manga URL**: A text input for the manga's URL, which is resolved to an ID if valid.
    /// - **Language**: A text input for setting the manga's language.
    /// - **Offset**: A text input for setting the manga's page offset.
    /// - **Database Offset**: A text input for setting the database's offset.
    /// - **Title**: A text input for setting the manga's title.
    /// - **Folder**: A text input for setting the folder to store the manga in.
    /// - **Volume**: A text input for setting the manga's volume.
    /// - **Chapter**: A text input for setting the manga's chapter.
    /// - **Max Consecutive**: A text input for setting the maximum number of consecutive pages to download.
    /// - **Checkboxes**: Options for saving the manga, enabling statistics, and forcing a download.
    /// - **Download Button**: Starts the manga download process with the configured settings.
    ///
    /// # Actions Performed:
    /// - When the "Download" button is clicked, the configuration settings are captured, and the `resolve_download` function is called to begin downloading the manga.
    /// - Displays the list of downloaded manga and allows users to select one for reading.
    ///
    /// # Example:
    /// ```
    /// self.main_config(ui);
    /// ```
    ///
    /// # Notes:
    /// - This function also allows for error handling if certain fields cannot be parsed (e.g., max consecutive pages).
    /// - Once the manga is downloaded, the user can select it from the list to enter the manga reader panel.
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
                        match self.setup_max_consecutive.clone().parse() {
                            Ok(max_consecutive) => max_consecutive,
                            Err(_err) => {
                                error::suspend_error(
                                    MdownError::ConversionError(
                                        String::from("Failed to parse max_consecutive"),
                                        14004
                                    )
                                );
                                40
                            }
                        },
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

    /// Displays a help panel with basic instructions for using the downloader and reader.
    ///
    /// This function provides a simple guide to the user on how to use the manga downloader and reader features.
    /// The help panel consists of two main sections: the downloader and the reader, each with brief instructions.
    ///
    /// # UI Elements:
    /// - **Downloader Section**:
    ///   - Provides a heading ("Downloader") and instructs the user to enter a URL and press download.
    /// - **Reader Section**:
    ///   - Provides a heading ("Reader") and instructions on navigating through manga pages using the arrow keys:
    ///     - Left and right arrows to move pages by 1.
    ///     - Up and down arrows to move pages by 5.
    ///
    /// # Example:
    /// ```
    /// self.help(ui);
    /// ```
    ///
    /// # Notes:
    /// - This function is intended to provide a quick overview of how to interact with the downloader and reader.
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

    /// Manages the display of the main panel, switching between different sections based on the current panel state.
    ///
    /// This function decides which UI panel to display based on the value of `self.panel`. It can show the following sections:
    /// - **"reader"**: Displays the manga reader interface.
    /// - **"main"**: Displays the main downloader interface.
    /// - **"help"**: Displays the help interface with usage instructions.
    ///
    /// # UI Elements:
    /// - If `self.panel` is `"reader"`, the function calls `self.reader(ctx, ui)` to display the reader panel.
    /// - If `self.panel` is `"main"`, the function calls `self.main(ctx, ui)` to display the main downloader panel.
    /// - If `self.panel` is `"help"`, the function calls `self.help(ui)` to display the help panel.
    ///
    /// # Example:
    /// ```
    /// self.main_panel(ctx, ui);
    /// ```
    ///
    /// # Notes:
    /// - This function is part of the user interface navigation system and determines which section to display based on the current panel state.
    fn main_panel(&mut self, ctx: &Context, ui: &mut Ui) {
        if self.panel == *"reader" {
            self.reader(ctx, ui)
        } else if self.panel == *"main" {
            self.main(ctx, ui);
        } else if self.panel == *"help" {
            self.help(ui);
        }
    }

    /// Displays an animated GIF in the UI, updating the frames at a set interval.
    ///
    /// This function manages the display of a GIF by loading its frames and cycling through them
    /// at regular intervals. It uses a `HashMap` to store GIF frames, and each time the function is
    /// called, it updates the frame displayed based on a time delay. The frames are loaded from the
    /// provided `path` and rendered on the central panel of the UI.
    ///
    /// # Parameters:
    /// - `ctx`: The current `Context` for the UI, used to load the texture for each frame.
    /// - `path`: A string slice representing the path to the GIF's frame data.
    ///
    /// # Behavior:
    /// - The function retrieves the frames for the GIF from `self.gif_images` using the provided `path`.
    /// - If frames are not found for the given `path`, a warning is logged and the function returns early.
    /// - The GIF's frames are updated every 100 milliseconds, cycling through the frames in a loop.
    /// - The current frame is drawn on the central panel of the UI.
    /// - After displaying a frame, the function requests a repaint after the frame delay, ensuring smooth animation.
    ///
    /// # Example:
    /// ```
    /// self.show_gif(ctx, "path_to_gif.gif");
    /// ```
    ///
    /// # Notes:
    /// - The `gif_images` map holds the GIF frame textures, which should be loaded in advance.
    /// - The function assumes that the GIF frames are preloaded and stored as textures within `gif_frames`.
    /// - `self.gif_current_frame` keeps track of the index of the current frame being displayed.
    ///
    /// # Additional Information:
    /// - The function uses a `frame_delay` of 100ms for a smooth frame update interval.
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

    /// Displays a confirmation dialog asking the user if they want to quit the application.
    ///
    /// This function creates a window with the title "Do you want to quit?" and provides two buttons:
    /// "Yes" and "No". If the user clicks "Yes", the application will close. If the user clicks "No",
    /// the dialog will simply close without taking any further action.
    ///
    /// # Parameters:
    /// - `ctx`: The current `Context` for the UI, used to display the dialog and interact with the viewport.
    ///
    /// # Behavior:
    /// - The dialog is non-collapsible and non-resizable, ensuring that the user cannot resize or collapse it.
    /// - If the user clicks the "Yes" button, the `exit_show_confirmation_dialog` flag is set to `false`,
    ///   the `exit_allowed_to_close` flag is set to `true`, and the GUI will close by sending a `Close` command
    ///   to the viewport. This action logs the event with an info message.
    /// - If the user clicks the "No" button, the `exit_show_confirmation_dialog` flag is set to `false`,
    ///   and the `exit_allowed_to_close` flag is set to `false`, preventing the application from closing.
    ///
    /// # Example:
    /// ```
    /// self.exit_dialog(ctx);
    /// ```
    ///
    /// # Notes:
    /// - This function is typically used when a user attempts to exit the application, triggering a confirmation prompt.
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

/// Retrieves chapter paths for the provided manga metadata and updates the global `READER_CHAPTER_PATHS` lock with the paths of chapters found in `.cbz` files.
///
/// This function scans the directory specified in the `manga_data.mwd` field for `.cbz` files and checks whether metadata for each found file matches any existing manga.
/// It stores the mapping of manga IDs to their file paths in the global `READER_CHAPTER_PATHS` variable. The process is performed asynchronously to avoid blocking.
///
/// # Parameters:
/// - `manga_data`: The `metadata::MangaMetadata` containing metadata for the manga. The manga's path is accessed from the `mwd` field to search for the `.cbz` files.
///
/// # Behavior:
/// - The function locks the `READER_CHAPTER_PATHS` global variable and sets it to an empty `HashMap` to clear previous chapter paths before starting a new search.
/// - It then attempts to find all `.cbz` files within the directory specified by `manga_data.mwd[4..]` using the `glob` crate.
/// - For each `.cbz` file found, it calls `resolute::check_for_metadata()` to determine if the file corresponds to a known manga.
/// - If a match is found, the manga ID and file path are added to the `READER_CHAPTER_PATHS` hash map.
/// - The task is executed asynchronously using `tokio::spawn` to prevent blocking the main thread during file searching.
///
/// # Example:
/// ```rust
/// let manga_data = metadata::MangaMetadata { /* manga metadata initialization */ };
/// get_chapter_paths(manga_data);
/// ```
///
/// # Notes:
/// - This function relies on the `glob` crate for file pattern matching and the `tokio` runtime for asynchronous tasks.
/// - The `READER_CHAPTER_PATHS` global variable is used to store the chapter paths, and its value is locked during the operation to ensure thread safety.
/// - It assumes the `check_for_metadata()` function in `resolute` returns metadata for the manga based on the file path.
///
/// # Error Handling:
/// - The function does not explicitly handle errors, but logs any failures during the process using `info!` for found entries and file paths.
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

/// Loads an image from the provided byte data, resizes it to fit within the specified available width and height, and returns the texture handle.
///
/// This function accepts image data as a byte slice (`image_data`), attempts to load it into an image format, and resizes it to fit within the given `available_width` and `available_height` while preserving the aspect ratio.
/// The image is resized using the `Triangle` filter for smoothing. If the image is successfully loaded and resized, the texture handle is returned.
///
/// # Parameters:
/// - `ctx`: The `Context` object used to load and create the texture from the resized image.
/// - `image_data`: A byte slice containing the raw image data (e.g., in PNG, JPEG format).
/// - `available_width`: The width within which the image needs to fit, preserving the aspect ratio.
/// - `available_height`: The height within which the image needs to fit, preserving the aspect ratio.
///
/// # Returns:
/// - `Some(TextureHandle)` if the image is successfully loaded, resized, and converted into a texture.
/// - `None` if there was an error loading the image or if the resizing fails.
///
/// # Behavior:
/// - The function attempts to load the image using the `load_from_memory` function, which expects raw image data in memory.
/// - If the image is loaded successfully, it calculates the scaling factor based on the provided `available_width` and `available_height` to fit the image inside the given area while maintaining its aspect ratio.
/// - The image is resized using the `Triangle` filter from the `image` crate, which is a high-quality resampling filter.
/// - The resized image is then converted into a `ColorImage` and used to create a `TextureHandle` via the `ctx.load_texture` method.
/// - If an error occurs at any point in the image loading or resizing process, a warning is logged, and `None` is returned.
///
/// # Example:
/// ```rust
/// let image_data = include_bytes!("path_to_image.png"); // Example image data
/// let texture_handle = load_and_resize_image(ctx, image_data, 300.0, 200.0);
/// ```
///
/// # Notes:
/// - The function uses the `image` crate to load and resize images and the `egui` context to load the resized image as a texture.
/// - The resizing is done while preserving the image's aspect ratio, meaning the image will be scaled to fit the specified width or height, whichever is the limiting factor.
///
/// # Error Handling:
/// - If there is a failure while loading or resizing the image, a warning is logged using `warn!` and `None` is returned.
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

/// Retrieves manga data by reading and parsing a `dat.json` file.
///
/// This function attempts to load and parse manga metadata from a JSON file located at the path returned by `getter::get_dat_path()`. It returns the parsed `MangaMetadata` as a `Vec` if successful, or an appropriate error if something goes wrong during the process.
///
/// # Returns:
/// - `Ok(Vec<metadata::MangaMetadata>)` if the manga data is successfully retrieved and parsed.
/// - `Err(MdownError)` if an error occurs while retrieving the path, reading the file, or parsing the JSON.
///
/// # Errors:
/// - `MdownError::ChainedError` if there is an error when fetching the path (Error code: `14003`).
/// - `MdownError::IoError` if the `dat.json` file cannot be found or read (Error code: `14000`).
/// - `MdownError::JsonError` if there is a failure while parsing the JSON data (Error code: `14001`).
///
/// # Steps:
/// 1. Calls `getter::get_dat_path()` to retrieve the path to the `dat.json` file.
/// 2. Checks if the file exists using `std::fs::metadata`.
/// 3. If the file exists, it reads the content of the file using `resolute::get_dat_content`.
/// 4. The content is then parsed into a `metadata::Dat` struct using `serde_json::from_value`.
/// 5. If successful, the function returns the parsed `Vec<metadata::MangaMetadata>`. If there is an error at any point, an appropriate error is returned.
///
/// # Example:
/// ```rust
/// let manga_data = get_manga_data();
/// match manga_data {
///     Ok(data) => {
///         // Use the manga data
///     },
///     Err(error) => {
///         // Handle the error
///     }
/// }
/// ```
///
/// # Notes:
/// - This function is designed to handle potential errors at multiple points: fetching the file path, reading the file, and parsing the JSON data.
/// - The error handling is done through the `MdownError` enum, which helps categorize different types of errors (I/O errors, JSON parsing errors, etc.).
///
/// # Error Handling:
/// - If `getter::get_dat_path()` fails, the error is wrapped in `MdownError::ChainedError`.
/// - If the file does not exist or can't be read, an I/O error is returned with the path and error details.
/// - If parsing the JSON content fails, a `JsonError` is returned with details about the parsing failure.
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

/// Loads and returns all GIFs required for the application, mapped by their name.
///
/// This function loads GIFs from predefined paths (e.g., `LOADING_GIF` for a loading animation) and returns them in a `HashMap`. The keys of the map are the names of the GIFs (as `String`), and the values are vectors of tuples, where each tuple contains a `ColorImage` (the frame) and a `u16` (likely representing the frame's duration or delay).
///
/// # Returns:
/// - `HashMap<String, Vec<(ColorImage, u16)>>` containing the GIFs, where the key is the name of the GIF (e.g., "loading") and the value is a vector of frames and their associated durations.
///
/// # Example:
/// ```rust
/// let gifs = load_all_gifs();
/// let loading_gif = gifs.get("loading");
/// ```
///
/// # Notes:
/// - This function is designed to load all the necessary GIFs used in the application and store them in a hash map for easy access based on their name.
/// - The GIFs are loaded by calling the `load_gif` function with the appropriate file paths (e.g., `LOADING_GIF`).
/// - Currently, the function loads only one GIF (`loading`), but it can be extended to include more GIFs by inserting additional entries into the `gif_images` map.
///
/// # Error Handling:
/// - The function assumes that the `load_gif` function handles loading errors internally.
fn load_all_gifs() -> HashMap<String, Vec<(ColorImage, u16)>> {
    let mut gif_images = HashMap::new();
    gif_images.insert("loading".to_owned(), load_gif(LOADING_GIF));
    gif_images
}

/// Loads a GIF from raw byte data and converts it into a sequence of frames with transparency handling.
///
/// This function takes raw GIF data (as a byte slice), decodes the GIF into frames, and processes each frame by converting it into `ColorImage` objects. It also applies transparency handling by checking for a specific transparent color (green, RGB: `(0, 255, 0)`) and making it fully transparent in the final image.
///
/// # Parameters:
/// - `file_data: &[u8]` - The raw byte data representing the GIF to be loaded.
///
/// # Returns:
/// - `Vec<(ColorImage, u16)>` - A vector of tuples, where each tuple contains a `ColorImage` (the decoded frame) and a `u16` (the delay for the frame in hundredths of a second).
///
/// # Example:
/// ```rust
/// let gif_data: &[u8] = ...;  // The raw byte data of a GIF
/// let frames = load_gif(gif_data);
/// for (frame, delay) in frames {
///     // Process each frame and its delay
/// }
/// ```
///
/// # Notes:
/// - The function assumes that the input data is valid GIF data. It uses the `gif` crate to decode the GIF, which processes each frame individually.
/// - The transparency handling assumes that the green color `(0, 255, 0)` represents transparent pixels in the GIF, replacing them with full transparency in the output `ColorImage`.
/// - The function stores the frames in a vector along with their associated delays, and each frame is transformed into a `ColorImage` format compatible with the `egui` library for rendering.
///
/// # Errors:
/// - The function will panic if the GIF decoding fails, or if it is unable to extract the palette or encounter other errors during the frame processing.
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

/// Resolves and initiates the download process for a manga given a URL or ID.
///
/// This function processes the provided URL or ID, attempts to resolve a valid manga ID, and then fetches related manga data from a remote source. It handles both regular URL-based resolution and UUID validation. Once the manga ID is determined, it performs a network request to retrieve the manga details and returns the result.
///
/// # Parameters:
/// - `url: &str` - The URL or ID of the manga. The function will attempt to resolve a valid ID from this input.
/// - `handle_id: Box<str>` - A unique identifier for the download request, used for logging and tracing.
///
/// # Returns:
/// - `Result<String, MdownError>` - A `Result` where:
///     - `Ok(String)` contains the manga ID or another meaningful string (usually an identifier).
///     - `Err(MdownError)` contains an error variant if the process fails.
///
/// # Example:
/// ```rust
/// let url = "https://mangadex.org/title/abcd1234";
/// let handle_id = Box::from("request_1234");
/// match resolve_download(url, handle_id).await {
///     Ok(manga_id) => println!("Found manga with ID: {}", manga_id),
///     Err(error) => eprintln!("Failed to resolve manga: {}", error),
/// }
/// ```
///
/// # Notes:
/// - The function tries to match the `url` with a regular expression to extract an ID. If no match is found, it checks if the `url` is a valid UUID.
/// - If the ID is valid, the function proceeds to fetch manga metadata using the resolved ID.
/// - If an error occurs during the JSON parsing or fetching, the function will return an appropriate error.
/// - If the ID cannot be resolved from the `url`, the function logs a message and returns a `NotFoundError`.
///
/// # Errors:
/// - `MdownError::JsonError`: If the manga metadata returned by the server is not valid JSON or contains unexpected values.
/// - `MdownError::ChainedError`: If there is an issue fetching manga data (network errors, etc.).
/// - `MdownError::NotFoundError`: If no valid ID could be resolved from the provided `url`.
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
