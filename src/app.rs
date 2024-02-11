pub struct Context {
    pub io: crate::io::Io,
    pub delta_time: f64,
    pub last_frame: std::time::Instant,
    pub world: crate::world::World,
    pub should_exit: bool,
    pub should_reload_view: bool,
}

impl Context {
    pub fn import_file(&mut self, path: &str) {
        self.world = crate::gltf::import_gltf(path);

        if self.world.scenes.is_empty() {
            self.world.scenes.push(crate::world::Scene::default());
            self.world.default_scene_index = Some(0);
        }

        if let Some(scene_index) = self.world.default_scene_index {
            self.world.add_camera_to_scenegraph(scene_index);
        }

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

pub struct App<'window> {
    event_loop: winit::event_loop::EventLoop<()>,
    context: Context,
    renderer: crate::render::Renderer<'window>,
}

impl<'window> App<'window> {
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        env_logger::init();

        let event_loop =
            winit::event_loop::EventLoop::new().expect("Failed to create winit event loop!");
        let window = winit::window::WindowBuilder::new()
            .with_title(title)
            .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
            .with_transparent(true)
            .build(&event_loop)
            .expect("Failed to create winit window!");
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        let renderer = crate::render::Renderer::new(window, width, height);
        let context = Context {
            io: crate::io::Io::default(),
            delta_time: 0.01,
            last_frame: std::time::Instant::now(),
            world: crate::world::World::default(),
            should_exit: false,
            should_reload_view: false,
        };
        Self {
            event_loop,
            context,
            renderer,
        }
    }

    pub fn run(self, mut state: impl State + 'static) {
        let Self {
            event_loop,
            mut context,
            mut renderer,
        } = self;

        state.initialize(&mut context);

        event_loop.run(move |event, elwt| {
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
                elwt.exit();
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
                elwt.exit();
            }

            if let winit::event::Event::AboutToWait = event {
                if context.should_reload_view {
                    renderer.load_world(&context.world);
                    context.should_reload_view = false;
                } else {
                    renderer.render_frame(&mut context);
                }
            }
        }).expect("Failed to execute frame!");
    }
}
