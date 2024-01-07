mod editor;

fn main() {
    serenity::app::App::new("Serenity", 1920, 1080).run(crate::editor::Editor::new());
}
