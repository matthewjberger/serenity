pub struct Context {
    pub window: winit::window::Window,
    pub io: crate::io::Io,
    pub delta_time: f64,
    pub last_frame: std::time::Instant,
    pub world: crate::world::World,
    pub should_exit: bool,
    pub should_reload_view: bool,
    pub debug_visible: bool,
    pub active_scene_index: usize,
}

impl Context {
    pub fn import_file(&mut self, path: &str) {
        self.world = crate::gltf::import_gltf(path);

        // Add a default scene if none exist
        if self.world.scenes.is_empty() {
            self.world.scenes.push(crate::world::Scene::default());
        }

        // Assign the default active scene
        let scene_index = 0;
        self.active_scene_index = scene_index;

        // Add a default camera
        let node_index = self.world.add_node();
        self.world.add_camera_to_node(node_index);
        let camera_graph_node_index =
            self.world
                .add_child_node(scene_index, petgraph::graph::NodeIndex::new(0), node_index);
        self.world.scenes[scene_index].default_camera_graph_node_index =
            Some(camera_graph_node_index);
        let node = &self.world.nodes[node_index];
        let metadata = &mut self.world.metadata[node.metadata_index];
        metadata.name = "Main Camera".to_string();

        // Add bounding boxes to all nodes
        self.world.scenes[scene_index]
            .graph
            .node_indices()
            .for_each(|graph_node_index| {
                let node_index = self.world.scenes[scene_index].graph[graph_node_index];
                let node = &self.world.nodes[node_index];
                if node.mesh_index.is_none() && node.primitive_mesh_index.is_none() {
                    return;
                }
                self.world.add_bounding_box(scene_index, graph_node_index);
            });

        self.should_reload_view = true;
    }
}

pub fn window_aspect_ratio(window: &winit::window::Window) -> f32 {
    let winit::dpi::PhysicalSize { width, height } = window.inner_size();
    width as f32 / height.max(1) as f32
}

pub trait State {
    /// Called once before the main loop
    fn initialize(&mut self, _context: &mut Context) {}

    /// Called when a winit event is received
    fn receive_event(&mut self, _context: &mut Context, _event: &winit::event::Event<()>) {}

    /// Called every frame prior to rendering
    fn update(&mut self, _context: &mut Context) {}
}

pub struct App {
    event_loop: winit::event_loop::EventLoop<()>,
    context: Context,
    renderer: crate::render::Renderer,
}

impl App {
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        let event_loop = winit::event_loop::EventLoop::new();
        let window = winit::window::WindowBuilder::new()
            .with_title(title)
            .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
            .with_transparent(true)
            .build(&event_loop)
            .expect("Failed to create winit window!");
        let renderer = crate::render::Renderer::new(&window, width, height);
        let context = Context {
            window,
            io: crate::io::Io::default(),
            delta_time: 0.01,
            last_frame: std::time::Instant::now(),
            world: crate::world::World::default(),
            should_exit: false,
            should_reload_view: false,
            debug_visible: false,
            active_scene_index: 0,
        };

        Self {
            event_loop,
            context,
            renderer,
        }
    }

    pub fn run(self, mut state: impl State + 'static) {
        env_logger::init();

        let Self {
            event_loop,
            mut context,
            mut renderer,
        } = self;

        state.initialize(&mut context);

        event_loop.run(move |event, _, control_flow| {
            *control_flow = winit::event_loop::ControlFlow::Poll;

            if let winit::event::Event::NewEvents(..) = event {
                context.delta_time = (std::time::Instant::now()
                    .duration_since(context.last_frame)
                    .as_micros() as f64)
                    / 1_000_000_f64;
                context.last_frame = std::time::Instant::now();

                state.update(&mut context);
            }

            if let winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested,
                ..
            } = event
            {
                *control_flow = winit::event_loop::ControlFlow::Exit
            }

            if let winit::event::Event::WindowEvent {
                event:
                    winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize { width, height }),
                ..
            } = event
            {
                renderer.resize(width, height);
            }

            context
                .io
                .receive_event(&event, renderer.gpu.window_center());
            state.receive_event(&mut context, &event);

            if context.should_exit {
                *control_flow = winit::event_loop::ControlFlow::Exit;
            }

            if let winit::event::Event::MainEventsCleared = event {
                if context.should_reload_view {
                    renderer.sync_world(&context.world);
                    renderer.sync_debug(&context);
                    context.should_reload_view = false;
                    return;
                }

                context.world.physics.step(context.delta_time as _);

                let scene_index = context.active_scene_index;
                let scene = &mut context.world.scenes[scene_index];

                scene.graph.node_indices().for_each(|graph_node_index| {
                    let node_index = scene.graph[graph_node_index];
                    if let Some(rigid_body_index) = context.world.nodes[node_index].rigid_body_index
                    {
                        let transform_index = context.world.nodes[node_index].transform_index;
                        let transform = &mut context.world.transforms[transform_index];
                        let rigid_body = &context.world.physics.bodies[rigid_body_index];
                        transform.translation =
                            context.world.physics.positions[rigid_body.position_index];
                    }
                });

                renderer.render_frame(&mut context);
            }
        });
    }
}
