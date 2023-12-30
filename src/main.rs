fn main() {
    let (title, width, height, default_gltf_path) = (
        "Standalone Winit/Wgpu Example",
        800,
        600,
        "./assets/DamagedHelmet.glb",
    );

    let gltf_bytes = std::fs::read(&default_gltf_path).expect("Failed to load default gltf file!");
    println!("Loaded gltf ({} bytes)", gltf_bytes.len());
    let mut gltf = gltf::Gltf::from_slice(&gltf_bytes).expect("Failed to load GLTF!");
    // TODO: do something with this loaded gltf

    let event_loop = winit::event_loop::EventLoop::new();

    let window = winit::window::WindowBuilder::new()
        .with_title(title)
        .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
        .with_transparent(true)
        .build(&event_loop)
        .expect("Failed to create winit window!");

    let (surface, device, queue, mut surface_config) = pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all),
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to request adapter!");

        fn required_limits(adapter: &wgpu::Adapter) -> wgpu::Limits {
            // Use the texture resolution limits from the adapter
            // to support images the size of the surface
            wgpu::Limits::default().using_resolution(adapter.limits())
        }

        let required_features = wgpu::Features::empty();
        let optional_features = wgpu::Features::all();
        let (device, queue) = {
            println!("WGPU Adapter Features: {:#?}", adapter.features());
            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        features: (optional_features & adapter.features()) | required_features,
                        limits: required_limits(&adapter),
                        label: Some("Render Device"),
                    },
                    None,
                )
                .await
                .expect("Failed to request a device!")
        };

        let surface_capabilities = surface.get_capabilities(&adapter);

        // This assumes an sRGB surface texture
        let format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_capabilities.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        (surface, device, queue, surface_config)
    });

    let mut gui_state = egui_winit::State::new(&event_loop);
    let gui_context = egui::Context::default();
    gui_context.set_pixels_per_point(window.scale_factor() as f32);

    let depth_format = None;
    let mut gui_renderer =
        egui_wgpu::Renderer::new(&device, surface_config.format, depth_format, 1);

    event_loop.run(move |event, _, control_flow| {
        let gui_captured_event = match &event {
            winit::event::Event::WindowEvent { event, window_id } => {
                if *window_id == window.id() {
                    gui_state.on_event(&gui_context, &event).consumed
                } else {
                    false
                }
            }
            _ => false,
        };

        if gui_captured_event {
            return;
        }

        match event {
            winit::event::Event::MainEventsCleared => {
                let gui_input = gui_state.take_egui_input(&window);

                gui_context.begin_frame(gui_input);

                egui::TopBottomPanel::top("top_panel")
                    .resizable(true)
                    .show(&gui_context, |ui| {
                        egui::menu::bar(ui, |ui| {
                            egui::global_dark_light_mode_switch(ui);
                            ui.menu_button("File", |ui| {
                                if ui.button("Import asset (gltf/glb)...").clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("GLTF / GLB", &["gltf", "glb"])
                                        .pick_file()
                                    {
                                        println!("File picked: {path:#?}");
                                        match std::fs::read(&path) {
                                            Ok(bytes) => {
                                                println!("Loaded gltf ({} bytes)", bytes.len());
                                                gltf = gltf::Gltf::from_slice(&bytes)
                                                    .expect("Failed to load GLTF!");
                                            }
                                            Err(error) => {
                                                eprintln!("{error}");
                                            }
                                        };
                                    }
                                    ui.close_menu();
                                }
                            });
                        });
                    });

                egui::SidePanel::left("left_panel")
                    .resizable(true)
                    .show(&gui_context, |ui| {
                        ui.collapsing("Scenes", |ui| {
                            gltf.scenes().for_each(|gltf_scene| {
                                draw_scene_tree_ui(ui, gltf_scene);
                            });
                        });

                        ui.separator();

                        ui.collapsing("Meshes", |ui| {
                            gltf.meshes().for_each(|gltf_mesh| {
                                draw_mesh_ui(ui, gltf_mesh);
                            });
                        });
                    });

                egui::SidePanel::right("right_panel")
                    .resizable(true)
                    .show(&gui_context, |ui| {
                        ui.heading("Inspector");
                    });

                let egui::FullOutput {
                    textures_delta,
                    shapes,
                    ..
                } = gui_context.end_frame();

                let paint_jobs = gui_context.tessellate(shapes);

                let screen_descriptor = {
                    let window_size = window.inner_size();
                    egui_wgpu::renderer::ScreenDescriptor {
                        size_in_pixels: [window_size.width, window_size.height],
                        pixels_per_point: window.scale_factor() as f32,
                    }
                };

                // TODO: Update the game here

                let surface_texture = surface
                    .get_current_texture()
                    .expect("Failed to get surface texture!");

                let view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

                for (id, image_delta) in &textures_delta.set {
                    gui_renderer.update_texture(&device, &queue, *id, image_delta);
                }

                for id in &textures_delta.free {
                    gui_renderer.free_texture(id);
                }

                gui_renderer.update_buffers(
                    &device,
                    &queue,
                    &mut encoder,
                    &paint_jobs,
                    &screen_descriptor,
                );

                encoder.insert_debug_marker("Render scene");

                // This scope around the render_pass prevents the
                // render_pass from holding a borrow to the encoder,
                // which would prevent calling `.finish()` in
                // preparation for queue submission.
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.1,
                                    g: 0.2,
                                    b: 0.3,
                                    a: 1.0,
                                }),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    gui_renderer.render(&mut render_pass, &paint_jobs, &screen_descriptor);
                }

                queue.submit(std::iter::once(encoder.finish()));

                surface_texture.present();
            }

            winit::event::Event::WindowEvent { event, window_id } if window_id == window.id() => {
                match event {
                    winit::event::WindowEvent::CloseRequested => {
                        *control_flow = winit::event_loop::ControlFlow::Exit
                    }

                    winit::event::WindowEvent::KeyboardInput { input, .. } => {
                        if let (
                            Some(winit::event::VirtualKeyCode::Escape),
                            winit::event::ElementState::Pressed,
                        ) = (input.virtual_keycode, input.state)
                        {
                            *control_flow = winit::event_loop::ControlFlow::Exit;
                        }

                        if let Some(_keycode) = input.virtual_keycode.as_ref() {
                            // Handle a key press
                        }
                    }

                    winit::event::WindowEvent::MouseInput {
                        button: _button,
                        state: _state,
                        ..
                    } => {
                        // Handle a mouse button press
                    }

                    winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize {
                        width,
                        height,
                    }) => {
                        if width != 0 && height != 0 {
                            println!("Resizing renderer surface to: ({width}, {height})");
                            surface_config.width = width;
                            surface_config.height = height;
                            surface.configure(&device, &surface_config);
                        }
                    }
                    _ => {}
                }
            }
            winit::event::Event::LoopDestroyed => {
                // Handle cleanup
            }
            _ => {}
        }
    });
}

fn draw_scene_tree_ui<'a>(ui: &mut egui::Ui, scene: gltf::Scene<'a>) {
    let name = scene.name().unwrap_or("Unnamed Scene");
    let id = ui.make_persistent_id(ui.next_auto_id());
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
        .show_header(ui, |ui| {
            let response = ui.selectable_label(false, format!("🎬 {name}"));
            if response.clicked() {
                println!("Scene selected: {name}");
            }
        })
        .body(|ui| {
            draw_scene_ui(ui, scene);
        });
}

fn draw_scene_ui(ui: &mut egui::Ui, gltf_scene: gltf::Scene<'_>) {
    gltf_scene.nodes().for_each(|node| {
        draw_gltf_node_ui(ui, node);
    });
}

fn draw_gltf_node_ui(ui: &mut egui::Ui, node: gltf::Node<'_>) {
    let name = node.name().unwrap_or("Unnamed Node");

    let is_leaf = node.children().len() == 0;
    if is_leaf {
        node_ui(ui, &name, true);
    }

    node.children().for_each(|child| {
        let id = ui.make_persistent_id(ui.next_auto_id());
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                node_ui(ui, &name, false);
            })
            .body(|ui| {
                draw_gltf_node_ui(ui, child);
            });
    });
}

fn node_ui(ui: &mut egui::Ui, name: &str, is_leaf: bool) {
    let prefix = if is_leaf { "\t⭕" } else { "🔴" };
    let response = ui.selectable_label(false, format!("{prefix} {name}"));
    if response.clicked() {
        println!("Node selected: {name}");
    }
    response.context_menu(|ui| {
        ui.label("Shown on right-clicks");
    });
}

fn draw_mesh_ui<'a>(ui: &mut egui::Ui, mesh: gltf::Mesh<'a>) {
    let name = mesh.name().unwrap_or("Unnamed Mesh");
    let response = ui.selectable_label(false, format!("🔶{name}"));
    if response.clicked() {
        println!("Mesh selected: {name}");
    }
}
