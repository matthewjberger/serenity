mod editor;

fn main() {
    dragonglass::app::App::new("Dragonglass", 1920, 1080).run(crate::editor::Editor::new());
}
