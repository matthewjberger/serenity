fn main() {
    dragonglass::app::App::new("Dragonglass", 1920, 1080).run(Editor);
}

#[derive(Default)]
pub struct Editor;

impl dragonglass::app::State for Editor {
    fn receive_event(
        &mut self,
        context: &mut dragonglass::app::Context,
        _event: &winit::event::Event<()>,
    ) {
        if context
            .io
            .is_key_pressed(winit::event::VirtualKeyCode::Escape)
        {
            context.should_exit = true;
        }
    }

    fn update(&mut self, context: &mut dragonglass::app::Context) {
        camera_system(context);
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

fn camera_system(context: &mut dragonglass::app::Context) {
    context.scene.walk_dfs_mut(|node, _| {
        node.components.iter_mut().for_each(|component| {
            if let dragonglass::scene::NodeComponent::Camera(camera) = component {
                let speed = (1.0_f64 * context.delta_time) as f32;

                if context.io.is_key_pressed(winit::event::VirtualKeyCode::W) {
                    camera.orientation.offset -= camera.orientation.direction() * speed;
                }
                if context.io.is_key_pressed(winit::event::VirtualKeyCode::A) {
                    camera.orientation.offset += camera.orientation.right() * speed;
                }
                if context.io.is_key_pressed(winit::event::VirtualKeyCode::S) {
                    camera.orientation.offset += camera.orientation.direction() * speed;
                }
                if context.io.is_key_pressed(winit::event::VirtualKeyCode::D) {
                    camera.orientation.offset -= camera.orientation.right() * speed;
                }
                if context
                    .io
                    .is_key_pressed(winit::event::VirtualKeyCode::Space)
                {
                    camera.orientation.offset += camera.orientation.up() * speed;
                }
                if context
                    .io
                    .is_key_pressed(winit::event::VirtualKeyCode::LShift)
                {
                    camera.orientation.offset -= camera.orientation.up() * speed;
                }

                camera
                    .orientation
                    .zoom(6.0 * context.io.mouse.wheel_delta.y * (context.delta_time as f32));

                if context.io.mouse.is_middle_clicked {
                    camera
                        .orientation
                        .pan(&(context.io.mouse.position_delta * context.delta_time as f32));
                }
                node.transform.translation = camera.orientation.position();

                if context.io.mouse.is_right_clicked {
                    if context
                        .io
                        .is_key_pressed(winit::event::VirtualKeyCode::LAlt)
                    {
                        camera.orientation.offset = nalgebra_glm::Vec3::new(0.0, 0.0, 0.0);
                    }

                    let mut delta = context.io.mouse.position_delta * context.delta_time as f32;
                    delta.x *= -1.0;
                    camera.orientation.rotate(&delta);
                }

                node.transform.rotation = camera.orientation.look_at_offset();
            }
        });
    });
}
