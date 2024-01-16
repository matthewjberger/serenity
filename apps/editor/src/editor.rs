use serenity::{egui, nalgebra_glm, winit};

pub struct Editor {
    broker: Broker,
    client: ClientHandle,
    toasts: egui_toast::Toasts,
    gizmo_mode: egui_gizmo::GizmoMode,
}

impl Editor {
    pub fn new() -> Self {
        let mut broker = Broker::default();
        let client: ClientHandle = Client::new(10).into();
        broker.subscribe(&Topic::Command.to_string(), &client);
        broker.subscribe(&Topic::Toast.to_string(), &client);
        Self {
            broker,
            client,
            toasts: egui_toast::Toasts::new()
                .anchor(egui::Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                .direction(egui::Direction::BottomUp),
            gizmo_mode: egui_gizmo::GizmoMode::Translate,
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

    fn receive_messages(&mut self, context: &mut serenity::app::Context) {
        while let Some(message) = self.client.borrow().next_message() {
            match message {
                Message::Command(command) => match command {
                    Command::Exit => {
                        context.should_exit = true;
                    }
                    Command::ImportGltfFile(path) => {
                        context.world = serenity::gltf::import_gltf(&path).clone();
                    }
                },
                Message::Toast(message) => {
                    self.toasts.add(egui_toast::Toast {
                        text: message.into(),
                        kind: egui_toast::ToastKind::Info,
                        options: egui_toast::ToastOptions::default()
                            .duration_in_seconds(5.0)
                            .show_progress(true),
                    });
                }
            }
        }
    }
}

impl serenity::app::State for Editor {
    fn receive_event(
        &mut self,
        _context: &mut serenity::app::Context,
        event: &winit::event::Event<()>,
    ) {
        if let winit::event::Event::WindowEvent {
            event:
                winit::event::WindowEvent::KeyboardInput {
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
            if let (winit::event::VirtualKeyCode::Escape, winit::event::ElementState::Pressed) =
                (keycode, state)
            {
                self.publish_exit_command();
            }
        }
    }

    fn update(&mut self, context: &mut serenity::app::Context) {
        self.receive_messages(context);
        camera_system(context);
    }

    fn ui(&mut self, _context: &mut serenity::app::Context, ui_context: &mut egui::Context) {
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
                                ui.close_menu();
                            }
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Translate").clicked() {
                            self.gizmo_mode = egui_gizmo::GizmoMode::Translate;
                        }

                        if ui.button("Rotate").clicked() {
                            self.gizmo_mode = egui_gizmo::GizmoMode::Rotate;
                        }

                        if ui.button("Scale").clicked() {
                            self.gizmo_mode = egui_gizmo::GizmoMode::Scale;
                        }
                    });
                });
            });
    }
}

fn camera_system(context: &mut serenity::app::Context) {
    let scene = &context.world.scenes[context.world.active_scene_index];
    scene.walk_dfs(|node_index, _graph_node_index| {
        let transform =
            &mut context.world.transforms[context.world.nodes[node_index].transform_index];
        let camera = &mut context.world.cameras[scene.active_camera_index];

        let speed = 10.0 * context.delta_time as f32;

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
        transform.translation = camera.orientation.position();

        if context.io.is_key_pressed(winit::event::VirtualKeyCode::H) {
            transform.translation = nalgebra_glm::Vec3::new(1.0, 1.0, 1.0) * 4.0;
            camera.orientation.offset = nalgebra_glm::Vec3::new(0.0, 0.0, 0.0);
        }

        if context.io.mouse.is_right_clicked {
            let mut delta = context.io.mouse.position_delta * context.delta_time as f32;
            delta.x *= -1.0;
            delta.y *= -1.0;
            camera.orientation.rotate(&delta);
        }

        transform.rotation = camera.orientation.look_at_offset();
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Command {
    ImportGltfFile(String),
    Exit,
}

#[derive(Clone, Debug)]
pub enum Message {
    Command(Command),
    Toast(String),
}
