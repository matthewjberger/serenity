mod editor;

fn main() {
    serenity::run(editor::Editor::new());
}
