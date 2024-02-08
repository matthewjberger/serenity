fn main() {
    serenity::app::App::new("Serenity", 1920, 1080).run(Game::default());
}

#[derive(Default)]
pub struct Game;

impl serenity::app::State for Game {
    fn initialize(&mut self, context: &mut serenity::app::Context) {
        context.import_file("resources/gltf/backrooms.glb");
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
                context.debug_visible = !context.debug_visible;
            }
        }
    }

    fn update(&mut self, context: &mut serenity::app::Context) {
        camera_system(context);
    }
}

fn camera_system(context: &mut serenity::app::Context) {
    let scene_index = context.active_scene_index;
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
