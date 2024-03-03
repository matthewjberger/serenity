#[cfg(not(target_arch = "wasm32"))]
pub fn run(state: impl State + 'static) {
    env_logger::init();
    pollster::block_on(run_async(state));
}

#[cfg(target_arch = "wasm32")]
pub fn run(state: impl State + 'static) {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init().expect("could not initialize logger");
    wasm_bindgen_futures::spawn_local(run_async(state));
}

pub async fn run_async(mut state: impl State + 'static) {
    let event_loop =
        winit::event_loop::EventLoop::new().expect("Failed to create winit event loop!");

    #[allow(unused_mut)]
    let mut builder = winit::window::WindowBuilder::new();

    if !cfg!(target_arch = "wasm32") {
        builder = builder.with_title(state.title());
    }

    #[cfg(target_arch = "wasm32")]
    {
        use web_sys::wasm_bindgen::JsCast;
        use winit::platform::web::WindowBuilderExtWebSys;
        let canvas = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        builder = builder.with_canvas(Some(canvas));
    }

    let window = builder
        .build(&event_loop)
        .expect("Failed to create winit window!");
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let window = std::sync::Arc::new(window);

    let window_size = window.inner_size();
    let (width, height) = (window_size.width, window_size.height);
    let mut renderer = render::Renderer::new(window.clone(), width, height).await;

    let mut context = Context {
        io: Io::default(),
        delta_time: 0.01,
        last_frame: chrono::Utc::now(),
        world: asset::Asset::default(),
        should_exit: false,
        should_reload_view: false,
    };

    let gui_context = egui::Context::default();
    gui_context.set_pixels_per_point(window.scale_factor() as f32);
    let viewport_id = gui_context.viewport_id();
    let mut gui_state = egui_winit::State::new(
        gui_context,
        viewport_id,
        &window,
        Some(window.scale_factor() as _),
        None,
    );

    state.initialize(&mut context);

    event_loop
        .run(move |event, elwt| {
            if let winit::event::Event::NewEvents(..) = &event {
                let now = chrono::Utc::now();
                let duration_since_last_frame = now.signed_duration_since(context.last_frame);
                context.delta_time =
                    duration_since_last_frame.num_microseconds().unwrap() as f64 / 1_000_000.0;
                context.last_frame = now;
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

            if context.should_reload_view {
                renderer.load_world(&context.world);
                context.should_reload_view = false;
                return;
            }

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
                        render::ScreenDescriptor {
                            size_in_pixels: [window_size.width, window_size.height],
                            pixels_per_point: window.scale_factor() as f32,
                        }
                    };
                    renderer.render_frame(
                        &mut context.world,
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

pub struct Context {
    pub io: Io,
    pub delta_time: f64,
    pub last_frame: chrono::DateTime<chrono::Utc>,
    pub world: asset::Asset,
    pub should_exit: bool,
    pub should_reload_view: bool,
}

pub fn window_aspect_ratio(window: &winit::window::Window) -> f32 {
    let winit::dpi::PhysicalSize { width, height } = window.inner_size();
    width as f32 / height.max(1) as f32
}

pub trait State {
    fn title(&self) -> &str {
        "Phantom App"
    }

    /// Called once before the main loop
    fn initialize(&mut self, _context: &mut Context) {}

    /// Called when a winit event is received
    fn receive_event(&mut self, _context: &mut Context, _event: &winit::event::Event<()>) {}

    /// Called every frame prior to rendering
    fn update(&mut self, _context: &mut Context, _ui: &egui::Context) {}
}

#[derive(Default)]
pub struct Io {
    pub keystates: std::collections::HashMap<winit::keyboard::KeyCode, winit::event::ElementState>,
    pub mouse: Mouse,
}

impl Io {
    pub fn is_key_pressed(&self, keycode: winit::keyboard::KeyCode) -> bool {
        self.keystates.contains_key(&keycode)
            && self.keystates[&keycode] == winit::event::ElementState::Pressed
    }

    pub fn receive_event<T>(
        &mut self,
        event: &winit::event::Event<T>,
        window_center: nalgebra_glm::Vec2,
    ) {
        if let winit::event::Event::WindowEvent {
            event:
                winit::event::WindowEvent::KeyboardInput {
                    event:
                        winit::event::KeyEvent {
                            physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                            state,
                            ..
                        },
                    ..
                },
            ..
        } = *event
        {
            *self.keystates.entry(key_code).or_insert(state) = state;
        }
        self.mouse.receive_event(event, window_center);
    }
}

#[derive(Default)]
pub struct Mouse {
    pub is_left_clicked: bool,
    pub is_middle_clicked: bool,
    pub is_right_clicked: bool,
    pub position: nalgebra_glm::Vec2,
    pub position_delta: nalgebra_glm::Vec2,
    pub offset_from_center: nalgebra_glm::Vec2,
    pub wheel_delta: nalgebra_glm::Vec2,
    pub moved: bool,
    pub scrolled: bool,
}

impl Mouse {
    pub fn receive_event<T>(
        &mut self,
        event: &winit::event::Event<T>,
        window_center: nalgebra_glm::Vec2,
    ) {
        match event {
            winit::event::Event::NewEvents { .. } => self.new_events(),
            winit::event::Event::WindowEvent { event, .. } => match *event {
                winit::event::WindowEvent::MouseInput { button, state, .. } => {
                    self.mouse_input(button, state)
                }
                winit::event::WindowEvent::CursorMoved { position, .. } => {
                    self.cursor_moved(position, window_center)
                }
                winit::event::WindowEvent::MouseWheel {
                    delta: winit::event::MouseScrollDelta::LineDelta(h_lines, v_lines),
                    ..
                } => self.mouse_wheel(h_lines, v_lines),
                _ => {}
            },
            _ => {}
        }
    }

    fn new_events(&mut self) {
        if !self.scrolled {
            self.wheel_delta = nalgebra_glm::vec2(0.0, 0.0);
        }
        self.scrolled = false;

        if !self.moved {
            self.position_delta = nalgebra_glm::vec2(0.0, 0.0);
        }
        self.moved = false;
    }

    fn cursor_moved(
        &mut self,
        position: winit::dpi::PhysicalPosition<f64>,
        window_center: nalgebra_glm::Vec2,
    ) {
        let last_position = self.position;
        let current_position = nalgebra_glm::vec2(position.x as _, position.y as _);
        self.position = current_position;
        self.position_delta = current_position - last_position;
        self.offset_from_center =
            window_center - nalgebra_glm::vec2(position.x as _, position.y as _);
        self.moved = true;
    }

    fn mouse_wheel(&mut self, h_lines: f32, v_lines: f32) {
        self.wheel_delta = nalgebra_glm::vec2(h_lines, v_lines);
        self.scrolled = true;
    }

    fn mouse_input(
        &mut self,
        button: winit::event::MouseButton,
        state: winit::event::ElementState,
    ) {
        let clicked = state == winit::event::ElementState::Pressed;
        match button {
            winit::event::MouseButton::Left => self.is_left_clicked = clicked,
            winit::event::MouseButton::Middle => self.is_middle_clicked = clicked,
            winit::event::MouseButton::Right => self.is_right_clicked = clicked,
            _ => {}
        }
    }
}
