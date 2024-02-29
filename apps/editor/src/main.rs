mod editor;

fn main() {
    serenity::app::run(editor::Editor::new());
}
