use serenity::{app::window_aspect_ratio, nalgebra_glm, petgraph, uuid, winit};

fn main() {
    serenity::app::App::new("Serenity", 1920, 1080).run(Game::default());
}

#[derive(Default)]
struct Game {
    player_node_index: petgraph::graph::NodeIndex<u32>,
}

impl serenity::app::State for Game {
    fn initialize(&mut self, context: &mut serenity::app::Context) {
        context.world = serenity::gltf::import_gltf("resources/models/DamagedHelmet.glb").clone();

        context
            .world
            .add_root_node(serenity::world::create_camera_node(window_aspect_ratio(
                &context.window,
            )));

        self.player_node_index = context.world.add_root_node({
            serenity::world::Node {
                id: uuid::Uuid::new_v4().to_string(),
                label: "Player".to_string(),
                transform: serenity::world::Transform {
                    translation: nalgebra_glm::vec3(0.0, 0.0, 0.0),
                    ..Default::default()
                },
                components: vec![serenity::world::NodeComponent::Mesh("player".to_string())],
            }
        });
    }

    fn receive_event(
        &mut self,
        context: &mut serenity::app::Context,
        event: &serenity::winit::event::Event<()>,
    ) {
        if let winit::event::Event::WindowEvent {
            event:
                winit::event::WindowEvent::KeyboardInput {
                    input:
                        serenity::winit::event::KeyboardInput {
                            virtual_keycode: Some(keycode),
                            state,
                            ..
                        },
                    ..
                },
            ..
        } = *event
        {
            if let (winit::event::VirtualKeyCode::Escape, winit::event::ElementState::Pressed) =
                (keycode, state)
            {
                context.should_exit = true;
            }
        }
    }

    fn update(&mut self, context: &mut serenity::app::Context) {
        if context.io.is_key_pressed(winit::event::VirtualKeyCode::W) {
            context.world.scene[self.player_node_index]
                .transform
                .translation
                .x += 100.0;
        }
    }
}
