fn main() {
    serenity::app::App::new("Serenity", 1920, 1080).run(Game);
}

#[derive(Default)]
pub struct Game;

impl serenity::app::State for Game {
    fn initialize(&mut self, context: &mut serenity::app::Context) {
        context.import_file("resources/gltf/physics.glb");

        let Some(scene_index) = context.world.default_scene_index else {
            return;
        };
        let scene = &context.world.scenes[scene_index];

        // Add rigid body and aabb to player
        for graph_node_index in scene.graph.node_indices() {
            let node_index = scene.graph[graph_node_index];
            let node = &mut context.world.nodes[node_index];
            let metadata = &context.world.metadata[node.metadata_index];
            let (global_translation, _global_rotation) = context
                .world
                .global_isometry(&scene.graph, graph_node_index);
            if metadata.name == "Player" || metadata.name == "Ground" {
                let position = global_translation;
                let node = &mut context.world.nodes[node_index];

                let rigid_body_index = context.world.physics.add_rigid_body(position);
                node.rigid_body_index = Some(rigid_body_index);

                if let Some(mesh_index) = node.mesh_index {
                    let mesh = &context.world.meshes[mesh_index];
                    let mut aabb = serenity::physics::AxisAlignedBoundingBox::new(
                        serenity::nalgebra_glm::Vec3::new(0.0, 0.0, 0.0),
                        serenity::nalgebra_glm::Vec3::new(0.0, 0.0, 0.0),
                    );
                    mesh.primitives.iter().for_each(|primitive| {
                        let vertices = &context.world.vertices[primitive.vertex_offset
                            ..(primitive.vertex_offset + primitive.number_of_vertices)];
                        vertices.into_iter().for_each(|vertex| {
                            aabb.expand_to_include_vertex(vertex);
                        });
                    });
                    let aabb_index = context.world.physics.add_aabb(aabb);
                    context.world.physics.bodies[rigid_body_index].aabb_index = aabb_index;

                    if metadata.name == "Player" {
                        context.world.physics.bodies[rigid_body_index].dynamic = true;
                    }

                    if metadata.name == "Ground" {
                        context.world.physics.bodies[rigid_body_index].dynamic = false;
                    }
                }
            }
        }
    }

    fn receive_event(
        &mut self,
        context: &mut serenity::app::Context,
        event: &serenity::winit::event::Event<()>,
    ) {
        if let serenity::winit::event::Event::WindowEvent {
            event:
                serenity::winit::event::WindowEvent::KeyboardInput {
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
            if let (
                serenity::winit::event::VirtualKeyCode::Escape,
                serenity::winit::event::ElementState::Pressed,
            ) = (keycode, state)
            {
                context.should_exit = true;
            }

            if let (
                serenity::winit::event::VirtualKeyCode::F3,
                serenity::winit::event::ElementState::Pressed,
            ) = (keycode, state)
            {
                context.world.show_debug = !context.world.show_debug;
            }
        }
    }

    fn update(&mut self, context: &mut serenity::app::Context) {
        camera_system(context);
    }
}

fn camera_system(context: &mut serenity::app::Context) {
    let Some(scene_index) = context.world.default_scene_index else {
        return;
    };

    let scene = &context.world.scenes[scene_index];

    let camera_node_index = scene.graph[scene
        .default_camera_graph_node_index
        .expect("No camera is available in the active scene!")];
    let camera_node = &mut context.world.nodes[camera_node_index];

    let metadata = &context.world.metadata[camera_node.metadata_index];
    if metadata.name != "Main Camera" {
        return;
    }

    let transform = &mut context.world.transforms[camera_node.transform_index];
    let camera = &mut context.world.cameras[camera_node.camera_index.unwrap()];

    let mut sync_transform = false;
    let speed = 10.0 * context.delta_time as f32;

    if context
        .io
        .is_key_pressed(serenity::winit::event::VirtualKeyCode::W)
    {
        camera.orientation.offset -= camera.orientation.direction() * speed;
        sync_transform = true;
    }

    if context
        .io
        .is_key_pressed(serenity::winit::event::VirtualKeyCode::A)
    {
        camera.orientation.offset += camera.orientation.right() * speed;
        sync_transform = true;
    }

    if context
        .io
        .is_key_pressed(serenity::winit::event::VirtualKeyCode::S)
    {
        camera.orientation.offset += camera.orientation.direction() * speed;
        sync_transform = true;
    }

    if context
        .io
        .is_key_pressed(serenity::winit::event::VirtualKeyCode::D)
    {
        camera.orientation.offset -= camera.orientation.right() * speed;
        sync_transform = true;
    }

    if context
        .io
        .is_key_pressed(serenity::winit::event::VirtualKeyCode::Space)
    {
        camera.orientation.offset += camera.orientation.up() * speed;
        sync_transform = true;
    }

    if context
        .io
        .is_key_pressed(serenity::winit::event::VirtualKeyCode::LShift)
    {
        camera.orientation.offset -= camera.orientation.up() * speed;
        sync_transform = true;
    }

    camera
        .orientation
        .zoom(6.0 * context.io.mouse.wheel_delta.y * (context.delta_time as f32));

    if context.io.mouse.is_middle_clicked {
        camera
            .orientation
            .pan(&(context.io.mouse.position_delta * context.delta_time as f32));
        sync_transform = true;
    }

    if context.io.mouse.is_right_clicked {
        let mut delta = context.io.mouse.position_delta * context.delta_time as f32;
        delta.x *= -1.0;
        delta.y *= -1.0;
        camera.orientation.rotate(&delta);
        sync_transform = true;
    }

    if sync_transform {
        transform.translation = camera.orientation.position();
        transform.rotation = camera.orientation.look_at_offset();
    }
}
