use serenity::{
    egui, nalgebra_glm, petgraph,
    winit::{self, dpi::PhysicalSize},
    world::NodeMetadata,
};

pub struct Editor {
    broker: Broker,
    client: ClientHandle,
    selected: Option<petgraph::graph::NodeIndex>,
    console_history: Vec<String>,
    console_command: String,
    toasts: egui_toast::Toasts,
    gizmo_mode: egui_gizmo::GizmoMode,
    command_history: std::collections::VecDeque<Command>,
    redo_stack: Vec<Command>,
    uniform_scaling: bool,
    physics_world_backup: Option<(
        serenity::physics::PhysicsWorld,
        Vec<serenity::world::Transform>,
    )>,
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
            selected: None,
            console_history: vec!["Welcome to the Serenity editor!".to_string()],
            console_command: "Type /help for more commands.".to_string(),
            toasts: egui_toast::Toasts::new()
                .anchor(egui::Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                .direction(egui::Direction::BottomUp),
            gizmo_mode: egui_gizmo::GizmoMode::Translate,
            command_history: std::collections::VecDeque::new(),
            redo_stack: Vec::new(),
            uniform_scaling: true,
            physics_world_backup: None,
        }
    }

    fn publish_undo_message(&mut self, command: Command) {
        self.broker
            .publish(&Topic::Command.to_string(), Message::Undo(command));
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

    fn publish_command(&mut self, command: Command) {
        self.broker
            .publish(&Topic::Command.to_string(), Message::Command(command));
    }

    fn publish_translate_command(&mut self, node_index: usize, x: f32, y: f32, z: f32) {
        self.broker.publish(
            &Topic::Command.to_string(),
            Message::Command(Command::Translate(node_index, x, y, z)),
        );
    }

    #[allow(dead_code)]
    fn publish_rotate_command(&mut self, node_index: usize, pitch: f32, yaw: f32, roll: f32) {
        self.broker.publish(
            &Topic::Command.to_string(),
            Message::Command(Command::Rotate(node_index, pitch, yaw, roll)),
        );
    }

    fn publish_scale_command(&mut self, node_index: usize, x: f32, y: f32, z: f32) {
        self.broker.publish(
            &Topic::Command.to_string(),
            Message::Command(Command::Scale(node_index, x, y, z)),
        );
    }

    fn receive_messages(&mut self, context: &mut serenity::app::Context) {
        while let Some(message) = self.client.borrow().next_message() {
            match message {
                Message::Command(command) => {
                    self.command_history.push_back(command.clone());
                    // arbitrary command history capacity
                    if self.command_history.len() == 10_000 {
                        self.command_history.pop_front(); // Remove the oldest element
                    }
                    match command {
                        Command::Exit => {
                            context.should_exit = true;
                        }
                        Command::ImportGltfFile(path) => {
                            context.world = serenity::gltf::import_gltf(&path);
                            context.should_reload_view = true;
                            self.selected = None;
                            self.redo_stack = Vec::new();
                            self.command_history = std::collections::VecDeque::new();

                            add_rigid_body_to_first_node(context);
                        }
                        Command::Translate(node_index, x, y, z) => {
                            translate_node(context, node_index, x, y, z);
                        }
                        Command::Rotate(node_index, pitch, yaw, roll) => {
                            rotate_node(context, node_index, pitch, yaw, roll);
                        }
                        Command::Scale(node_index, x, y, z) => {
                            scale_node(context, node_index, x, y, z);
                        }
                    }
                }

                Message::Undo(command) => match command {
                    Command::Translate(node_index, x, y, z) => {
                        translate_node(context, node_index, -x, -y, -z);
                    }
                    Command::Rotate(node_index, pitch, yaw, roll) => {
                        rotate_node(context, node_index, -pitch, -yaw, -roll);
                    }
                    Command::Scale(node_index, x, y, z) => {
                        scale_node(context, node_index, -x, -y, -z);
                    }
                    _ => {}
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

    fn inspector_transform_grid_ui(
        &mut self,
        ui: &mut egui::Ui,
        transform: &serenity::world::Transform,
        node_index: usize,
    ) {
        ui.label("Translation");
        ui.label("X");
        ui.label("Y");
        ui.label("Z");
        ui.end_row();

        let (mut translation_x, mut translation_y, mut translation_z) = (
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
        );
        ui.label("");
        ui.add(egui::DragValue::new(&mut translation_x).speed(0.1));
        ui.add(egui::DragValue::new(&mut translation_y).speed(0.1));
        ui.add(egui::DragValue::new(&mut translation_z).speed(0.1));
        ui.end_row();
        if translation_x != transform.translation.x
            || translation_y != transform.translation.y
            || translation_z != transform.translation.z
        {
            self.publish_translate_command(
                node_index,
                translation_x - transform.translation.x,
                translation_y - transform.translation.y,
                translation_z - transform.translation.z,
            );
        }

        ui.label("Scale");
        ui.label("X");
        ui.label("Y");
        ui.label("Z");
        ui.checkbox(&mut self.uniform_scaling, "Uniform");
        ui.end_row();
        let (mut scale_x, mut scale_y, mut scale_z) =
            (transform.scale.x, transform.scale.y, transform.scale.z);
        ui.label("");
        let mut uniform_scale = 0.0;
        if ui
            .add(egui::DragValue::new(&mut scale_x).speed(0.1))
            .changed()
        {
            uniform_scale = scale_x - transform.scale.x;
        }
        if ui
            .add(egui::DragValue::new(&mut scale_y).speed(0.1))
            .changed()
        {
            uniform_scale = scale_y - transform.scale.y;
        }
        if ui
            .add(egui::DragValue::new(&mut scale_z).speed(0.1))
            .changed()
        {
            uniform_scale = scale_z - transform.scale.z;
        }
        ui.end_row();

        if scale_x != transform.scale.x
            || scale_y != transform.scale.y
            || scale_z != transform.scale.z
        {
            if self.uniform_scaling {
                self.publish_scale_command(node_index, uniform_scale, uniform_scale, uniform_scale);
            } else {
                self.publish_scale_command(
                    node_index,
                    scale_x - transform.scale.x,
                    scale_y - transform.scale.y,
                    scale_z - transform.scale.z,
                );
            }
        }
    }

    fn backup_physics_world(&mut self, context: &mut serenity::app::Context) {
        self.physics_world_backup = Some((
            context.world.physics.clone(),
            context.world.transforms.clone(),
        ))
    }

    fn restore_physics_world(&mut self, context: &mut serenity::app::Context) {
        if let Some((physics_world, transforms)) = self.physics_world_backup.take() {
            context.world.physics = physics_world;
            context.world.transforms = transforms;
        }
    }
}

// TODO: remove this, it's for testing purposes
fn add_rigid_body_to_first_node(context: &mut serenity::app::Context) {
    if let Some(scene_index) = context.active_scene_index {
        let scene = &context.world.scenes[scene_index];
        if let Some(graph_node_index) = scene.graph.node_indices().next() {
            let node_index = scene.graph[graph_node_index];
            let node = &mut context.world.nodes[node_index];
            let rigid_body_index = context
                .world
                .physics
                .add_rigid_body(nalgebra_glm::Vec3::new(0.0, 0.0, 0.0));
            node.rigid_body_index = Some(rigid_body_index);
        }
    }
}

fn translate_node(context: &mut serenity::app::Context, node_index: usize, x: f32, y: f32, z: f32) {
    let transform_index = context.world.nodes[node_index].transform_index;
    let transform = &mut context.world.transforms[transform_index];
    transform.translation.x += x;
    transform.translation.y += y;
    transform.translation.z += z;
}

fn rotate_node(
    context: &mut serenity::app::Context,
    node_index: usize,
    pitch: f32,
    yaw: f32,
    roll: f32,
) {
    let transform_index = context.world.nodes[node_index].transform_index;
    let transform = &mut context.world.transforms[transform_index];
    let x_quat = nalgebra_glm::quat_angle_axis(pitch, &nalgebra_glm::Vec3::x_axis());
    let y_quat = nalgebra_glm::quat_angle_axis(yaw, &nalgebra_glm::Vec3::y_axis());
    let z_quat = nalgebra_glm::quat_angle_axis(roll, &nalgebra_glm::Vec3::z_axis());
    transform.rotation = x_quat * y_quat * z_quat * transform.rotation;
}

fn scale_node(context: &mut serenity::app::Context, node_index: usize, x: f32, y: f32, z: f32) {
    let transform_index = context.world.nodes[node_index].transform_index;
    let transform = &mut context.world.transforms[transform_index];
    transform.scale.x += x;
    transform.scale.y += y;
    transform.scale.z += z;
}

impl serenity::app::State for Editor {
    fn initialize(&mut self, context: &mut serenity::app::Context) {
        context.world = serenity::gltf::import_gltf("resources/models/Lantern.glb");
        context.active_scene_index = Some(0);
        context.should_reload_view = true;
        add_rigid_body_to_first_node(context);
    }

    fn receive_event(
        &mut self,
        context: &mut serenity::app::Context,
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

            if let (winit::event::VirtualKeyCode::F3, winit::event::ElementState::Pressed) =
                (keycode, state)
            {
                context.debug_visible = !context.debug_visible;
            }

            let left_ctrl_down = context
                .io
                .is_key_pressed(serenity::winit::event::VirtualKeyCode::LControl);

            if let (winit::event::VirtualKeyCode::R, winit::event::ElementState::Pressed, true) =
                (keycode, state, left_ctrl_down)
            {
                if let Some(command) = self.redo_stack.pop() {
                    self.publish_command(command);
                }
            }

            if let (winit::event::VirtualKeyCode::Z, winit::event::ElementState::Pressed, true) =
                (keycode, state, left_ctrl_down)
            {
                if let Some(command) = self.command_history.pop_back() {
                    self.publish_undo_message(command.clone());
                    self.redo_stack.push(command);
                }
            }

            if let (winit::event::VirtualKeyCode::H, winit::event::ElementState::Pressed, true) =
                (keycode, state, left_ctrl_down)
            {
                context.gui_visible = !context.gui_visible;
            }
        }
    }

    fn update(&mut self, context: &mut serenity::app::Context) {
        self.receive_messages(context);
        if let Some(active_scene_index) = context.active_scene_index {
            let scene = &context.world.scenes[active_scene_index];
            let mut ubo_offset = 0;
            scene.graph.node_indices().for_each(|graph_node_index| {
                let node_index = scene.graph[graph_node_index];
                let node = &context.world.nodes[node_index];
                if let Some(camera_index) = node.camera_index {
                    let transform = &mut context.world.transforms[node.transform_index];
                    let camera = &mut context.world.cameras[camera_index];
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
                    if context.io.mouse.is_right_clicked {
                        let mut delta = context.io.mouse.position_delta * context.delta_time as f32;
                        delta.x *= -1.0;
                        delta.y *= -1.0;
                        camera.orientation.rotate(&delta);
                    }
                    transform.rotation = camera.orientation.look_at_offset();
                }
                ubo_offset += 1;
            });
        }
    }

    fn ui(&mut self, context: &mut serenity::app::Context, ui_context: &mut egui::Context) {
        egui::Area::new("viewport").show(ui_context, |ui| {
            ui.with_layer_id(egui::LayerId::background(), |ui| {
                if let Some(selected) = self.selected {
                    if let Some(scene_index) = context.active_scene_index {
                        let scene = &context.world.scenes[scene_index];
                        let node_index = scene.graph[selected];
                        let node = &context.world.nodes[node_index];
                        let transform = &context.world.transforms[node.transform_index];
                        ui.group(|ui| {
                            let PhysicalSize { width, height } = context.window.inner_size();
                            let aspect_ratio = width as f32 / height.max(1) as f32;
                            let (_camera_position, projection, view) =
                                serenity::world::create_camera_matrices(
                                    &context.world,
                                    &scene,
                                    aspect_ratio,
                                )
                                .unwrap_or_default();
                            let model_matrix = transform.matrix();

                            let gizmo = egui_gizmo::Gizmo::new("My gizmo")
                                .view_matrix(view)
                                .projection_matrix(projection)
                                .model_matrix(model_matrix)
                                .mode(self.gizmo_mode);

                            if let Some(response) = gizmo.interact(ui) {
                                match self.gizmo_mode {
                                    egui_gizmo::GizmoMode::Translate => {
                                        self.publish_translate_command(
                                            node_index,
                                            response.translation.x - transform.translation.x,
                                            response.translation.y - transform.translation.y,
                                            response.translation.z - transform.translation.z,
                                        );
                                    }

                                    egui_gizmo::GizmoMode::Scale => {
                                        self.publish_scale_command(
                                            node_index,
                                            response.scale.x - transform.scale.x,
                                            response.scale.y - transform.scale.y,
                                            response.scale.z - transform.scale.z,
                                        );
                                    }

                                    _ => {}
                                }
                            }
                        });
                    }
                }
            });
        });

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
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui
                            .checkbox(&mut context.physics_enabled, "Enable Physics")
                            .clicked()
                        {
                            if context.physics_enabled {
                                self.backup_physics_world(context);
                            } else {
                                self.restore_physics_world(context);
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
                if let Some(scene_index) = context.active_scene_index {
                    let scene = &context.world.scenes[scene_index];
                    ui.group(|ui| {
                        egui::ScrollArea::vertical()
                            .id_source(ui.next_auto_id())
                            .show(ui, |ui| {
                                node_ui(
                                    &context.world,
                                    ui,
                                    &scene.graph,
                                    0.into(),
                                    &mut self.selected,
                                );
                            });
                    });
                }

                ui.allocate_space(ui.available_size());
            });

        egui::SidePanel::right("right_panel")
            .resizable(true)
            .show(ui_context, |ui| {
                ui.set_width(ui.available_width());
                ui.heading("Inspector");
                if let Some(selected_graph_node_index) = self.selected {
                    if let Some(scene_index) = context.active_scene_index {
                        let scene = &context.world.scenes[scene_index];
                        let node_index = scene.graph[selected_graph_node_index];
                        let node = &context.world.nodes[node_index];
                        egui::ScrollArea::vertical()
                            .id_source(ui.next_auto_id())
                            .show(ui, |ui| {
                                let transform_index = node.transform_index;
                                let transform = &context.world.transforms[transform_index];
                                ui.heading("Transform");
                                egui::Grid::new("node_transform_grid").striped(true).show(
                                    ui,
                                    |ui| {
                                        self.inspector_transform_grid_ui(ui, transform, node_index);
                                    },
                                );
                            });
                        ui.allocate_space(ui.available_size());
                    }
                }
            });

        egui::Window::new("Console")
            .collapsible(true)
            .default_open(false)
            .movable(true)
            .show(ui_context, |ui| {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let input = ui.text_edit_singleline(&mut self.console_command);
                        if input.lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter))
                            || ui.button("Run").clicked()
                        {
                            self.console_history
                                .push(format!(">> {}", self.console_command));
                            self.console_command.clear();
                            ui.memory_mut(|memory| memory.request_focus(input.id));
                        }
                    });
                    egui::ScrollArea::vertical()
                        .id_source(ui.next_auto_id())
                        .auto_shrink([false, true])
                        .stick_to_bottom(true)
                        .max_width(ui.available_width())
                        .max_height(ui.available_height() * 0.5)
                        .show(ui, |ui| {
                            self.console_history.iter().for_each(|line| {
                                ui.label(line);
                            });
                        });
                    ui.allocate_space(ui.available_size());
                });
            });

        self.toasts.show(ui_context);
    }
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
    Translate(usize, f32, f32, f32),
    Rotate(usize, f32, f32, f32),
    Scale(usize, f32, f32, f32),
    Exit,
}

#[derive(Clone, Debug)]
pub enum Message {
    Command(Command),
    Undo(Command),
    Toast(String),
}

fn node_ui(
    world: &serenity::world::World,
    ui: &mut egui::Ui,
    graph: &serenity::world::SceneGraph,
    graph_node_index: petgraph::graph::NodeIndex,
    selected_graph_node_index: &mut Option<petgraph::graph::NodeIndex>,
) {
    let id = ui.make_persistent_id(ui.next_auto_id());
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
        .show_header(ui, |ui| {
            let node_index = graph[graph_node_index];
            let NodeMetadata { name } = &world.metadata[node_index];
            let selected = selected_graph_node_index
                .as_ref()
                .map(|index| *index == graph_node_index)
                .unwrap_or_default();
            let response = ui.selectable_label(selected, format!("🔴 {name}"));
            if response.clicked() {
                *selected_graph_node_index = Some(graph_node_index);
            }
        })
        .body(|ui| {
            graph
                .neighbors_directed(graph_node_index, petgraph::Direction::Outgoing)
                .for_each(|child_index| {
                    node_ui(world, ui, graph, child_index, selected_graph_node_index);
                });
        });
}
