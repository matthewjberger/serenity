fn main() {
    serenity::run(Sandbox);
}

#[derive(Default)]
pub struct Sandbox;

impl serenity::app::State for Sandbox {
    fn receive_event(
        &mut self,
        context: &mut serenity::app::Context,
        event: &serenity::winit::event::Event<()>,
    ) {
        if let serenity::winit::event::Event::WindowEvent {
            event:
                serenity::winit::event::WindowEvent::KeyboardInput {
                    event:
                        serenity::winit::event::KeyEvent {
                            physical_key: serenity::winit::keyboard::PhysicalKey::Code(key_code),
                            state,
                            ..
                        },
                    ..
                },
            ..
        } = *event
        {
            if matches!(
                (key_code, state),
                (
                    serenity::winit::keyboard::KeyCode::Escape,
                    serenity::winit::event::ElementState::Pressed
                )
            ) {
                context.should_exit = true;
            }
        }
    }

    fn update(&mut self, _context: &mut serenity::app::Context, ui: &serenity::egui::Context) {
        serenity::egui::Window::new("Sandbox").show(ui, |ui| {
            ui.label("Place ui controls here");
        });
    }
}
