#[derive(Default)]
pub struct Game;

impl phantom::app::State for Game {
    fn initialize(&mut self, context: &mut phantom::app::Context) {
        let mut asset = phantom::gltf::import_gltf_slice(include_bytes!("../glb/helmet.glb"));

        asset.add_main_camera_to_scenegraph(0);

        context.world = asset;
        context.should_reload_view = true;
    }

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

    fn update(&mut self, context: &mut phantom::app::Context, _ui: &phantom::egui::Context) {
        phantom::camera::camera_system(context);
    }
}
