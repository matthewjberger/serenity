mod app;
mod gltf;
mod gpu;
mod gui;
mod scene;
mod view;

fn main() {
    env_logger::init();
    app::App::new("Looking Glass", 1920, 1080).run();
}
