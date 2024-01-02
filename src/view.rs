pub struct View {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    depth_texture_view: wgpu::TextureView,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    meshes: std::collections::HashMap<String, Vec<crate::scene::PrimitiveDrawCommand>>,
}

impl View {
    pub fn new(gpu: &crate::gpu::Gpu) -> Self {
        let depth_texture_view =
            gpu.create_depth_texture(gpu.surface_config.width, gpu.surface_config.height);

        let (vertex_buffer, index_buffer) = create_geometry_buffers(&gpu.device, vec![], vec![]);

        let uniform_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &gpu.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::cast_slice(&[UniformBuffer::default()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        );

        let uniform_bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("uniform_bind_group_layout"),
                });

        let uniform_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        let pipeline = create_pipeline(gpu, &[&uniform_bind_group_layout]);

        Self {
            vertex_buffer,
            index_buffer,
            depth_texture_view,
            meshes: std::collections::HashMap::new(),
            uniform_buffer,
            uniform_bind_group,
            uniform_bind_group_layout,
            pipeline,
        }
    }

    fn ui(
        &mut self,
        gpu: &crate::gpu::Gpu,
        gui: &mut crate::gui::Gui,
        scene: &mut crate::scene::Scene,
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
                                let (vertices, indices, meshes) = scenes[0].flatten();
                                let (vertex_buffer, index_buffer) =
                                    create_geometry_buffers(&gpu.device, vertices, indices);
                                self.vertex_buffer = vertex_buffer;
                                self.index_buffer = index_buffer;
                                scenes.iter().for_each(|scene| {
                                    log::info!("{scene:#?}");
                                    log::info!("{}", scene.graph.as_dotviz());
                                });
                                self.meshes = meshes;
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

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.depth_texture_view = Self::create_depth_texture(device, width, height);
    }

    fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
        let texture = device.create_texture(
            &(wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: crate::gpu::Gpu::DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }),
        );
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn render(
        &mut self,
        window: &winit::window::Window,
        gpu: &crate::gpu::Gpu,
        gui: &mut crate::gui::Gui,
        scene: &mut crate::scene::Scene,
    ) {
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        gui.begin_frame(window);
        self.ui(gpu, gui, scene);
        let (paint_jobs, screen_descriptor) = gui.end_frame(gpu, window, &mut encoder);

        let surface_texture = gpu
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
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
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

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            scene.walk_dfs(|node| {
                if let Some(mesh) = node.mesh.as_ref() {
                    let window_size = window.inner_size();
                    let projection_matrix = nalgebra_glm::perspective_lh_zo(
                        window_size.width as f32 / window_size.height.max(1) as f32,
                        80_f32.to_radians(),
                        0.1,
                        1000.0,
                    );
                    let view_matrix = nalgebra_glm::look_at_lh(
                        &nalgebra_glm::vec3(0.0, 0.0, 10.0),
                        &nalgebra_glm::vec3(0.0, 0.0, 0.0),
                        &nalgebra_glm::Vec3::y(),
                    );

                    let uniform_buffer = UniformBuffer {
                        mvp: projection_matrix * view_matrix,
                    };

                    gpu.queue.write_buffer(
                        &self.uniform_buffer,
                        0,
                        bytemuck::cast_slice(&[uniform_buffer]),
                    );

                    render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

                    if let Some(commands) = self.meshes.get(&mesh.id) {
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
            });

            gui.renderer
                .render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));

        surface_texture.present();
    }
}

fn create_geometry_buffers(
    device: &wgpu::Device,
    vertices: Vec<crate::scene::Vertex>,
    indices: Vec<u16>,
) -> (wgpu::Buffer, wgpu::Buffer) {
    let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        },
    );
    let index_buffer = wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        },
    );
    (vertex_buffer, index_buffer)
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct UniformBuffer {
    mvp: nalgebra_glm::Mat4,
}

fn create_pipeline(
    gpu: &crate::gpu::Gpu,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
) -> wgpu::RenderPipeline {
    let shader_module = gpu
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER_SOURCE)),
        });

    let pipeline_layout = gpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts,
            push_constant_ranges: &[],
        });

    gpu.device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vertex_main",
                buffers: &[crate::scene::Vertex::description(
                    &crate::scene::Vertex::attributes(),
                )],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint16),
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::gpu::Gpu::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fragment_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        })
}

impl crate::scene::Vertex {
    pub fn attributes() -> Vec<wgpu::VertexAttribute> {
        wgpu::vertex_attr_array![
            0 => Float32x3, // position
            1 => Float32x3, // normal
            2 => Float32x2, // uv_0
            3 => Float32x2, // uv_1
            4 => Float32x4, // joint_0
            5 => Float32x4, // weight_0
            6 => Float32x3, // color_0
        ]
        .to_vec()
    }

    pub fn description(attributes: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<crate::scene::Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes,
        }
    }
}

const SHADER_SOURCE: &str = "
struct Uniform {
    mvp: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> ubo: Uniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv_0: vec2<f32>,
    @location(3) uv_1: vec2<f32>,
    @location(4) joint_0: vec4<f32>,
    @location(5) weight_0: vec4<f32>,
    @location(6) color_0: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vertex_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.color = vert.color_0;
    out.position = ubo.mvp * vec4<f32>(vert.position, 1.0);
    return out;
};

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
";
