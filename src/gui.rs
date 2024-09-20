use eframe::egui;
use egui::{ containers::*, * };
use lazy_static::lazy_static;
use parking_lot::Mutex;
use serde_json::Value;
use tracing::info;

use crate::{
    args::{ self, ARGS },
    error::MdownError,
    getter,
    resolute,
    utils,
    version_manager::get_current_version,
};

lazy_static! {
    pub(crate) static ref CURRENT_CHAPTER: Mutex<String> = Mutex::new(String::new());
}

pub(crate) fn start() -> Result<(), MdownError> {
    match app() {
        Ok(()) => (),
        Err(err) => eprintln!("Error gui: {}", err),
    }

    match utils::remove_cache() {
        Ok(()) => (),
        Err(err) => {
            return Err(err);
        }
    }
    *resolute::FINAL_END.lock() = true;
    Ok(())
}

pub(crate) fn app() -> Result<(), eframe::Error> {
    info!("Starting gui");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        &format!("mdown v{}", get_current_version()),
        options,
        Box::new(|_cc| Ok(Box::new(App::new(_cc))))
    )
}

#[derive(Default)]
struct App {
    show_confirmation_dialog: bool,
    allowed_to_close: bool,
    panel: String,
    url: String,
    lang: String,
    offset: String,
    database_offset: String,
    title: String,
    folder: String,
    volume: String,
    chapter: String,
    max_consecutive: String,
    saver: bool,
    stat: bool,
    force: bool,
    texture_handle: Option<TextureHandle>,
}

impl App {
    fn new(_: &eframe::CreationContext<'_>) -> Self {
        let url = ARGS.lock().url.clone();
        let lang = ARGS.lock().lang.clone();
        let offset = ARGS.lock().offset.clone();
        let database_offset = ARGS.lock().database_offset.clone();
        let title = ARGS.lock().title.clone();
        let folder = ARGS.lock().folder.clone();
        let volume = ARGS.lock().volume.clone();
        let chapter = ARGS.lock().chapter.clone();
        let max_consecutive = ARGS.lock().max_consecutive.clone();
        let saver = ARGS.lock().saver.clone();
        let stat = ARGS.lock().stat.clone();
        let force = ARGS.lock().force.clone();
        Self {
            allowed_to_close: false,
            show_confirmation_dialog: false,
            panel: "main".to_owned(),
            url: match url.as_str() {
                "UNSPECIFIED" => String::new(),
                value => value.to_owned(),
            },
            lang: lang,
            offset: offset,
            database_offset: database_offset,
            title: title,
            folder: folder,
            volume: volume,
            chapter: chapter,
            max_consecutive: max_consecutive,
            saver: saver,
            stat: stat,
            force: force,
            texture_handle: None,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let color = if ui.visuals().dark_mode {
                Color32::from_additive_luminance(196)
            } else {
                Color32::from_black_alpha(240)
            };
            menu::bar(ui, |ui| {
                ui.menu_button("Menu", |ui| {
                    if ui.button("Main").clicked() {
                        self.panel = String::from("main");
                    }
                    if ui.button("Help").clicked() {
                        self.panel = String::from("help");
                    }
                });
            });
            ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
                ui.heading(&format!("mdown v{}", get_current_version()));
            });
            if self.panel == String::from("main") {
                if !*resolute::DOWNLOADING.lock() {
                    ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
                        ScrollArea::vertical().show(ui, |ui| {
                            ui.label("Set url of manga");
                            ui.text_edit_singleline(&mut self.url);
                            if let Some(id) = utils::resolve_regex(self.url.as_str()) {
                                ui.label(format!("Found id: {}", id.as_str()));
                            }
                            if utils::is_valid_uuid(self.url.as_str()) {
                                ui.label(format!("Found id: {}", self.url));
                            }
                            ui.label("Set language of manga");
                            ui.text_edit_singleline(&mut self.lang);
                            ui.label("Set offset");
                            ui.text_edit_singleline(&mut self.offset);
                            ui.label("Set offset of database");
                            ui.text_edit_singleline(&mut self.database_offset);
                            ui.label("Set title of manga");
                            ui.text_edit_singleline(&mut self.title);
                            ui.label("Set folder to put manga in");
                            ui.text_edit_singleline(&mut self.folder);
                            ui.label("Set volume of manga");
                            ui.text_edit_singleline(&mut self.volume);
                            ui.label("Set chapter of manga");
                            ui.text_edit_singleline(&mut self.chapter);
                            ui.label("Set max consecutive of manga");
                            ui.text_edit_singleline(&mut self.max_consecutive);
                            ui.checkbox(&mut self.saver, "Saver");
                            ui.checkbox(&mut self.stat, "Statistics");
                            ui.checkbox(&mut self.force, "Force");

                            ui.add_space(5.0);
                            if ui.button("Download").clicked() {
                                let handle_id = utils::generate_random_id(12);
                                *ARGS.lock() = args::Args::from(
                                    self.url.clone(),
                                    self.lang.clone(),
                                    self.title.clone(),
                                    self.folder.clone(),
                                    self.volume.clone(),
                                    self.chapter.clone(),
                                    self.saver.clone(),
                                    self.stat.clone(),
                                    self.max_consecutive.clone(),
                                    self.force.clone(),
                                    self.offset.clone(),
                                    self.database_offset.clone()
                                );
                                let url = self.url.clone();
                                *resolute::SAVER.lock() = self.saver;
                                resolute::SCANLATION_GROUPS.lock().clear();
                                let _ = tokio::spawn(async move {
                                    let _ = resolve_download(&url, handle_id).await;
                                });
                            }

                            ui.add_space(5.0);

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
                } else {
                    ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
                        // wave thing
                        Frame::canvas(ui.style()).show(ui, |ui| {
                            ui.ctx().request_repaint();
                            let time = ui.input(|i| i.time);

                            let desired_size = (ui.available_width() / 3.0) * 2.0 * vec2(1.0, 0.35);
                            let (_id, rect) = ui.allocate_space(desired_size);

                            let to_screen = emath::RectTransform::from_to(
                                Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0),
                                rect
                            );

                            let mut shapes = vec![];

                            for &mode in &[2, 3, 5] {
                                let mode = mode as f64;
                                let n = 120;
                                let speed = 1.5;

                                let points: Vec<Pos2> = (0..=n)
                                    .map(|i| {
                                        let t = (i as f64) / (n as f64);
                                        let amp = (time * speed * mode).sin() / mode;
                                        let y =
                                            amp *
                                            (((t * std::f64::consts::TAU) / 2.0) * mode).sin();
                                        to_screen * pos2(t as f32, y as f32)
                                    })
                                    .collect();

                                let thickness = 10.0 / (mode as f32);
                                shapes.push(
                                    epaint::Shape::line(points, Stroke::new(thickness, color))
                                );
                            }

                            ui.painter().extend(shapes);
                        });

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

                            if self.texture_handle.is_some() {
                                match std::fs::metadata(".cache\\preview\\preview.png") {
                                    Ok(_metadata) => {
                                        if
                                            *resolute::CURRENT_CHAPTER.lock() !=
                                            *CURRENT_CHAPTER.lock()
                                        {
                                            *CURRENT_CHAPTER.lock() = resolute::CURRENT_CHAPTER
                                                .lock()
                                                .to_string();
                                            self.texture_handle = None;
                                        }
                                    }
                                    Err(_err) => (),
                                };
                            }
                            if let Some(texture_handle) = &self.texture_handle {
                                ui.image(texture_handle);
                            } else {
                                match image::open(".cache\\preview\\preview.png") {
                                    Ok(img) => {
                                        let img_rgba8 = img.to_rgba8();
                                        let size = [
                                            img_rgba8.width() as usize,
                                            img_rgba8.height() as usize,
                                        ];
                                        let color_image = ColorImage::from_rgba_unmultiplied(
                                            size,
                                            &img_rgba8
                                        );
                                        let texture_handle = ctx.load_texture(
                                            "my_image",
                                            color_image,
                                            TextureOptions::default()
                                        );
                                        self.texture_handle = Some(texture_handle);
                                    }
                                    Err(_err) => (),
                                }
                            }
                        })
                    });
                }
            } else if self.panel == String::from("help") {
                ui.add_space(20.0);
                ui.label("Write url and press download");
            }
        });

        if ctx.input(|i| i.viewport().close_requested()) {
            if self.allowed_to_close {
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.show_confirmation_dialog = true;
            }
        }

        if self.show_confirmation_dialog {
            egui::Window
                ::new("Do you want to quit?")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Yes").clicked() {
                            self.show_confirmation_dialog = false;
                            self.allowed_to_close = true;
                            info!("Closing gui");
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("No").clicked() {
                            self.show_confirmation_dialog = false;
                            self.allowed_to_close = false;
                        }
                    });
                });
        }
    }
}
async fn resolve_download(url: &str, handle_id: Box<str>) -> Result<String, MdownError> {
    let id;

    if let Some(id_temp) = utils::resolve_regex(&url) {
        id = id_temp.as_str().to_string();
    } else if utils::is_valid_uuid(&url) {
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
                        return Err(MdownError::JsonError(String::from("Invalid JSON")));
                    }
                };
                if let Value::Object(obj) = json_value {
                    return resolute::resolve(obj, id).await;
                } else {
                    return Err(MdownError::JsonError(String::from("Unexpected JSON value")));
                }
            }
            Err(err) => {
                return Err(err);
            }
        }
    } else {
        info!("@{} Didn't find any id", handle_id);
        return Err(MdownError::NotFoundError(String::from("ID")));
    }
}
