pub struct Renderer {
    pub gpu: crate::gpu::Gpu,
    pub gui: crate::gui::Gui,
    pub view: crate::view::View,
    pub depth_texture_view: wgpu::TextureView,
}

impl Renderer {
    pub fn new<
        W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    >(
        window: &W,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> Self {
        let gpu = pollster::block_on(crate::gpu::Gpu::new_async(&window, width, height));
        let depth_texture_view =
            gpu.create_depth_texture(gpu.surface_config.width, gpu.surface_config.height);
        let gui = crate::gui::Gui::new(&window, &gpu, scale_factor);
        let view = crate::view::View::new(&gpu);
        Self {
            gpu,
            gui,
            view,
            depth_texture_view,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
        self.depth_texture_view = self.gpu.create_depth_texture(
            self.gpu.surface_config.width,
            self.gpu.surface_config.height,
        );
    }

    pub fn render_frame(
        &mut self,
        context: &mut crate::app::Context,
        ui_callback: impl FnOnce(&mut crate::app::Context, &mut egui::Context),
    ) {
        self.begin_frame(context);
        ui_callback(context, &mut self.gui.context);
        self.end_frame(context);
    }

    fn begin_frame(&mut self, context: &mut crate::app::Context) {
        self.gui.begin_frame(&context.window);
    }

    fn end_frame(&mut self, context: &mut crate::app::Context) {
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let (paint_jobs, screen_descriptor) =
            self.gui.end_frame(&self.gpu, &context.window, &mut encoder);

        let surface_texture = self
            .gpu
            .surface
            .get_current_texture()
            .expect("Failed to get surface texture!");

        let surface_texture_view =
            surface_texture
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
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            self.view
                .render(&mut render_pass, &self.gpu, &context.scene);
            self.gui
                .renderer
                .render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));

        surface_texture.present();
    }
}

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl From<crate::scene::Sampler> for wgpu::SamplerDescriptor<'static> {
    fn from(sampler: crate::scene::Sampler) -> Self {
        let min_filter = match sampler.min_filter {
            crate::scene::Filter::Linear => wgpu::FilterMode::Linear,
            crate::scene::Filter::Nearest => wgpu::FilterMode::Nearest,
        };

        let mipmap_filter = match sampler.min_filter {
            crate::scene::Filter::Linear => wgpu::FilterMode::Linear,
            crate::scene::Filter::Nearest => wgpu::FilterMode::Nearest,
        };

        let mag_filter = match sampler.mag_filter {
            crate::scene::Filter::Nearest => wgpu::FilterMode::Nearest,
            crate::scene::Filter::Linear => wgpu::FilterMode::Linear,
        };

        let address_mode_u = match sampler.wrap_s {
            crate::scene::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            crate::scene::WrappingMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
            crate::scene::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
        };

        let address_mode_v = match sampler.wrap_t {
            crate::scene::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            crate::scene::WrappingMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
            crate::scene::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
        };

        let address_mode_w = wgpu::AddressMode::Repeat;

        wgpu::SamplerDescriptor {
            address_mode_u,
            address_mode_v,
            address_mode_w,
            mag_filter,
            min_filter,
            mipmap_filter,
            ..Default::default()
        }
    }
}
