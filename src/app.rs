pub(crate) struct App {
    event_loop: winit::event_loop::EventLoop<()>,
    window: winit::window::Window,
    gpu: crate::gpu::Gpu,
    gui: crate::gui::Gui,
    io: crate::io::Io,
    view: crate::view::View,
    scene: crate::scene::Scene,
    delta_time: f64,
    last_frame: std::time::Instant,
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
            delta_time: 0.01,
            last_frame: std::time::Instant::now(),
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
            mut delta_time,
            mut last_frame,
        } = self;

        env_logger::init();

        event_loop.run(move |event, _, control_flow| {
            *control_flow = winit::event_loop::ControlFlow::Poll;

            if let winit::event::Event::NewEvents(..) = event {
                delta_time = (std::time::Instant::now()
                    .duration_since(last_frame)
                    .as_micros() as f64)
                    / 1_000_000_f64;
                last_frame = std::time::Instant::now();
            }

            if io.is_key_pressed(winit::event::VirtualKeyCode::Escape) {
                *control_flow = winit::event_loop::ControlFlow::Exit;
            }

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

            if !gui.receive_event(&event, &window) {
                io.receive_event(&event, gpu.window_center());
            }

            camera_system(&mut scene, &io, delta_time);

            if let winit::event::Event::MainEventsCleared = event {
                view.render(
                    &window,
                    &gpu,
                    &mut gui,
                    &mut scene,
                    |gpu, gui, scene, view| ui(gpu, gui, scene, view),
                );
            }
        });
    }
}

fn camera_system(scene: &mut crate::scene::Scene, io: &crate::io::Io, delta_time: f64) {
    scene.walk_dfs_mut(|node, _| {
        node.components.iter_mut().for_each(|component| {
            if let crate::scene::NodeComponent::Camera(_camera) = component {
                if io.is_key_pressed(winit::event::VirtualKeyCode::W) {
                    node.transform.translation.z -= (0.05_f64 * delta_time) as f32;
                }
                if io.is_key_pressed(winit::event::VirtualKeyCode::A) {
                    node.transform.translation.x -= (0.05_f64 * delta_time) as f32;
                }
                if io.is_key_pressed(winit::event::VirtualKeyCode::S) {
                    node.transform.translation.z += (0.05_f64 * delta_time) as f32;
                }
                if io.is_key_pressed(winit::event::VirtualKeyCode::D) {
                    node.transform.translation.x += (0.05_f64 * delta_time) as f32;
                }
                if io.is_key_pressed(winit::event::VirtualKeyCode::Space) {
                    node.transform.translation.y += (0.05_f64 * delta_time) as f32;
                }
                if io.is_key_pressed(winit::event::VirtualKeyCode::LShift) {
                    node.transform.translation.y -= (0.05_f64 * delta_time) as f32;
                }
            }
        });
    });
}

fn ui(
    gpu: &crate::gpu::Gpu,
    gui: &mut crate::gui::Gui,
    scene: &mut crate::scene::Scene,
    view: &mut crate::view::View,
) {
    egui::TopBottomPanel::top("top_panel")
        .resizable(true)
        .show(&gui.context, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::global_dark_light_mode_switch(ui);
                ui.menu_button("File", |ui| {
                    if ui.button("Import asset (gltf/glb)...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("GLTF / GLB", &["gltf", "glb"])
                            .pick_file()
                        {
                            let scenes =
                                crate::gltf::import_gltf(path).expect("Failed to import gltf!");
                            *scene = scenes[0].clone();
                            if !scene.has_camera() {
                                scene.add_root_node(crate::scene::create_camera_node(
                                    gpu.aspect_ratio(),
                                ));
                            }
                            view.import_scene(&scenes[0], gpu);
                        }
                    };
                });
            });
        });

    egui::SidePanel::left("left_panel")
        .resizable(true)
        .show(&gui.context, |ui| {
            ui.heading("Scene Explorer");
        });

    egui::SidePanel::right("right_panel")
        .resizable(true)
        .show(&gui.context, |ui| {
            ui.heading("Inspector");
        });

    egui::TopBottomPanel::bottom("bottom_panel")
        .resizable(true)
        .show(&gui.context, |ui| {
            ui.heading("Console");
        });
}
