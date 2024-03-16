#[derive(Default)]
pub struct Game;

impl phantom::app::State for Game {
    fn initialize(&mut self, context: &mut phantom::app::Context) {
        context.world = phantom::gltf::import_gltf_slice(include_bytes!("../physics.glb"));
        if context.world.scenes.is_empty() {
            context.world.scenes.push(phantom::asset::Scene::default());
            context.world.default_scene_index = Some(0);
        }
        if let Some(scene_index) = context.world.default_scene_index {
            context.world.add_camera_to_scenegraph(scene_index);
        }
        context.should_reload_view = true;

        let light_node = context.world.add_node();
        context.world.add_light_to_node(light_node);
        context.world.add_root_node_to_scenegraph(0, light_node);

        // Add rigid body and aabb to player
        let mut player_graph_node_index = None;
        let scene = &context.world.scenes[0];
        for graph_node_index in scene.graph.node_indices() {
            let node_index = scene.graph[graph_node_index];
            let metadata_index = context.world.nodes[node_index].metadata_index;
            let is_player = &context.world.metadata[metadata_index].name == &"Player";
            if is_player {
                player_graph_node_index = Some(graph_node_index);
                break;
            }
        }
        if let Some(graph_node_index) = player_graph_node_index {
            let node_index = scene.graph[graph_node_index];
            let (translation, _rotation) = context
                .world
                .global_isometry(&scene.graph, graph_node_index);
            let node = &mut context.world.nodes[node_index];
            let rigid_body_index = context.world.physics.add_rigid_body(translation);
            node.rigid_body_index = Some(rigid_body_index);
        }
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
