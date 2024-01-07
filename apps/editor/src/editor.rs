use dragonglass::{egui, nalgebra_glm, petgraph, winit};

pub struct Editor {
    broker: Broker,
    client: ClientHandle,
    selected: Option<petgraph::graph::NodeIndex>,
    console_history: Vec<String>,
    console_command: String,
}

impl Editor {
    pub fn new() -> Self {
        let mut broker = Broker::default();
        let client: ClientHandle = Client::new(10).into();
        broker.subscribe(&Topic::Command.to_string(), &client);
        Self {
            broker,
            client,
            selected: None,
            console_history: vec!["Welcome to the DragonGlass Editor!".to_string()],
            console_command: "Type /help for more commands.".to_string(),
        }
    }

    fn publish_exit_command(&mut self) {
        self.broker
            .publish(&Topic::Command.to_string(), Message::Command(Command::Exit));
    }

    fn publish_import_gltf_command(&mut self, path: &str) {
        self.broker.publish(
            &Topic::Command.to_string(),
            Message::Command(Command::ImportGltfFile(path.to_string())),
        );
    }

    fn receive_messages(
        &mut self,
        context: &mut dragonglass::app::Context,
        renderer: &mut dragonglass::render::Renderer,
    ) {
        while let Some(message) = self.client.borrow().next_message() {
            match message {
                Message::Command(command) => match command {
                    Command::Exit => {
                        context.should_exit = true;
                    }
                    Command::ImportGltfFile(path) => {
                        context.scene = dragonglass::gltf::import_gltf(&path).unwrap()[0].clone();
                        if !context.scene.has_camera() {
                            context
                                .scene
                                .add_root_node(dragonglass::scene::create_camera_node(
                                    renderer.gpu.aspect_ratio(),
                                ));
                        }
                        renderer.view.import_scene(&context.scene, &renderer.gpu);
                    }
                },
            }
        }
    }
}

impl dragonglass::app::State for Editor {
    fn receive_event(
        &mut self,
        _context: &mut dragonglass::app::Context,
        event: &winit::event::Event<()>,
    ) {
        if let winit::event::Event::WindowEvent {
            event:
                winit::event::WindowEvent::KeyboardInput {
                    input:
                        dragonglass::winit::event::KeyboardInput {
                            virtual_keycode: Some(keycode),
                            state,
                            ..
                        },
                    ..
                },
            ..
        } = *event
        {
            if let (winit::event::VirtualKeyCode::Escape, winit::event::ElementState::Pressed) =
                (keycode, state)
            {
                self.publish_exit_command();
            }
        }
    }

    fn update(
        &mut self,
        context: &mut dragonglass::app::Context,
        renderer: &mut dragonglass::render::Renderer,
    ) {
        self.receive_messages(context, renderer);
        camera_system(context);
    }

    fn ui(&mut self, context: &mut dragonglass::app::Context, ui_context: &mut egui::Context) {
        egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .show(ui_context, |ui| {
                egui::menu::bar(ui, |ui| {
                    egui::global_dark_light_mode_switch(ui);
                    ui.menu_button("File", |ui| {
                        if ui.button("Import asset (gltf/glb)...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("GLTF / GLB", &["gltf", "glb"])
                                .pick_file()
                            {
                                self.publish_import_gltf_command(&path.display().to_string());
                            }
                        }
                    });
                });
            });
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .show(ui_context, |ui| {
                ui.set_width(ui.available_width());

                ui.heading("Scene Tree");
                if context.scene.graph.node_count() > 0 {
                    ui.group(|ui| {
                        egui::ScrollArea::vertical()
                            .id_source(ui.next_auto_id())
                            .show(ui, |ui| {
                                node_ui(ui, &context.scene.graph, 0.into(), &mut self.selected);
                            });
                    });
                }
            });
        egui::SidePanel::right("right_panel")
            .resizable(true)
            .show(ui_context, |ui| {
                ui.set_width(ui.available_width());

                ui.heading("Properties");
                ui.group(|ui| {
                    egui::ScrollArea::vertical()
                        .id_source(ui.next_auto_id())
                        .show(ui, |ui| {
                            if let Some(selected) = self.selected {
                                if let Some(node) = context.scene.graph.node_weight(selected) {
                                    ui.group(|ui| {
                                        ui.heading("Transform");
                                        ui.label(format!("Name: {}", node.name));
                                        ui.label(format!(
                                            "Position: {:?}",
                                            node.transform.translation
                                        ));
                                        ui.label(format!(
                                            "Rotation: {:?}",
                                            node.transform.rotation
                                        ));
                                        ui.label(format!("Scale: {:?}", node.transform.scale));
                                    });
                                }
                            }
                        });
                });
            });

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .show(ui_context, |ui| {
                ui.set_height(ui.available_height());

                ui.group(|ui| {
                    ui.heading("Console");
                    ui.horizontal(|ui| {
                        let input = ui.text_edit_singleline(&mut self.console_command);
                        if input.lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter))
                            || ui.button("Run").clicked()
                        {
                            self.console_history
                                .push(format!(">> {}\n", self.console_command));
                            self.console_history.push(self.console_command.to_string());
                            self.console_command.clear();
                            ui.memory_mut(|memory| memory.request_focus(input.id));
                        }
                    });

                    ui.group(|ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, true])
                            .max_height(ui.available_height() - 30.0)
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                for line in self.console_history.iter() {
                                    ui.label(line);
                                    ui.colored_label(egui::Color32::GREEN, "Achievement unlocked!");
                                }
                            });
                    });
                });
            });
    }
}

fn node_ui(
    ui: &mut egui::Ui,
    graph: &petgraph::graph::Graph<dragonglass::scene::Node, ()>,
    node_index: petgraph::graph::NodeIndex,
    selected_index: &mut Option<petgraph::graph::NodeIndex>,
) {
    let node = &graph[node_index];
    let id = ui.make_persistent_id(ui.next_auto_id());
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
        .show_header(ui, |ui| {
            let name = node.name.to_string();
            let selected = selected_index
                .as_ref()
                .map(|index| *index == node_index)
                .unwrap_or_default();
            let response = ui.selectable_label(selected, format!("🔴 {name}"));
            if response.clicked() {
                *selected_index = Some(node_index);
            }
        })
        .body(|ui| {
            graph
                .neighbors_directed(node_index, petgraph::Direction::Outgoing)
                .for_each(|child_index| {
                    node_ui(ui, graph, child_index, selected_index);
                });
        });
}

fn camera_system(context: &mut dragonglass::app::Context) {
    context.scene.walk_dfs_mut(|node, _| {
        node.components.iter_mut().for_each(|component| {
            if let dragonglass::scene::NodeComponent::Camera(camera) = component {
                let speed = (1.0_f64 * context.delta_time) as f32;

                if context.io.is_key_pressed(winit::event::VirtualKeyCode::W) {
                    camera.orientation.offset -= camera.orientation.direction() * speed;
                }
                if context.io.is_key_pressed(winit::event::VirtualKeyCode::A) {
                    camera.orientation.offset += camera.orientation.right() * speed;
                }
                if context.io.is_key_pressed(winit::event::VirtualKeyCode::S) {
                    camera.orientation.offset += camera.orientation.direction() * speed;
                }
                if context.io.is_key_pressed(winit::event::VirtualKeyCode::D) {
                    camera.orientation.offset -= camera.orientation.right() * speed;
                }
                if context
                    .io
                    .is_key_pressed(winit::event::VirtualKeyCode::Space)
                {
                    camera.orientation.offset += camera.orientation.up() * speed;
                }
                if context
                    .io
                    .is_key_pressed(winit::event::VirtualKeyCode::LShift)
                {
                    camera.orientation.offset -= camera.orientation.up() * speed;
                }

                camera
                    .orientation
                    .zoom(6.0 * context.io.mouse.wheel_delta.y * (context.delta_time as f32));

                if context.io.mouse.is_middle_clicked {
                    camera
                        .orientation
                        .pan(&(context.io.mouse.position_delta * context.delta_time as f32));
                }
                node.transform.translation = camera.orientation.position();

                if context.io.mouse.is_right_clicked {
                    if context
                        .io
                        .is_key_pressed(winit::event::VirtualKeyCode::LAlt)
                    {
                        camera.orientation.offset = nalgebra_glm::Vec3::new(0.0, 0.0, 0.0);
                    }

                    let mut delta = context.io.mouse.position_delta * context.delta_time as f32;
                    delta.x *= -1.0;
                    camera.orientation.rotate(&delta);
                }

                node.transform.rotation = camera.orientation.look_at_offset();
            }
        });
    });
}

#[derive(Default)]
pub struct Broker {
    pub subscribers:
        std::collections::HashMap<String, Vec<std::rc::Weak<std::cell::RefCell<Client>>>>,
}

impl Broker {
    pub fn subscribe(&mut self, topic: &str, client: &ClientHandle) {
        self.subscribers
            .entry(topic.to_string())
            .or_default()
            .push(std::rc::Rc::downgrade(client));
    }

    pub fn publish(&mut self, topic: &str, message: Message) {
        if let Some(subscribers) = self.subscribers.get_mut(topic) {
            subscribers.retain(|subscriber_weak| match subscriber_weak.upgrade() {
                Some(subscriber) => {
                    let subscriber = subscriber.borrow_mut();
                    if subscriber.event_queue.borrow().len() == subscriber.ringbuffer_size {
                        subscriber.event_queue.borrow_mut().pop_front();
                    }
                    subscriber
                        .event_queue
                        .borrow_mut()
                        .push_back(message.clone());
                    true
                }
                None => false,
            });
        }
    }
}

pub struct Client {
    pub id: uuid::Uuid,
    pub event_queue: std::cell::RefCell<std::collections::VecDeque<Message>>,
    pub ringbuffer_size: usize,
}

pub type ClientHandle = std::rc::Rc<std::cell::RefCell<Client>>;

impl From<Client> for ClientHandle {
    fn from(client: Client) -> Self {
        std::rc::Rc::new(std::cell::RefCell::new(client))
    }
}

impl Client {
    pub fn new(ringbuffer_size: usize) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            event_queue: std::cell::RefCell::new(std::collections::VecDeque::new()),
            ringbuffer_size,
        }
    }

    pub fn next_message(&self) -> Option<Message> {
        self.event_queue.borrow_mut().pop_front()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Topic {
    Command,
}

impl std::fmt::Display for Topic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command => write!(f, "command"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Command {
    ImportGltfFile(String),
    Exit,
}

#[derive(Clone, Debug)]
pub enum Message {
    Command(Command),
}
