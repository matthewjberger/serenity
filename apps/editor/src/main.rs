mod editor;

fn main() {
    serenity::app::App::new("serenity", 1920, 1080).run(crate::editor::Editor::new());
}
