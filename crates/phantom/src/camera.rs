pub fn camera_system(context: &mut crate::app::Context) {
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

    if context.io.is_key_pressed(winit::keyboard::KeyCode::KeyW) {
        camera.orientation.offset -= camera.orientation.direction() * speed;
        sync_transform = true;
    }

    if context.io.is_key_pressed(winit::keyboard::KeyCode::KeyA) {
        camera.orientation.offset += camera.orientation.right() * speed;
        sync_transform = true;
    }

    if context.io.is_key_pressed(winit::keyboard::KeyCode::KeyS) {
        camera.orientation.offset += camera.orientation.direction() * speed;
        sync_transform = true;
    }

    if context.io.is_key_pressed(winit::keyboard::KeyCode::KeyD) {
        camera.orientation.offset -= camera.orientation.right() * speed;
        sync_transform = true;
    }

    if context.io.is_key_pressed(winit::keyboard::KeyCode::Space) {
        camera.orientation.offset += camera.orientation.up() * speed;
        sync_transform = true;
    }

    if context
        .io
        .is_key_pressed(winit::keyboard::KeyCode::ShiftLeft)
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
