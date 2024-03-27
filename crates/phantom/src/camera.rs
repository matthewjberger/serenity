pub fn camera_system(context: &mut crate::app::Context) {
    let scene = &context
        .world
        .scenes
        .first()
        .expect("No scene is available!");

    let camera_node_index = scene.graph[scene
        .default_camera_graph_node_index
        .expect("No camera is available in the active scene!")];
    let camera_node = &mut context.world.nodes[camera_node_index];

    let metadata = &context.world.metadata[camera_node.metadata_index];
    if metadata.name != "Main Camera" {
        return;
    }

    let transform = &mut context.world.transforms[camera_node.transform_index];
    let orientation = &mut context.world.orientations[camera_node.orientation_index.unwrap()];

    let mut sync_transform = false;
    let speed = 10.0 * context.delta_time as f32;

    if context.io.is_key_pressed(winit::keyboard::KeyCode::KeyW) {
        orientation.offset -= orientation.direction() * speed;
        sync_transform = true;
    }

    if context.io.is_key_pressed(winit::keyboard::KeyCode::KeyA) {
        orientation.offset += orientation.right() * speed;
        sync_transform = true;
    }

    if context.io.is_key_pressed(winit::keyboard::KeyCode::KeyS) {
        orientation.offset += orientation.direction() * speed;
        sync_transform = true;
    }

    if context.io.is_key_pressed(winit::keyboard::KeyCode::KeyD) {
        orientation.offset -= orientation.right() * speed;
        sync_transform = true;
    }

    if context.io.is_key_pressed(winit::keyboard::KeyCode::Space) {
        orientation.offset += orientation.up() * speed;
        sync_transform = true;
    }

    if context
        .io
        .is_key_pressed(winit::keyboard::KeyCode::ShiftLeft)
    {
        orientation.offset -= orientation.up() * speed;
        sync_transform = true;
    }

    orientation.zoom(6.0 * context.io.mouse.wheel_delta.y * (context.delta_time as f32));

    if context.io.mouse.is_middle_clicked {
        orientation.pan(&(context.io.mouse.position_delta * context.delta_time as f32));
        sync_transform = true;
    }

    if context.io.mouse.is_right_clicked {
        let mut delta = context.io.mouse.position_delta * context.delta_time as f32;
        delta.x *= -1.0;
        delta.y *= -1.0;
        orientation.rotate(&delta);
        sync_transform = true;
    }

    if sync_transform {
        transform.translation = orientation.position();
        transform.rotation = orientation.look_at_offset();
    }
}
