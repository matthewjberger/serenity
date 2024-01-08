pub struct View {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub dynamic_uniform_buffer: wgpu::Buffer,
    pub dynamic_uniform_bind_group: wgpu::BindGroup,
    pub pipeline: wgpu::RenderPipeline,
    pub mesh_draw_commands:
        std::collections::HashMap<String, Vec<crate::scene::PrimitiveDrawCommand>>,
}

impl View {
    pub const MAX_NUMBER_OF_MESHES: usize = 10_000;

    pub fn new(gpu: &crate::gpu::Gpu) -> Self {
        let (vertex_buffer, index_buffer) = create_geometry_buffers(&gpu.device, &[], &[]);
        let (uniform_buffer, uniform_bind_group_layout, uniform_bind_group) = create_uniform(gpu);
        let (dynamic_uniform_buffer, dynamic_uniform_bind_group_layout, dynamic_uniform_bind_group) =
            create_dynamic_uniform(gpu, Self::MAX_NUMBER_OF_MESHES as _);

        let pipeline = create_pipeline(
            gpu,
            &[
                &uniform_bind_group_layout,
                &dynamic_uniform_bind_group_layout,
            ],
        );

        Self {
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group,
            dynamic_uniform_buffer,
            dynamic_uniform_bind_group,
            pipeline,
            mesh_draw_commands: std::collections::HashMap::new(),
        }
    }

    pub fn render<'rp>(
        &'rp self,
        render_pass: &mut wgpu::RenderPass<'rp>,
        gpu: &crate::gpu::Gpu,
        scene: &crate::scene::Scene,
    ) {
        let (camera_position, projection, view) =
            create_camera_matrices(scene, gpu.aspect_ratio()).unwrap_or_default();
        gpu.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[Uniform {
                view,
                projection,
                camera_position: nalgebra_glm::vec3_to_vec4(&camera_position),
            }]),
        );

        let mut mesh_ubos = vec![DynamicUniform::default(); View::MAX_NUMBER_OF_MESHES];
        let mut ubo_offset = 0;
        scene.walk_dfs(|_, node_index| {
            mesh_ubos[ubo_offset] = DynamicUniform {
                model: scene.graph.global_transform(node_index),
            };
            ubo_offset += 1;
        });
        gpu.queue
            .write_buffer(&self.dynamic_uniform_buffer, 0, unsafe {
                std::slice::from_raw_parts(
                    mesh_ubos.as_ptr() as *const u8,
                    mesh_ubos.len() * gpu.alignment() as usize,
                )
            });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        let mut ubo_offset = 0;
        scene.walk_dfs(|node, _| {
            let offset = ubo_offset;
            ubo_offset += 1;
            node.components.iter().for_each(|component| {
                if let crate::scene::NodeComponent::Mesh(mesh_id) = component {
                    let offset = (offset * gpu.alignment()) as wgpu::DynamicOffset;
                    render_pass.set_bind_group(1, &self.dynamic_uniform_bind_group, &[offset]);
                    if let Some(commands) = self.mesh_draw_commands.get(mesh_id) {
                        execute_draw_commands(commands, render_pass);
                    }
                }
            });
        });
    }

    pub fn import_scene(&mut self, scene: &crate::scene::Scene, gpu: &crate::gpu::Gpu) {
        let (vertices, indices, mesh_draw_commands) = scene.flatten_geometry();
        let (vertex_buffer, index_buffer) =
            create_geometry_buffers(&gpu.device, &vertices, &indices);
        self.vertex_buffer = vertex_buffer;
        self.index_buffer = index_buffer;
        self.mesh_draw_commands = mesh_draw_commands;
    }
}

fn execute_draw_commands(
    commands: &[crate::scene::PrimitiveDrawCommand],
    render_pass: &mut wgpu::RenderPass,
) {
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

fn create_dynamic_uniform(
    gpu: &crate::gpu::Gpu,
    max_meshes: wgpu::BufferAddress,
) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
    let dynamic_uniform_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("dynamic_uniform_buffer"),
        size: max_meshes * gpu.alignment(),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let dynamic_uniform_bind_group_layout =
        gpu.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<DynamicUniform>() as _,
                        ),
                    },
                    count: None,
                }],
                label: Some("dynamic_uniform_buffer_layout"),
            });

    let dynamic_uniform_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &dynamic_uniform_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &dynamic_uniform_buffer,
                offset: 0,
                size: wgpu::BufferSize::new(std::mem::size_of::<DynamicUniform>() as _),
            }),
        }],
        label: Some("dynamic_uniform_bind_group"),
    });

    (
        dynamic_uniform_buffer,
        dynamic_uniform_bind_group_layout,
        dynamic_uniform_bind_group,
    )
}

fn create_uniform(gpu: &crate::gpu::Gpu) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
    let uniform_buffer = wgpu::util::DeviceExt::create_buffer_init(
        &gpu.device,
        &wgpu::util::BufferInitDescriptor {
            label: Some("uniform_buffer"),
            contents: bytemuck::cast_slice(&[Uniform::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        },
    );

    let uniform_bind_group_layout =
        gpu.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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

    (
        uniform_buffer,
        uniform_bind_group_layout,
        uniform_bind_group,
    )
}

pub fn create_camera_matrices(
    scene: &crate::scene::Scene,
    aspect_ratio: f32,
) -> Option<(nalgebra_glm::Vec3, nalgebra_glm::Mat4, nalgebra_glm::Mat4)> {
    let mut result = None;
    scene.walk_dfs(|node, _| {
        for component in node.components.iter() {
            if let crate::scene::NodeComponent::Camera(camera) = component {
                result = Some((
                    // TODO: later this will need to be the translation of the global transform,
                    //       need to be able to aggregate transforms without turning them in to glm::Mat4 first
                    node.transform.translation,
                    camera.projection_matrix(aspect_ratio),
                    {
                        let eye = node.transform.translation;
                        let target = eye
                            + nalgebra_glm::quat_rotate_vec3(
                                &node.transform.rotation.normalize(),
                                &(-nalgebra_glm::Vec3::z()),
                            );
                        let up = nalgebra_glm::quat_rotate_vec3(
                            &node.transform.rotation.normalize(),
                            &nalgebra_glm::Vec3::y(),
                        );
                        nalgebra_glm::look_at(&eye, &target, &up)
                    },
                ));
            }
        }
    });
    result
}

fn create_geometry_buffers(
    device: &wgpu::Device,
    vertices: &[crate::scene::Vertex],
    indices: &[u16],
) -> (wgpu::Buffer, wgpu::Buffer) {
    let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        },
    );
    let index_buffer = wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        },
    );
    (vertex_buffer, index_buffer)
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Light {
    position: nalgebra_glm::Vec4,
    color: nalgebra_glm::Vec4,
}

impl Light {
    pub fn new(position: nalgebra_glm::Vec3, color: nalgebra_glm::Vec3) -> Self {
        let mut position = nalgebra_glm::vec3_to_vec4(&position);
        position.w = 1.0;
        let mut color = nalgebra_glm::vec3_to_vec4(&color);
        color.w = 1.0;
        Self { position, color }
    }
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniform {
    pub view: nalgebra_glm::Mat4,
    pub projection: nalgebra_glm::Mat4,
    pub camera_position: nalgebra_glm::Vec4,
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
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Line,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
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

#[repr(C, align(256))]
#[derive(Default, Copy, Clone, Debug, bytemuck::Zeroable)]
pub struct DynamicUniform {
    pub model: nalgebra_glm::Mat4,
}

const SHADER_SOURCE: &str = "
struct Uniform {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    camera_position: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> ubo: Uniform;

struct DynamicUniform {
    model: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> mesh_ubo: DynamicUniform;

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
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
};

@vertex
fn vertex_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let mvp = ubo.projection * ubo.view * mesh_ubo.model;
    out.color = vert.color_0;
    out.position = mvp * vec4(vert.position, 1.0);
    out.normal = vec4((mvp * vec4(vert.normal, 0.0)).xyz, 1.0).xyz;
    return out;
};

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color: vec4<f32> = vec4(in.color, 1.0);

    let ambient_strength = 0.1;
    let ambient_color = ambient_strength;

    let result = (ambient_color) * object_color.rgb;

    return vec4<f32>(result, object_color.a);
}
";
