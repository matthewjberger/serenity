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
    fn update(&mut self, _context: &mut Context, _ui: &egui::Context) {}
}

pub struct App<'window> {
    event_loop: winit::event_loop::EventLoop<()>,
    context: Context,
    renderer: crate::render::Renderer<'window>,
    window: std::sync::Arc<winit::window::Window>,
    gui_state: egui_winit::State,
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

        let window = std::sync::Arc::new(window);

        let renderer = crate::render::Renderer::new(window.clone(), width, height);

        let context = Context {
            io: crate::io::Io::default(),
            delta_time: 0.01,
            last_frame: std::time::Instant::now(),
            world: crate::world::World::default(),
            should_exit: false,
            should_reload_view: false,
        };

        let gui_context = egui::Context::default();
        gui_context.set_pixels_per_point(window.scale_factor() as f32);
        let viewport_id = gui_context.viewport_id();
        let gui_state = egui_winit::State::new(
            gui_context,
            viewport_id,
            &window,
            Some(window.scale_factor() as _),
            None,
        );

        Self {
            event_loop,
            context,
            renderer,
            window,
            gui_state,
        }
    }

    pub fn run(self, mut state: impl State + 'static) {
        let Self {
            event_loop,
            mut context,
            mut renderer,
            window,
            mut gui_state,
        } = self;

        state.initialize(&mut context);

        event_loop
            .run(move |event, elwt| {
                if let winit::event::Event::NewEvents(..) = &event {
                    context.delta_time = (std::time::Instant::now()
                        .duration_since(context.last_frame)
                        .as_micros() as f64)
                        / 1_000_000_f64;
                    context.last_frame = std::time::Instant::now();
                }

                if let winit::event::Event::WindowEvent { ref event, .. } = &event {
                    if gui_state.on_window_event(&window, event).consumed {
                        return;
                    }
                }

                // TODO: Just give access to window events instead of the whole event, then move these into the match arm
                context
                    .io
                    .receive_event(&event, renderer.gpu.window_center());
                state.receive_event(&mut context, &event);

                match event {
                    winit::event::Event::WindowEvent { ref event, .. } => {
                        match event {
                            winit::event::WindowEvent::KeyboardInput {
                                event:
                                    winit::event::KeyEvent {
                                        physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                                        ..
                                    },
                                ..
                            } => {
                                // Exit by pressing the escape key
                                if matches!(key_code, winit::keyboard::KeyCode::Escape) {
                                    elwt.exit();
                                }
                            }

                            // Close button handler
                            winit::event::WindowEvent::CloseRequested => {
                                elwt.exit();
                            }

                            winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize {
                                width,
                                height,
                            }) => {
                                if *width > 0 && *height > 0 {
                                    renderer.resize(*width, *height);
                                }
                            }
                            _ => {}
                        }
                    }

                    winit::event::Event::AboutToWait => {
                        if window.inner_size().width == 0 || window.inner_size().height == 0 {
                            return;
                        }

                        if context.should_exit {
                            elwt.exit();
                        }

                        if context.should_reload_view {
                            renderer.load_world(&context.world);
                            context.should_reload_view = false;
                        }

                        let gui_input = gui_state.take_egui_input(&window);
                        gui_state.egui_ctx().begin_frame(gui_input);

                        state.update(&mut context, gui_state.egui_ctx());

                        let egui::FullOutput {
                            textures_delta,
                            shapes,
                            pixels_per_point,
                            ..
                        } = gui_state.egui_ctx().end_frame();

                        let paint_jobs = gui_state.egui_ctx().tessellate(shapes, pixels_per_point);

                        let screen_descriptor = {
                            let window_size = window.inner_size();
                            egui_wgpu::ScreenDescriptor {
                                size_in_pixels: [window_size.width, window_size.height],
                                pixels_per_point: window.scale_factor() as f32,
                            }
                        };
                        renderer.render_frame(
                            &mut context,
                            &textures_delta,
                            paint_jobs,
                            screen_descriptor,
                        );
                    }

                    _ => {}
                }
            })
            .expect("Failed to execute frame!");
    }
}
