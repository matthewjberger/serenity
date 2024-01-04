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

        event_loop.run(move |event, _, control_flow| {
            if gui.consumed_event(&event, &window) {
                return;
            }

            io.receive_event(
                &event,
                nalgebra_glm::vec2(
                    window.inner_size().width as f32 / 2.0,
                    window.inner_size().height as f32 / 2.0,
                ),
            );

            update_scene(&mut scene, &io);

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

fn update_scene(scene: &mut crate::scene::Scene, io: &crate::io::Io) {
    scene.walk_dfs_mut(|node, _| {
        update_cameras(node, io);
    });

    // MouseOrbit
    //   orientation.zoom(mouse.wheel_delta.y * 0.3);
    //   let mouse_delta = mouse_position_delta * delta_time as f32;
    //   if right_mouse_clicked && !Lshift
    //     mouse_delta.x = -1.0 * mouse_delta.x;
    //     orientation.rotate(&mouse_delta);
    //   if middle_mouse_clicked || (right_mouse_clicked && Lshift)
    // 		orientation.pan(&mouse_delta)
    // 	 transform.translation = self.orientation.position();
    // 			transform.rotation = self.orientation.look_at_offset();
    //   Ungrab cursor (cursor grab mode none)
    //   Hide cursor

    // MouseLook {
    //   let mouse_delta = offset_from_center * delta_time;
    //   orientation.rotate(&mouse_delta);
    //   transform.rotation = orientation.look_forward();
    //   Grab cursor (cursor grab mode confied)
    //   Hide cursor
    //   center cursor
}

fn update_cameras(node: &mut crate::scene::Node, io: &crate::io::Io) {
    let has_camera = node
        .components
        .iter()
        .any(|component| matches!(component, crate::scene::NodeComponent::Camera(_)));
    if has_camera {
        for component in node.components.iter_mut() {
            if let crate::scene::NodeComponent::Camera(camera) = component {
                camera.orientation.zoom(io.mouse.wheel_delta.y * 0.3);

                if io.mouse.is_right_clicked
                    && io.is_key_pressed(winit::event::VirtualKeyCode::LShift)
                {
                    let mut delta = io.mouse.position_delta;
                    delta.x *= -1.0;
                    camera.orientation.rotate(&delta);
                }

                if io.mouse.is_middle_clicked
                    && io.is_key_pressed(winit::event::VirtualKeyCode::LShift)
                {
                    camera.orientation.pan(&io.mouse.position_delta);
                }

                // add wasd movement
                if io.is_key_pressed(winit::event::VirtualKeyCode::W) {
                    node.transform.translation.x += 2.0;
                }

                node.transform.apply_orientation(&camera.orientation);

                break;
            }
        }
    }
}
