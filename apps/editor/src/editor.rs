pub struct Editor {
    pending_messages: Vec<Message>,
    selected_graph_node_index: Option<phantom::petgraph::graph::NodeIndex>,
    redo_stack: Vec<Command>,
    command_history: std::collections::VecDeque<Command>,
    assets: Vec<phantom::asset::Asset>,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            selected_graph_node_index: None,
            pending_messages: Vec::new(),
            redo_stack: Vec::new(),
            command_history: std::collections::VecDeque::new(),
            assets: Vec::new(),
        }
    }

    fn receive_messages(&mut self, context: &mut phantom::app::Context) {
        let messages = self.pending_messages.drain(..).collect::<Vec<_>>();
        for messages in messages.into_iter() {
            match messages {
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
                        Command::ImportGltfFile { path } => {
                            self.selected_graph_node_index = None;
                            self.redo_stack = Vec::new();
                            self.command_history = std::collections::VecDeque::new();
                            let name = path.to_string();

                            let mut asset = phantom::gltf::import_gltf_file(path);
                            asset.name = name;

                            if asset.scenes.is_empty() {
                                asset.scenes.push(phantom::asset::Scene::default());
                            }
                            phantom::asset::add_main_camera_to_scenegraph(
                                &mut context.asset.scenes,
                                &mut context.asset.metadata,
                                &mut context.asset.nodes,
                                &mut context.asset.cameras,
                                &mut context.asset.orientations,
                                &mut context.asset.transforms,
                                0,
                            );
                            context.should_reload_view = true;

                            context.asset = asset.clone();
                            self.assets.push(asset);
                        }
                    }
                }
            }
        }
    }

    fn ui(&mut self, ui: &phantom::egui::Context, context: &mut phantom::app::Context) {
        self.top_bar_ui(ui);
        self.scene_tree_ui(ui, context);
    }

    fn scene_tree_ui(&mut self, ui: &phantom::egui::Context, context: &mut phantom::app::Context) {
        phantom::egui::Window::new("Scene Tree")
            .resizable(true)
            .show(ui, |ui| {
                let scene = &context
                    .asset
                    .scenes
                    .first()
                    .expect("No scene is available!");
                ui.group(|ui| {
                    phantom::egui::ScrollArea::vertical()
                        .id_source(ui.next_auto_id())
                        .show(ui, |ui| {
                            node_ui(
                                &context.asset,
                                ui,
                                &scene.graph,
                                0.into(),
                                &mut self.selected_graph_node_index,
                            );
                        });
                });
            });
    }

    fn top_bar_ui(&mut self, ui: &phantom::egui::Context) {
        phantom::egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .show(ui, |ui| {
                phantom::egui::menu::bar(ui, |ui| {
                    phantom::egui::global_dark_light_mode_switch(ui);
                    ui.menu_button("File", |ui| {
                        if ui.button("Import asset (gltf/glb)...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("GLTF / GLB", &["gltf", "glb"])
                                .pick_file()
                            {
                                self.pending_messages.push(Message::Command(
                                    Command::ImportGltfFile {
                                        path: path.display().to_string(),
                                    },
                                ));
                                ui.close_menu();
                            }
                        }
                    });

                    ui.separator();
                });
            });
    }
}

impl phantom::app::State for Editor {
    fn initialize(&mut self, context: &mut phantom::app::Context) {
        let mut asset = phantom::gltf::import_gltf_slice(include_bytes!("../glb/helmet.glb"));

        phantom::asset::add_main_camera_to_scenegraph(
            &mut asset.scenes,
            &mut asset.metadata,
            &mut asset.nodes,
            &mut asset.cameras,
            &mut asset.orientations,
            &mut asset.transforms,
            0,
        );
        context.should_reload_view = true;

        context.asset = asset;
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
        self.ui(ui, context);
        phantom::camera::camera_system(context);
    }
}

fn node_ui(
    asset: &phantom::asset::Asset,
    ui: &mut phantom::egui::Ui,
    graph: &phantom::asset::SceneGraph,
    graph_node_index: phantom::petgraph::graph::NodeIndex,
    selected_graph_node_index: &mut Option<phantom::petgraph::graph::NodeIndex>,
) {
    let id = ui.make_persistent_id(ui.next_auto_id());
    phantom::egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
        .show_header(ui, |ui| {
            let node_index = graph[graph_node_index];
            let phantom::asset::NodeMetadata { name } = &asset.metadata[node_index];
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
                    node_ui(asset, ui, graph, child_index, selected_graph_node_index);
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
    ImportGltfFile { path: String },
}
