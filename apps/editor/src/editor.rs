pub struct Editor {
    pending_messages: Vec<Message>,
    selected_graph_node_index: Option<phantom::petgraph::graph::NodeIndex>,
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

    fn receive_messages(&mut self, context: &mut phantom::app::Context) {
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
                        Command::ImportGltfFile(_path) => {
                            self.selected_graph_node_index = None;
                            self.redo_stack = Vec::new();
                            self.command_history = std::collections::VecDeque::new();
                            // TODO: import the file
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

impl phantom::app::State for Editor {
    fn initialize(&mut self, context: &mut phantom::app::Context) {
        context.world = phantom::gltf::import_gltf_slice(include_bytes!("../glb/helmet.glb"));
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

    fn update(&mut self, context: &mut phantom::app::Context, ui: &phantom::egui::Context) {
        self.receive_messages(context);

        phantom::egui::Area::new("viewport").show(ui, |ui| {
            ui.with_layer_id(phantom::egui::LayerId::background(), |ui| {
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
                                    phantom::world::create_camera_matrices(
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

        phantom::egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .show(ui, |ui| {
                phantom::egui::menu::bar(ui, |ui| {
                    phantom::egui::global_dark_light_mode_switch(ui);
                    ui.menu_button("File", |ui| {
                        if ui.button("Import asset (gltf/glb)...").clicked() {
                            // if let Some(path) = rfd::FileDialog::new()
                            //     .add_filter("GLTF / GLB", &["gltf", "glb"])
                            //     .pick_file()
                            // {
                            //     self.pending_messages.push(Message::Command(
                            //         Command::ImportGltfFile(path.display().to_string()),
                            //     ));
                            //     ui.close_menu();
                            // }
                        }
                    });

                    ui.separator();

                    phantom::egui::ComboBox::from_label("Mode")
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

                    phantom::egui::ComboBox::from_label("Orientation")
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

        phantom::egui::Window::new("Scene Tree")
            .resizable(true)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.heading("Scene Tree");
                if let Some(scene_index) = context.world.default_scene_index {
                    let scene = &context.world.scenes[scene_index];
                    ui.group(|ui| {
                        phantom::egui::ScrollArea::vertical()
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

        camera::camera_system(context);
    }
}

fn node_ui(
    world: &phantom::world::World,
    ui: &mut phantom::egui::Ui,
    graph: &phantom::world::SceneGraph,
    graph_node_index: phantom::petgraph::graph::NodeIndex,
    selected_graph_node_index: &mut Option<phantom::petgraph::graph::NodeIndex>,
) {
    let id = ui.make_persistent_id(ui.next_auto_id());
    phantom::egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
        .show_header(ui, |ui| {
            let node_index = graph[graph_node_index];
            let phantom::world::NodeMetadata { name } = &world.metadata[node_index];
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
                .neighbors_directed(graph_node_index, phantom::petgraph::Direction::Outgoing)
                .for_each(|child_index| {
                    node_ui(world, ui, graph, child_index, selected_graph_node_index);
                });
        });
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum Message {
    Command(Command),
}

#[derive(Debug, Clone, phantom::serde::Serialize, phantom::serde::Deserialize)]
#[serde(crate = "phantom::serde")]
pub enum Command {
    Exit,
    ImportGltfFile(String),
}
