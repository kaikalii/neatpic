mod settings;

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::exit,
};

use egui_macroquad::{
    egui::{
        style::Margin, Color32, Context, DragValue, Frame, SidePanel, Slider, TextEdit,
        TopBottomPanel, Ui, Widget,
    },
    macroquad::{self, prelude::*},
};
use image::ImageResult;
use settings::Settings;

fn conf() -> Conf {
    let path = env::args().nth(1).unwrap_or_default();
    let ctx = OpenContext::new(path.as_ref());
    let mut window_title = ctx.dir.to_string_lossy().into_owned();
    if window_title.is_empty() {
        window_title = "NeatPic".into();
    }
    let settings = Settings::load();
    Conf {
        window_title,
        window_width: settings.window_width,
        window_height: settings.window_height,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    prevent_quit();

    let mut app = ViewerApp::new();

    egui_macroquad::cfg(|ctx| {
        ctx.set_pixels_per_point(PIXELS_PER_POINT);
    });

    loop {
        clear_background(GRAY);

        app.update();

        app.viewer();

        egui_macroquad::ui(|ctx| app.show(ctx));

        egui_macroquad::draw();

        if is_quit_requested() {
            app.settings.save();
            exit(0);
        }

        next_frame().await;
    }
}

fn dpi_scale() -> f32 {
    unsafe { get_internal_gl().quad_context.dpi_scale() }
}

fn mouse_pos() -> Vec2 {
    let (x, y) = mouse_position();
    vec2(x, y)
}

const PIXELS_PER_POINT: f32 = 1.8;

struct ViewerApp {
    mouse_pos: Vec2,
    side_panel_width: f32,
    curr: Option<CurrImage>,
    images: Vec<LoadedImage>,
    settings: Settings,
}

struct CurrImage {
    index: usize,
    zoom: f32,
    dynamic_zoom: bool,
    offset: Vec2,
}

struct LoadedImage {
    path: PathBuf,
    texture: Option<ImageResult<Texture2D>>,
}

impl ViewerApp {
    fn new() -> Self {
        let path = env::args().nth(1).unwrap_or_default();
        let OpenContext {
            images: paths,
            index,
            ..
        } = OpenContext::new(path.as_ref());
        ViewerApp {
            mouse_pos: mouse_pos(),
            side_panel_width: 1.0,
            curr: index.map(|index| CurrImage {
                index,
                zoom: 1.0,
                dynamic_zoom: true,
                offset: Vec2::ZERO,
            }),
            images: paths,
            settings: Settings::load(),
        }
    }
    fn show(&mut self, ctx: &Context) {
        // Side Panel
        self.side_panel_width = SidePanel::right(0)
            .resizable(false)
            .show(ctx, |ui| self.side_panel(ui))
            .response
            .rect
            .width()
            * PIXELS_PER_POINT
            / dpi_scale();
        // Top Panel
        TopBottomPanel::top(1)
            .frame(Frame {
                fill: Color32::TRANSPARENT,
                outer_margin: Margin::same(2.0),
                inner_margin: Margin::same(2.0),
                ..Default::default()
            })
            .show(ctx, |ui| self.top_panel(ui));
    }
    fn side_panel(&mut self, ui: &mut Ui) {
        if let Some(curr) = &mut self.curr {
            if let Some(Ok(texture)) = self.images[curr.index].texture {
                TextEdit::singleline(
                    &mut self.images[curr.index].path.to_string_lossy().into_owned(),
                )
                .desired_width(150.0)
                .ui(ui);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.label("size: ");
                    DragValue::new(&mut (texture.width() as u32)).ui(ui);
                    ui.label("×");
                    DragValue::new(&mut (texture.height() as u32)).ui(ui);
                });
            }
        }
    }
    fn top_panel(&mut self, ui: &mut Ui) {
        if let Some(curr) = &mut self.curr {
            let mut zoom = curr.zoom * 100.0;
            if Slider::new(&mut zoom, 1.0..=1000.0)
                .clamp_to_range(false)
                .fixed_decimals(0)
                .logarithmic(true)
                .suffix("%")
                .ui(ui)
                .changed()
            {
                curr.zoom = zoom / 100.0;
                curr.dynamic_zoom = false;
            }
        }
    }
    fn viewer(&mut self) {
        if let Some(curr) = &mut self.curr {
            // Load image
            let loaded = &mut self.images[curr.index];
            let texture = if let Ok(texture) = loaded.texture() {
                *texture
            } else {
                return;
            };
            // Determine image size
            let image_size = vec2(texture.width(), texture.height());
            let window_size = vec2(screen_width() - self.side_panel_width, screen_height());
            let max_size = vec2(
                window_size.x.clamp(0.0, image_size.x),
                window_size.y.min(image_size.y),
            );
            if curr.dynamic_zoom {
                let width_ratio = max_size.x / image_size.x;
                let height_ratio = max_size.y / image_size.y;
                curr.zoom = width_ratio.min(height_ratio);
            };
            let size = image_size * curr.zoom;
            // Zoom
            let scroll = mouse_wheel().1 / 120.0;
            if scroll != 0.0 {
                if curr.dynamic_zoom {
                    curr.zoom = max_size.x / image_size.x;
                    curr.dynamic_zoom = false;
                }
                curr.zoom *= 1.1f32.powf(scroll);
            }
            let offset = (window_size - size) / 2.0 + curr.offset;
            // Draw image
            draw_texture_ex(
                texture,
                offset.x,
                offset.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(size),
                    ..Default::default()
                },
            );
        }
    }
    fn update(&mut self) {
        self.settings.window_width = (screen_width() * dpi_scale()) as i32;
        self.settings.window_height = (screen_height() * dpi_scale()) as i32;
        if let Some(curr) = &mut self.curr {
            // Offset
            if is_mouse_button_down(MouseButton::Left) {
                let delta = mouse_pos() - self.mouse_pos;
                curr.offset += delta;
            }
            self.mouse_pos = mouse_pos();
        }
    }
}

impl LoadedImage {
    fn texture(&mut self) -> &ImageResult<Texture2D> {
        self.texture.get_or_insert_with(|| {
            let img = image::open(&self.path)?.into_rgba8();
            let (width, height) = img.dimensions();
            Ok(Texture2D::from_rgba8(
                width as u16,
                height as u16,
                img.as_raw(),
            ))
        })
    }
}

struct OpenContext {
    images: Vec<LoadedImage>,
    index: Option<usize>,
    dir: PathBuf,
}

impl OpenContext {
    fn new(loaded: &Path) -> Self {
        let dir = if loaded.to_string_lossy().is_empty() || loaded.is_dir() {
            loaded
        } else {
            loaded.parent().unwrap()
        }
        .to_path_buf();
        let mut index = None;
        let mut images = Vec::new();
        for entry in fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            if !entry.file_type().unwrap().is_file() {
                continue;
            }
            let path = entry.path();
            let is_image_file = path
                .extension()
                .map_or(false, |a| ["png", "jpg", "bmp"].into_iter().any(|b| a == b));
            if !is_image_file {
                continue;
            }
            if path == loaded {
                index = Some(images.len());
            }
            images.push(LoadedImage {
                path,
                texture: None,
            });
        }
        if index.is_none() && !images.is_empty() {
            index = Some(0);
        }
        OpenContext { images, index, dir }
    }
}
