use std::{
    env, fs,
    path::{Path, PathBuf},
};

use bevy::{
    log::Level,
    prelude::{Image as BevyImage, *},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    utils::tracing::event,
};
use egui::*;
use image::ImageResult;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(bevy_egui::EguiPlugin)
        .insert_resource(ViewerApp::new())
        .add_startup_system(startup)
        .add_system(viewer)
        .add_system(ui)
        .run();
}

struct ViewerApp {
    curr: Option<CurrImage>,
    paths: Vec<PathBuf>,
}

struct CurrImage {
    index: usize,
    zoom: Option<f32>,
}

#[derive(Component)]
struct ImageSprite {
    path: PathBuf,
}

fn startup(mut commands: Commands) {
    // Spawn camera
    commands.spawn_bundle(Camera2dBundle::default());
    // Spawn image
    commands
        .spawn_bundle(SpriteBundle::default())
        .insert(ImageSprite {
            path: PathBuf::new(),
        });
}

fn viewer(
    mut images: ResMut<Assets<BevyImage>>,
    mut app: ResMut<ViewerApp>,
    // windows: Res<Windows>,
    mut sprite: Query<(&mut Handle<BevyImage>, &mut Sprite, &mut ImageSprite)>,
) {
    let app = &mut *app;
    let (mut handle, _, mut img_sprite) = sprite.single_mut();
    if let Some(curr) = &mut app.curr {
        let path = &app.paths[curr.index];
        if &img_sprite.path != path {
            match load_image(path) {
                Ok(img) => {
                    event!(Level::INFO, "loaded {path:?}");
                    *handle = images.add(img);
                }
                Err(e) => {
                    panic!("{e}")
                }
            }
            img_sprite.path = path.clone();
        }
    }
}

fn ui() {}

impl ViewerApp {
    fn new() -> Self {
        let mut app = ViewerApp {
            curr: None,
            paths: Vec::new(),
        };

        if let Some(path) = env::args().nth(1) {
            let (paths, index) = load_paths_from_one(path.as_ref());
            if let Some(index) = index {
                app.paths = paths;
                app.curr = Some(CurrImage { index, zoom: None });
            }
        }

        app
    }
    // fn ui(&mut self, ui: &mut Ui) {
    //     ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
    //         // Side bar
    //         SidePanel::right(0).resizable(false).show_inside(ui, |ui| {
    //             if let Some(curr) = &self.curr {
    //                 if let Some(handle) = &curr.handle {
    //                     let [width, height] = handle.size();
    //                     ui.label(format!("size: {width}x{height}"));
    //                 }
    //             }
    //         });
    //         // Image
    //         if let Some(curr) = &mut self.curr {
    //             let path = &self.paths[curr.index];
    //             let texture = curr.handle.get_or_insert_with(|| match load_image(path) {
    //                 Ok(image_data) => {
    //                     let handle = ui.ctx().load_texture(
    //                         path.to_string_lossy(),
    //                         image_data,
    //                         TextureFilter::Linear,
    //                     );
    //                     handle
    //                 }
    //                 Err(e) => {
    //                     panic!("{e}")
    //                 }
    //             });
    //             let [width, height] = texture.size();
    //             let image_size = vec2(width as f32, height as f32);
    //             let size = if let Some(zoom) = curr.zoom {
    //                 image_size * zoom
    //             } else {
    //                 let window_size = ui.available_size();
    //                 let aspect = image_size.x / image_size.y;
    //                 let width_ratio = image_size.x / window_size.x;
    //                 let height_ratio = image_size.y / window_size.y;
    //                 if width_ratio > height_ratio {
    //                     vec2(window_size.x, window_size.x / aspect)
    //                 } else {
    //                     vec2(window_size.y * aspect, window_size.y)
    //                 }
    //             };
    //             ui.centered_and_justified(|ui| {
    //                 Image::new(texture, size).ui(ui);
    //             });
    //         }
    //     });
    // }
}

fn load_image(path: &Path) -> ImageResult<BevyImage> {
    let img = image::open(path)?.into_rgba8();
    let (width, height) = img.dimensions();
    Ok(BevyImage::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        img.into_raw(),
        TextureFormat::Rgba8Uint,
    ))
}

fn load_paths_from_one(loaded: &Path) -> (Vec<PathBuf>, Option<usize>) {
    let dir = loaded.parent().unwrap();
    let mut i = None;
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        if !entry.file_type().unwrap().is_file() {
            continue;
        }
        let path = entry.path();
        if path == loaded {
            i = Some(paths.len());
        }
        paths.push(path);
    }
    (paths, i)
}
