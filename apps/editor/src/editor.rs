pub struct Editor {
    pending_messages: Vec<Message>,
    selected_graph_node_index: Option<phantom::petgraph::graph::NodeIndex>,
    redo_stack: Vec<Command>,
    command_history: std::collections::VecDeque<Command>,
    assets: Vec<phantom::world::World>,
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
                                asset.scenes.push(phantom::world::Scene::default());
                            }
                            asset.add_main_camera_to_scenegraph(0);
                            context.should_reload_view = true;

                            let light_node = asset.add_node();
                            asset.add_light_to_node(light_node);
                            asset.add_root_node_to_scenegraph(0, light_node);
                            context.world = asset.clone();
                            self.assets.push(asset);
                        }
                    }
                }
            }
        }
    }
}

impl phantom::app::State for Editor {
    fn initialize(&mut self, context: &mut phantom::app::Context) {
        let mut asset = phantom::gltf::import_gltf_slice(include_bytes!("../glb/helmet.glb"));

        asset.add_main_camera_to_scenegraph(0);
        context.should_reload_view = true;

        let light_node = asset.add_node();
        asset.add_light_to_node(light_node);
        asset.add_root_node_to_scenegraph(0, light_node);

        asset.load_sdf_font(
            "./apps/editor/fonts/font.fnt",
            "./apps/editor/fonts/font_sdf_rgba.png",
        );

        context.world = asset;
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

            if matches!(
                (key_code, state),
                (
                    phantom::winit::keyboard::KeyCode::KeyF,
                    phantom::winit::event::ElementState::Pressed
                )
            ) {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("GLTF / GLB", &["gltf", "glb"])
                    .pick_file()
                {
                    self.pending_messages
                        .push(Message::Command(Command::ImportGltfFile {
                            path: path.display().to_string(),
                        }));
                }
            }
        }
    }

    fn update(&mut self, context: &mut phantom::app::Context) {
        self.receive_messages(context);
        phantom::camera::camera_system(context);
    }
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
