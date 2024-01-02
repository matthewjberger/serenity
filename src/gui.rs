use crate::gpu::Gpu;

pub struct Gui {
    pub renderer: egui_wgpu::Renderer,
    pub context: egui::Context,
    pub state: egui_winit::State,
}

impl Gui {
    pub fn new(window: &winit::window::Window, gpu: &crate::gpu::Gpu) -> Self {
        let state = egui_winit::State::new(window);
        let context = egui::Context::default();
        context.set_pixels_per_point(window.scale_factor() as f32);

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

    #[allow(dead_code)]
    pub fn consumed_event(
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
        let Gpu { device, queue, .. } = gpu;

        let egui::FullOutput {
            textures_delta,
            shapes,
            ..
        } = self.context.end_frame();
        for (id, image_delta) in &textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
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
        self.renderer
            .update_buffers(device, queue, encoder, &paint_jobs, &screen_descriptor);
        (paint_jobs, screen_descriptor)
    }

    pub fn begin_frame(&mut self, window: &winit::window::Window) {
        self.context.begin_frame(self.state.take_egui_input(window))
    }

    //     #[allow(dead_code)]
    //     fn scene_explorer_ui(gltf_document: gltf::Document, ui: &mut egui::Ui) {
    //         ui.heading("Scene Explorer");
    //         gltf_document.scenes().for_each(|gltf_scene| {
    //             let name = gltf_scene.name().unwrap_or("Unnamed Scene");
    //             let id = ui.make_persistent_id(ui.next_auto_id());
    //             egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
    //                 .show_header(ui, |ui| {
    //                     let response = ui.selectable_label(false, format!("🎬 {name}"));
    //                     if response.clicked() {
    //                         log::info!("Scene selected: {name}");
    //                     }
    //                 })
    //                 .body(|ui| {
    //                     gltf_scene.nodes().for_each(|node| {
    //                         Self::draw_gltf_node_ui(ui, node);
    //                     });
    //                 });
    //         })
    //     }

    //     fn draw_gltf_node_ui(ui: &mut egui::Ui, node: gltf::Node) {
    //         let name = node.name().unwrap_or("Unnamed Node");

    //         let is_leaf = node.children().len() == 0;
    //         if is_leaf {
    //             Self::node_ui(ui, name, true);
    //         }

    //         node.children().for_each(|child| {
    //             let id = ui.make_persistent_id(ui.next_auto_id());
    //             egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
    //                 .show_header(ui, |ui| {
    //                     Self::node_ui(ui, name, false);
    //                 })
    //                 .body(|ui| {
    //                     Self::draw_gltf_node_ui(ui, child);
    //                 });
    //         });
    //     }

    //     fn node_ui(ui: &mut egui::Ui, name: &str, is_leaf: bool) {
    //         let prefix = if is_leaf { "\t⭕" } else { "🔴" };
    //         let response = ui.selectable_label(false, format!("{prefix} {name}"));
    //         if response.clicked() {
    //             log::info!("Node selected: {name}");
    //         }
    //         response.context_menu(|ui| {
    //             ui.label("Shown on right-clicks");
    //         });
    //     }
}
