fn main() {
    serenity::app::run(Editor::default());
}

#[derive(Default)]
pub struct Editor {
    selected_graph_node_index: Option<serenity::petgraph::graph::NodeIndex>,
}

impl serenity::app::State for Editor {
    fn initialize(&mut self, context: &mut serenity::app::Context) {
        context.import_file("glb/helmet.glb");
        let light_node = context.world.add_node();
        context.world.add_light_to_node(light_node);
        context.world.add_root_node_to_scenegraph(0, light_node);
    }

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

    fn update(&mut self, context: &mut serenity::app::Context, ui: &serenity::egui::Context) {
        serenity::egui::SidePanel::left("left_panel")
            .resizable(true)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.heading("Scene Tree");
                if let Some(scene_index) = context.world.default_scene_index {
                    let scene = &context.world.scenes[scene_index];
                    ui.group(|ui| {
                        serenity::egui::ScrollArea::vertical()
                            .id_source(ui.next_auto_id())
                            .show(ui, |ui| {
                                node_ui(
                                    &context.world,
                                    ui,
                                    &scene.graph,
                                    0.into(),
                                    &mut self.selected_graph_node_index,
                                );
                            });
                    });
                }

                ui.allocate_space(ui.available_size());
            });

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
        .is_key_pressed(serenity::winit::keyboard::KeyCode::KeyW)
    {
        camera.orientation.offset -= camera.orientation.direction() * speed;
        sync_transform = true;
    }

    if context
        .io
        .is_key_pressed(serenity::winit::keyboard::KeyCode::KeyA)
    {
        camera.orientation.offset += camera.orientation.right() * speed;
        sync_transform = true;
    }

    if context
        .io
        .is_key_pressed(serenity::winit::keyboard::KeyCode::KeyS)
    {
        camera.orientation.offset += camera.orientation.direction() * speed;
        sync_transform = true;
    }

    if context
        .io
        .is_key_pressed(serenity::winit::keyboard::KeyCode::KeyD)
    {
        camera.orientation.offset -= camera.orientation.right() * speed;
        sync_transform = true;
    }

    if context
        .io
        .is_key_pressed(serenity::winit::keyboard::KeyCode::Space)
    {
        camera.orientation.offset += camera.orientation.up() * speed;
        sync_transform = true;
    }

    if context
        .io
        .is_key_pressed(serenity::winit::keyboard::KeyCode::ShiftLeft)
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

fn node_ui(
    world: &serenity::world::World,
    ui: &mut serenity::egui::Ui,
    graph: &serenity::world::SceneGraph,
    graph_node_index: serenity::petgraph::graph::NodeIndex,
    selected_graph_node_index: &mut Option<serenity::petgraph::graph::NodeIndex>,
) {
    let id = ui.make_persistent_id(ui.next_auto_id());
    serenity::egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
        .show_header(ui, |ui| {
            let node_index = graph[graph_node_index];
            let serenity::world::NodeMetadata { name } = &world.metadata[node_index];
            let selected = selected_graph_node_index
                .as_ref()
                .map(|index| *index == graph_node_index)
                .unwrap_or_default();
            let response = ui.selectable_label(selected, format!("🔴 {name}"));
            if response.clicked() {
                *selected_graph_node_index = Some(graph_node_index);
            }
            response.context_menu(|ui| {
                if ui.button("Add child node").clicked() {
                    //
                }
            });
        })
        .body(|ui| {
            graph
                .neighbors_directed(graph_node_index, serenity::petgraph::Direction::Outgoing)
                .for_each(|child_index| {
                    node_ui(world, ui, graph, child_index, selected_graph_node_index);
                });
        });
}
