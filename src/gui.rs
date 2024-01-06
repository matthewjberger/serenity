pub struct Gui {
    pub renderer: egui_wgpu::Renderer,
    pub context: egui::Context,
    pub state: egui_winit::State,
}

impl Gui {
    pub fn new<W>(window: &W, gpu: &crate::gpu::Gpu, scale_factor: f64) -> Self
    where
        W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    {
        let state = egui_winit::State::new(window);
        let context = egui::Context::default();
        context.set_pixels_per_point(scale_factor as _);

        // This is required for egui to load and display images in the UI
        egui_extras::install_image_loaders(&context);

        Self {
            state,
            context,
            renderer: egui_wgpu::Renderer::new(
                &gpu.device,
                gpu.surface_config.format,
                Some(crate::gpu::Gpu::DEPTH_FORMAT),
                1,
            ),
        }
    }

    pub fn receive_event(
        &mut self,
        event: &winit::event::Event<()>,
        window: &winit::window::Window,
    ) -> bool {
        match event {
            winit::event::Event::WindowEvent { event, window_id } => {
                if *window_id == window.id() {
                    self.state.on_event(&self.context, event).consumed
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn end_frame(
        &mut self,
        gpu: &crate::gpu::Gpu,
        window: &winit::window::Window,
        encoder: &mut wgpu::CommandEncoder,
    ) -> (
        Vec<egui::ClippedPrimitive>,
        egui_wgpu::renderer::ScreenDescriptor,
    ) {
        let egui::FullOutput {
            textures_delta,
            shapes,
            ..
        } = self.context.end_frame();
        for (id, image_delta) in &textures_delta.set {
            self.renderer
                .update_texture(&gpu.device, &gpu.queue, *id, image_delta);
        }
        for id in &textures_delta.free {
            self.renderer.free_texture(id);
        }
        let paint_jobs = self.context.tessellate(shapes);
        let window_size = window.inner_size();
        let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
            size_in_pixels: [window_size.width.max(1), window_size.height.max(1)],
            pixels_per_point: window.scale_factor() as f32,
        };
        self.renderer.update_buffers(
            &gpu.device,
            &gpu.queue,
            encoder,
            &paint_jobs,
            &screen_descriptor,
        );
        (paint_jobs, screen_descriptor)
    }

    pub fn begin_frame(&mut self, window: &winit::window::Window) {
        self.context.begin_frame(self.state.take_egui_input(window))
    }
}
