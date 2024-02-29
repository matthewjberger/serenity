pub struct Editor {
    pending_messages: Vec<Message>,
    selected_graph_node_index: Option<serenity::petgraph::graph::NodeIndex>,
    redo_stack: Vec<Command>,
    command_history: std::collections::VecDeque<Command>,
    gizmo_orientation: egui_gizmo::GizmoOrientation,
    gizmo_mode: egui_gizmo::GizmoMode,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            selected_graph_node_index: None,
            pending_messages: Vec::new(),
            redo_stack: Vec::new(),
            command_history: std::collections::VecDeque::new(),
            gizmo_orientation: egui_gizmo::GizmoOrientation::Global,
            gizmo_mode: egui_gizmo::GizmoMode::Translate,
        }
    }

    fn receive_messages(&mut self, context: &mut serenity::app::Context) {
        let messages = self.pending_messages.drain(..).collect::<Vec<_>>();
        for message in messages.into_iter() {
            match message {
                Message::Command(command) => {
                    self.command_history.push_back(command.clone());
                    // arbitrary command history capacity
                    if self.command_history.len() == 100 {
                        self.command_history.pop_front(); // Remove the oldest element
                    }
                    match command {
                        Command::Exit => {
                            context.should_exit = true;
                        }
                        Command::ImportGltfFile(path) => {
                            self.selected_graph_node_index = None;
                            self.redo_stack = Vec::new();
                            self.command_history = std::collections::VecDeque::new();
                            context.import_file(&path);
                            let light_node = context.world.add_node();
                            context.world.add_light_to_node(light_node);
                            context.world.add_root_node_to_scenegraph(0, light_node);
                        }
                    }
                }
            }
        }
    }
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
        self.receive_messages(context);

        serenity::egui::Area::new("viewport").show(ui, |ui| {
            ui.with_layer_id(serenity::egui::LayerId::background(), |ui| {
                if let Some(selected_graph_node_index) = self.selected_graph_node_index {
                    if let Some(scene_index) = context.world.default_scene_index {
                        let scene = &context.world.scenes[scene_index];

                        scene.graph.node_indices().for_each(|graph_node_index| {
                            if graph_node_index != selected_graph_node_index {
                                return;
                            }

                            let model_matrix = context
                                .world
                                .global_transform(&scene.graph, graph_node_index);

                            ui.group(|ui| {
                                let (_camera_position, projection, view) =
                                    serenity::world::create_camera_matrices(
                                        &context.world,
                                        scene,
                                        4.0 / 3.0, // TODO: use a real aspect ratio here
                                    );

                                let gizmo = egui_gizmo::Gizmo::new(ui.next_auto_id())
                                    .view_matrix(view.into())
                                    .projection_matrix(projection.into())
                                    .model_matrix(model_matrix.into())
                                    .orientation(self.gizmo_orientation)
                                    .mode(self.gizmo_mode);

                                if let Some(_gizmo_result) = gizmo.interact(ui) {
                                    // TODO: add gizmo controls
                                }
                            });
                        });
                    }
                }
            });
        });

        serenity::egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .show(ui, |ui| {
                serenity::egui::menu::bar(ui, |ui| {
                    serenity::egui::global_dark_light_mode_switch(ui);
                    ui.menu_button("File", |ui| {
                        if ui.button("Import asset (gltf/glb)...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("GLTF / GLB", &["gltf", "glb"])
                                .pick_file()
                            {
                                self.pending_messages.push(Message::Command(
                                    Command::ImportGltfFile(path.display().to_string()),
                                ));
                                ui.close_menu();
                            }
                        }
                    });

                    ui.separator();

                    serenity::egui::ComboBox::from_label("Mode")
                        .selected_text(format!("{:?}", self.gizmo_mode))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.gizmo_mode,
                                egui_gizmo::GizmoMode::Rotate,
                                "Rotate",
                            );
                            ui.selectable_value(
                                &mut self.gizmo_mode,
                                egui_gizmo::GizmoMode::Translate,
                                "Translate",
                            );
                            ui.selectable_value(
                                &mut self.gizmo_mode,
                                egui_gizmo::GizmoMode::Scale,
                                "Scale",
                            );
                        });

                    ui.separator();

                    serenity::egui::ComboBox::from_label("Orientation")
                        .selected_text(format!("{:?}", self.gizmo_orientation))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.gizmo_orientation,
                                egui_gizmo::GizmoOrientation::Global,
                                "Global",
                            );
                            ui.selectable_value(
                                &mut self.gizmo_orientation,
                                egui_gizmo::GizmoOrientation::Local,
                                "Local",
                            );
                        });

                    ui.separator();
                });
            });

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

#[derive(Clone, Debug)]
pub enum Message {
    Command(Command),
}

#[derive(Debug, Clone, serenity::serde::Serialize, serenity::serde::Deserialize)]
#[serde(crate = "serenity::serde")]
pub enum Command {
    Exit,
    ImportGltfFile(String),
}

#[derive(Debug, Clone, serenity::serde::Serialize, serenity::serde::Deserialize)]
#[serde(crate = "serenity::serde")]
pub enum Topic {
    Command,
    Toast,
}

impl std::fmt::Display for Topic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command => write!(f, "Command"),
            Self::Toast => write!(f, "Toast"),
        }
    }
}
