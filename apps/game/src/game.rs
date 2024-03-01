#[derive(Default)]
pub struct Game;

impl phantom::app::State for Game {
    fn receive_event(
        &mut self,
        context: &mut phantom::app::Context,
        event: &phantom::winit::event::Event<()>,
    ) {
        if let phantom::winit::event::Event::WindowEvent {
            event:
                phantom::winit::event::WindowEvent::KeyboardInput {
                    event:
                        phantom::winit::event::KeyEvent {
                            physical_key: phantom::winit::keyboard::PhysicalKey::Code(key_code),
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
                    phantom::winit::keyboard::KeyCode::Escape,
                    phantom::winit::event::ElementState::Pressed
                )
            ) {
                context.should_exit = true;
            }
        }
    }

    fn update(&mut self, _context: &mut phantom::app::Context, ui: &phantom::egui::Context) {
        phantom::egui::Window::new("Game").show(ui, |ui| {
            ui.label("Place ui controls here");
        });
    }
}
