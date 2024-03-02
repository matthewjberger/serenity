#[derive(Default)]
pub struct Game;

impl phantom::app::State for Game {
    fn initialize(&mut self, context: &mut phantom::app::Context) {
        context.world = phantom::gltf::import_gltf_slice(include_bytes!("../level.glb"));
        if context.world.scenes.is_empty() {
            context.world.scenes.push(phantom::world::Scene::default());
            context.world.default_scene_index = Some(0);
        }
        if let Some(scene_index) = context.world.default_scene_index {
            context.world.add_camera_to_scenegraph(scene_index);
        }
        context.should_reload_view = true;

        let light_node = context.world.add_node();
        context.world.add_light_to_node(light_node);
        context.world.add_root_node_to_scenegraph(0, light_node);
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
        camera::camera_system(context);
    }
}
