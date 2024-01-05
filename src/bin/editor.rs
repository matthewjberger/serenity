fn main() {
    dragonglass::app::App::new("Dragonglass", 1920, 1080).run(Editor);
}

#[derive(Default)]
pub struct Editor;

impl dragonglass::app::State for Editor {
    fn receive_events(
        &mut self,
        context: &mut dragonglass::app::Context,
        _event: &winit::event::Event<'_, ()>,
        control_flow: &mut winit::event_loop::ControlFlow,
    ) {
        if context
            .io
            .is_key_pressed(winit::event::VirtualKeyCode::Escape)
        {
            *control_flow = winit::event_loop::ControlFlow::Exit;
        }
    }

    fn update(&mut self, context: &mut dragonglass::app::Context) {
        context.scene.walk_dfs_mut(|node, _| {
            node.components.iter_mut().for_each(|component| {
                if let dragonglass::scene::NodeComponent::Camera(_camera) = component {
                    if context.io.is_key_pressed(winit::event::VirtualKeyCode::W) {
                        node.transform.translation.z -= (0.05_f64 * context.delta_time) as f32;
                    }
                    if context.io.is_key_pressed(winit::event::VirtualKeyCode::A) {
                        node.transform.translation.x -= (0.05_f64 * context.delta_time) as f32;
                    }
                    if context.io.is_key_pressed(winit::event::VirtualKeyCode::S) {
                        node.transform.translation.z += (0.05_f64 * context.delta_time) as f32;
                    }
                    if context.io.is_key_pressed(winit::event::VirtualKeyCode::D) {
                        node.transform.translation.x += (0.05_f64 * context.delta_time) as f32;
                    }
                    if context
                        .io
                        .is_key_pressed(winit::event::VirtualKeyCode::Space)
                    {
                        node.transform.translation.y += (0.05_f64 * context.delta_time) as f32;
                    }
                    if context
                        .io
                        .is_key_pressed(winit::event::VirtualKeyCode::LShift)
                    {
                        node.transform.translation.y -= (0.05_f64 * context.delta_time) as f32;
                    }
                }
            });
        });
    }

    fn ui(&mut self, context: &mut dragonglass::app::Context) {
        egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .show(&context.gui.context, |ui| {
                egui::menu::bar(ui, |ui| {
                    egui::global_dark_light_mode_switch(ui);
                    ui.menu_button("File", |ui| {
                        if ui.button("Import asset (gltf/glb)...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("GLTF / GLB", &["gltf", "glb"])
                                .pick_file()
                            {
                                let scenes = dragonglass::gltf::import_gltf(path)
                                    .expect("Failed to import gltf!");
                                context.scene = scenes[0].clone();
                                if !context.scene.has_camera() {
                                    context.scene.add_root_node(
                                        dragonglass::scene::create_camera_node(
                                            context.gpu.aspect_ratio(),
                                        ),
                                    );
                                }
                                context.view.import_scene(&scenes[0], &context.gpu);
                            }
                        };
                    });
                });
            });

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .show(&context.gui.context, |ui| {
                ui.heading("Scene Tree");
            });
    }
}
