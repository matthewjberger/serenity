pub(crate) struct App {
    event_loop: winit::event_loop::EventLoop<()>,
    window: winit::window::Window,
    gpu: crate::gpu::Gpu,
    gui: crate::gui::Gui,
    io: crate::io::Io,
    view: crate::view::View,
    scene: crate::scene::Scene,
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

        let gpu = crate::gpu::Gpu::new(&window, width, height);
        let gui = crate::gui::Gui::new(&window, &gpu);
        let view = crate::view::View::new(&gpu);

        let mut scene = crate::scene::Scene::default();
        scene.add_root_node(crate::scene::create_camera_node(gpu.aspect_ratio()));

        Self {
            event_loop,
            window,
            gpu,
            gui,
            io: crate::io::Io::default(),
            scene,
            view,
        }
    }

    pub fn run(self) {
        let Self {
            event_loop,
            window,
            mut gui,
            mut view,
            mut gpu,
            mut io,
            mut scene,
        } = self;

        env_logger::init();

        event_loop.run(move |event, _, control_flow| {
            io.receive_event(
                &event,
                nalgebra_glm::vec2(
                    window.inner_size().width as f32 / 2.0,
                    window.inner_size().height as f32 / 2.0,
                ),
            );

            if gui.consumed_event(&event, &window) {
                return;
            }

            camera_system(&mut scene, &io);

            if let winit::event::Event::WindowEvent {
                event:
                    winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize { width, height }),
                ..
            } = event
            {
                gpu.resize(width, height);
                view.resize(&gpu, width, height);
            }

            if let winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested,
                ..
            } = event
            {
                *control_flow = winit::event_loop::ControlFlow::Exit
            }

            if let winit::event::Event::MainEventsCleared = event {
                view.render(&window, &gpu, &mut gui, &mut scene);
            }
        });
    }
}

fn camera_system(scene: &mut crate::scene::Scene, io: &crate::io::Io) {
    scene.walk_dfs_mut(|node, _| {
        node.components.iter_mut().for_each(|component| {
            if let crate::scene::NodeComponent::Camera(camera) = component {
                update_camera_orientation(camera, &io);
                node.transform.rotation = camera.orientation.look_at();
                node.transform.translation = camera.orientation.translation();
            }
        });
    });
}

fn update_camera_orientation(camera: &mut crate::scene::Camera, io: &crate::io::Io) {
    camera.orientation.zoom(io.mouse.wheel_delta.y * 0.03);

    if io.mouse.is_right_clicked && io.is_key_pressed(winit::event::VirtualKeyCode::LShift) {
        let mut delta = io.mouse.position_delta;
        delta.x *= -1.0;
        delta *= 0.03;
        camera.orientation.rotate(&delta);
    }

    if io.mouse.is_middle_clicked && io.is_key_pressed(winit::event::VirtualKeyCode::LShift) {
        camera.orientation.pan(&(io.mouse.position_delta * 0.03));
    }

    if io.is_key_pressed(winit::event::VirtualKeyCode::W) {
        camera.orientation.pan(&nalgebra_glm::Vec2::new(0.0, 0.5));
    }

    if io.is_key_pressed(winit::event::VirtualKeyCode::S) {
        camera.orientation.pan(&nalgebra_glm::Vec2::new(0.0, -0.5));
    }

    if io.is_key_pressed(winit::event::VirtualKeyCode::A) {
        camera.orientation.pan(&nalgebra_glm::Vec2::new(-0.5, 0.0));
    }

    if io.is_key_pressed(winit::event::VirtualKeyCode::D) {
        camera.orientation.pan(&nalgebra_glm::Vec2::new(0.5, 0.0));
    }
}
