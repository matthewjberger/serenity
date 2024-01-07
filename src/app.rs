pub struct Context {
    pub window: winit::window::Window,
    pub io: crate::io::Io,
    pub delta_time: f64,
    pub last_frame: std::time::Instant,
    pub scene: crate::scene::Scene,
    pub should_exit: bool,
}

pub trait State {
    /// Called when a winit event is received
    fn receive_event(&mut self, _context: &mut Context, _event: &winit::event::Event<()>) {}

    /// Called every frame prior to rendering
    fn update(&mut self, _context: &mut Context, _renderer: &mut crate::render::Renderer) {}

    /// Called every frame after update()
    /// to create UI paint jobs for rendering
    fn ui(&mut self, _context: &mut Context, _ui: &mut egui::Context) {}
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
        let renderer = crate::render::Renderer::new(&window, width, height, window.scale_factor());
        let context = Context {
            window,
            io: crate::io::Io::default(),
            delta_time: 0.01,
            last_frame: std::time::Instant::now(),
            scene: crate::scene::Scene::default(),
            should_exit: false,
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

        event_loop.run(move |event, _, control_flow| {
            *control_flow = winit::event_loop::ControlFlow::Poll;

            if let winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested,
                ..
            } = event
            {
                *control_flow = winit::event_loop::ControlFlow::Exit
            }

            if let winit::event::Event::NewEvents(..) = event {
                context.delta_time = (std::time::Instant::now()
                    .duration_since(context.last_frame)
                    .as_micros() as f64)
                    / 1_000_000_f64;
                context.last_frame = std::time::Instant::now();
            }

            if let winit::event::Event::WindowEvent {
                event:
                    winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize { width, height }),
                ..
            } = event
            {
                renderer.resize(width, height);
            }

            if !renderer.gui.receive_event(&event, &context.window) {
                context
                    .io
                    .receive_event(&event, renderer.gpu.window_center());
            }

            state.receive_event(&mut context, &event);
            state.update(&mut context, &mut renderer);

            if context.should_exit {
                *control_flow = winit::event_loop::ControlFlow::Exit;
            }

            if let winit::event::Event::MainEventsCleared = event {
                renderer.render_frame(&mut context, |context, ui| {
                    state.ui(context, ui);
                });
            }
        });
    }
}
