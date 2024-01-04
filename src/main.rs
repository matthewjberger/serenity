mod app;
mod gltf;
mod gpu;
mod gui;
mod io;
mod scene;
mod view;

fn main() {
    app::App::new("Looking Glass", 1920, 1080).run();
}
