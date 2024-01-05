pub struct Context {
    pub window: winit::window::Window,
    pub gpu: crate::gpu::Gpu,
    pub gui: crate::gui::Gui,
    pub io: crate::io::Io,
    pub view: crate::view::View,
    pub delta_time: f64,
    pub last_frame: std::time::Instant,
    pub scene: crate::scene::Scene,
}

pub struct App {
    event_loop: winit::event_loop::EventLoop<()>,
    context: Context,
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
        scene
            .graph
            .add_node(crate::scene::create_camera_node(gpu.aspect_ratio()));

        Self {
            event_loop,
            context: Context {
                window,
                gpu,
                gui,
                io: crate::io::Io::default(),
                view,
                delta_time: 0.01,
                last_frame: std::time::Instant::now(),
                scene,
            },
        }
    }

    pub fn run(self, mut state: impl State + 'static) {
        env_logger::init();

        let Self {
            event_loop,
            mut context,
        } = self;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = winit::event_loop::ControlFlow::Poll;

            receive_events(&event, &mut context, control_flow);

            state.receive_events(&mut context, &event, control_flow);
            state.update(&mut context);

            if let winit::event::Event::MainEventsCleared = event {
                let mut encoder =
                    context
                        .gpu
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        });
                let (paint_jobs, screen_descriptor) =
                    render_ui(&mut context, &mut state, &mut encoder);
                render(encoder, &mut context, paint_jobs, screen_descriptor);
            }
        });
    }
}

fn render_ui(
    context: &mut Context,
    state: &mut impl State,
    encoder: &mut wgpu::CommandEncoder,
) -> (
    Vec<egui::ClippedPrimitive>,
    egui_wgpu::renderer::ScreenDescriptor,
) {
    context.gui.begin_frame(&context.window);
    state.ui(context);
    let (paint_jobs, screen_descriptor) =
        context
            .gui
            .end_frame(&context.gpu, &context.window, encoder);
    (paint_jobs, screen_descriptor)
}

fn render(
    mut encoder: wgpu::CommandEncoder,
    context: &mut Context,
    paint_jobs: Vec<egui::ClippedPrimitive>,
    screen_descriptor: egui_wgpu::renderer::ScreenDescriptor,
) {
    let surface_texture = context
        .gpu
        .surface
        .get_current_texture()
        .expect("Failed to get surface texture!");

    let surface_texture_view = surface_texture
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    encoder.insert_debug_marker("Render scene");

    // This scope around the render_pass prevents the
    // render_pass from holding a borrow to the encoder,
    // which would prevent calling `.finish()` in
    // preparation for queue submission.
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.4,
                        g: 0.2,
                        b: 0.2,
                        a: 1.0,
                    }),
                    store: true,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &context.view.depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        render_pass.set_pipeline(&context.view.pipeline);
        render_pass.set_bind_group(0, &context.view.uniform_bind_group, &[]);

        render_pass.set_vertex_buffer(0, context.view.vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            context.view.index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );

        let (projection_matrix, view_matrix) =
            crate::view::create_camera_matrices(&context.scene, context.gpu.aspect_ratio())
                .expect("No camera is available!");

        context.scene.walk_dfs(|node| {
            let model_matrix = node.transform.matrix();

            for component in node.components.iter() {
                if let crate::scene::NodeComponent::Mesh(mesh) = component {
                    let uniform_buffer = crate::view::UniformBuffer {
                        mvp: projection_matrix * view_matrix * model_matrix,
                    };

                    context.gpu.queue.write_buffer(
                        &context.view.uniform_buffer,
                        0,
                        bytemuck::cast_slice(&[uniform_buffer]),
                    );

                    render_pass.set_bind_group(0, &context.view.uniform_bind_group, &[]);

                    if let Some(commands) = context.view.meshes.get(&mesh.id) {
                        commands.iter().for_each(|command| {
                            let index_offset = command.index_offset as u32;
                            let number_of_indices = index_offset + command.indices as u32;
                            render_pass.draw_indexed(
                                index_offset..number_of_indices,
                                command.vertex_offset as i32,
                                0..1, // TODO: support multiple instances per primitive
                            );
                        });
                    }
                }
            }
        });

        context
            .gui
            .renderer
            .render(&mut render_pass, &paint_jobs, &screen_descriptor);
    }

    context.gpu.queue.submit(std::iter::once(encoder.finish()));

    surface_texture.present();
}

pub trait State {
    fn receive_events(
        &mut self,
        context: &mut Context,
        event: &winit::event::Event<'_, ()>,
        control_flow: &mut winit::event_loop::ControlFlow,
    );
    fn update(&mut self, context: &mut Context);
    fn ui(&mut self, context: &mut Context);
}

fn receive_events(
    event: &winit::event::Event<'_, ()>,
    context: &mut Context,
    control_flow: &mut winit::event_loop::ControlFlow,
) {
    if let winit::event::Event::NewEvents(..) = *event {
        context.delta_time = (std::time::Instant::now()
            .duration_since(context.last_frame)
            .as_micros() as f64)
            / 1_000_000_f64;
        context.last_frame = std::time::Instant::now();
    }

    if let winit::event::Event::WindowEvent {
        event: winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize { width, height }),
        ..
    } = *event
    {
        context.gpu.resize(width, height);
        context.view.resize(&context.gpu, width, height);
    }

    if let winit::event::Event::WindowEvent {
        event: winit::event::WindowEvent::CloseRequested,
        ..
    } = *event
    {
        *control_flow = winit::event_loop::ControlFlow::Exit
    }

    if !context.gui.receive_event(event, &context.window) {
        context.io.receive_event(event, context.gpu.window_center());
    }
}
