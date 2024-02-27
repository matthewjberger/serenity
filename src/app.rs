#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

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

    #[cfg(not(target_arch = "wasm32"))]
    {
        builder = builder
            .with_title("Serenity")
            .with_inner_size(winit::dpi::PhysicalSize::new(1920, 1080));
    }

    #[cfg(target_arch = "wasm32")]
    {
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

    let mut renderer = crate::render::Renderer::new(window, 1920, 1080).await;

    let mut context = Context {
        io: crate::io::Io::default(),
        world: crate::world::World::default(),
        should_exit: false,
        should_reload_view: false,
    };

    state.initialize(&mut context);

    event_loop
        .run(move |event, elwt| {
            if let winit::event::Event::NewEvents(..) = event {
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
        })
        .expect("Failed to execute frame!");
}

pub struct Context {
    pub io: crate::io::Io,
    pub world: crate::world::World,
    pub should_exit: bool,
    pub should_reload_view: bool,
}

impl Context {
    pub fn import_gltf_slice(&mut self, bytes: &[u8]) {
        self.world = crate::gltf::import_gltf_slice(bytes);

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

pub trait State {
    /// Called once before the main loop
    fn initialize(&mut self, _context: &mut Context) {}

    /// Called when a winit event is received
    fn receive_event(&mut self, _context: &mut Context, _event: &winit::event::Event<()>) {}

    /// Called every frame prior to rendering
    fn update(&mut self, _context: &mut Context) {}
}
