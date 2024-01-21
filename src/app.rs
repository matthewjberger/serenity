pub struct Context {
    pub window: winit::window::Window,
    pub io: crate::io::Io,
    pub delta_time: f64,
    pub last_frame: std::time::Instant,
    pub world: crate::world::World,
    pub should_exit: bool,
    pub should_sync_renderer: bool,
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
            world: crate::world::World::default(),
            should_exit: false,
            should_sync_renderer: false,
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

            let gui_consumed_event = {
                match &event {
                    winit::event::Event::WindowEvent { event, window_id } => {
                        if *window_id == context.window.id() {
                            renderer
                                .gui
                                .state
                                .on_event(&renderer.gui.context, event)
                                .consumed
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            };

            if !gui_consumed_event {
                context
                    .io
                    .receive_event(&event, renderer.gpu.window_center());
                state.receive_event(&mut context, &event);
            }

            if context.should_exit {
                *control_flow = winit::event_loop::ControlFlow::Exit;
            }

            if let winit::event::Event::MainEventsCleared = event {
                if context.should_sync_renderer {
                    renderer.assign_world(&context.world);
                    context.should_sync_renderer = false;
                }
                renderer.render_frame(&mut context, |context, ui| {
                    state.ui(context, ui);
                });
            }
        });
    }
}
