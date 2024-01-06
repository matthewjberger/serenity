pub struct Context {
    pub window: winit::window::Window,
    pub gpu: crate::gpu::Gpu,
    pub gui: crate::gui::Gui,
    pub io: crate::io::Io,
    pub view: crate::view::View,
    pub delta_time: f64,
    pub last_frame: std::time::Instant,
    pub scene: crate::scene::Scene,
    pub depth_texture_view: wgpu::TextureView,
    pub should_exit: bool,
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
        let depth_texture_view =
            gpu.create_depth_texture(gpu.surface_config.width, gpu.surface_config.height);

        let gui = crate::gui::Gui::new(&window, &gpu);
        let view = crate::view::View::new(&gpu);

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
                scene: crate::scene::Scene::default(),
                depth_texture_view,
                should_exit: false,
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

            receive_event(&event, &mut context, control_flow);

            state.receive_event(&mut context, &event);
            state.update(&mut context);

            if context.should_exit {
                *control_flow = winit::event_loop::ControlFlow::Exit;
            }

            if let winit::event::Event::MainEventsCleared = event {
                render_frame(&mut context, &mut state);
            }
        });
    }
}

fn render_frame(context: &mut Context, state: &mut impl State) {
    let mut encoder = context
        .gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
    let (paint_jobs, screen_descriptor) = {
        let encoder: &mut wgpu::CommandEncoder = &mut encoder;
        context.gui.begin_frame(&context.window);
        state.ui(context);
        let (paint_jobs, screen_descriptor) =
            context
                .gui
                .end_frame(&context.gpu, &context.window, encoder);
        (paint_jobs, screen_descriptor)
    };

    let surface_texture = context
        .gpu
        .surface
        .get_current_texture()
        .expect("Failed to get surface texture!");

    let surface_texture_view = surface_texture
        .texture
        .create_view(&wgpu::TextureViewDescriptor {
            label: wgpu::Label::default(),
            aspect: wgpu::TextureAspect::default(),
            format: None,
            dimension: None,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

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
                view: &context.depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        context
            .view
            .render(&mut render_pass, &context.gpu, &context.scene);
        context
            .gui
            .renderer
            .render(&mut render_pass, &paint_jobs, &screen_descriptor);
    }

    context.gpu.queue.submit(std::iter::once(encoder.finish()));

    surface_texture.present();
}

pub trait State {
    /// Called when a winit event is received
    fn receive_event(&mut self, context: &mut Context, event: &winit::event::Event<()>);

    /// Called every frame prior to rendering
    fn update(&mut self, context: &mut Context);

    /// Called every frame after update()
    /// to create UI paint jobs for rendering
    fn ui(&mut self, context: &mut Context);
}

fn receive_event(
    event: &winit::event::Event<()>,
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
        context.depth_texture_view = context.gpu.create_depth_texture(width, height);
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
