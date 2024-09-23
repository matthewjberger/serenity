// TODO: add other shapes for instanced debug display
//       line, sphere, capsule
use nalgebra_glm as glm;
use wgpu::util::DeviceExt;

pub struct DebugRender {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub cube_pipeline: wgpu::RenderPipeline,
    pub line_pipeline: wgpu::RenderPipeline,
    pub grid_vertices: wgpu::Buffer,
    pub grid_instances: wgpu::Buffer,
}

impl DebugRender {
    pub fn new(gpu: &crate::gpu::Gpu) -> Self {
        let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &gpu.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Cube Vertex Buffer"),
                contents: bytemuck::cast_slice(&VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );
        let index_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &gpu.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Cube Index Buffer"),
                contents: bytemuck::cast_slice(&INDICES),
                usage: wgpu::BufferUsages::INDEX,
            },
        );

        let instance_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &gpu.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: &[],
                usage: wgpu::BufferUsages::VERTEX,
            },
        );

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

        let cube_pipeline = create_cube_pipeline(gpu, &uniform_bind_group_layout);

        let line_pipeline = create_line_pipeline(gpu, &uniform_bind_group_layout);

        let (grid_vertices, grid_instances) = create_grid_buffers(gpu);

        Self {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            uniform_buffer,
            uniform_bind_group,
            cube_pipeline,
            line_pipeline,
            grid_vertices,
            grid_instances,
        }
    }

    pub fn render<'rp>(
        &'rp mut self,
        render_pass: &mut wgpu::RenderPass<'rp>,
        gpu: &crate::gpu::Gpu,
        context: &crate::app::Context,
    ) {
        if let Some(scene_index) = context.active_scene_index {
            let scene = &context.world.scenes[scene_index];

            let (camera_position, projection, view) =
                crate::world::create_camera_matrices(&context.world, scene, gpu.aspect_ratio());

            gpu.queue.write_buffer(
                &self.uniform_buffer,
                0,
                bytemuck::cast_slice(&[Uniform {
                    view,
                    projection,
                    camera_position: nalgebra_glm::vec3_to_vec4(&camera_position),
                }]),
            );

            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            // Render grid
            render_pass.set_pipeline(&self.line_pipeline);
            render_pass.set_vertex_buffer(0, self.grid_vertices.slice(..));
            render_pass.set_vertex_buffer(1, self.grid_instances.slice(..));
            let num_grid_lines = (GRID_SIZE + 1) * 2; // Vertical + Horizontal lines
            render_pass.draw(0..2, 0..num_grid_lines);

            // Render debug shapes
            render_pass.set_pipeline(&self.cube_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            [crate::world::Shape::Cube, crate::world::Shape::CubeExtents]
                .iter()
                .for_each(|shape| {
                    scene.graph.node_indices().for_each(|graph_node_index| {
                        let node_index = scene.graph[graph_node_index];
                        let node = &context.world.nodes[node_index];

                        if node.mesh_index.is_none() {
                            return;
                        }

                        if let Some(primitive_mesh_index) = node.primitive_mesh_index {
                            let primitive_mesh =
                                &context.world.primitive_meshes[primitive_mesh_index];

                            if &primitive_mesh.shape != shape {
                                return;
                            }

                            let instance_offset = primitive_mesh_index as u32;
                            match primitive_mesh.shape {
                                crate::world::Shape::CubeExtents | crate::world::Shape::Cube => {
                                    render_pass.draw_indexed(
                                        0..(INDICES.len() as u32),
                                        0,
                                        instance_offset..(instance_offset + 1),
                                    );
                                }
                            }
                        }
                    });
                });
        }
    }

    pub fn sync_context(&mut self, context: &crate::app::Context, gpu: &crate::gpu::Gpu) {
        let mut instance_bindings = Vec::new();

        if let Some(scene_index) = context.active_scene_index {
            let scene = &context.world.scenes[scene_index];
            scene.graph.node_indices().for_each(|graph_node_index| {
                let node_index = scene.graph[graph_node_index];
                let node = &context.world.nodes[node_index];

                if let Some(primitive_mesh_index) = node.primitive_mesh_index {
                    let primitive_mesh = &context.world.primitive_meshes[primitive_mesh_index];
                    match node.aabb_index {
                        Some(aabb) => {
                            let aabb = &context.world.aabbs[aabb];
                            let transform = context
                                .world
                                .global_transform(&scene.graph, graph_node_index);
                            let instance_binding = InstanceBinding {
                                model: (transform
                                    * nalgebra_glm::translation(&aabb.center())
                                    * nalgebra_glm::scaling(&(aabb.extents() / 2.0))),
                                color: primitive_mesh.color,
                            };
                            instance_bindings.push(instance_binding);
                        }
                        None => {
                            let model = context
                                .world
                                .global_transform(&scene.graph, graph_node_index);
                            let instance_binding = InstanceBinding {
                                model,
                                color: primitive_mesh.color,
                            };
                            instance_bindings.push(instance_binding);
                        }
                    }
                }
            });
        }

        if (self.instance_buffer.size() as usize)
            < instance_bindings.len() * std::mem::size_of::<InstanceBinding>()
        {
            self.instance_buffer = wgpu::util::DeviceExt::create_buffer_init(
                &gpu.device,
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Instance Buffer"),
                    contents: bytemuck::cast_slice(&instance_bindings),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                },
            );
        } else {
            gpu.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&instance_bindings),
            );
        }
    }
}

fn create_cube_pipeline(
    gpu: &crate::gpu::Gpu,
    uniform_bind_group_layout: &wgpu::BindGroupLayout,
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
            bind_group_layouts: &[uniform_bind_group_layout],
            push_constant_ranges: &[],
        });
    gpu.device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vertex_main",
                buffers: &[
                    Vertex::description(&Vertex::vertex_attributes()),
                    InstanceBinding::description(&InstanceBinding::vertex_attributes()),
                ],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
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

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniform {
    pub view: nalgebra_glm::Mat4,
    pub projection: nalgebra_glm::Mat4,
    pub camera_position: nalgebra_glm::Vec4,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: nalgebra_glm::Vec3,
}

impl Vertex {
    pub fn vertex_attributes() -> Vec<wgpu::VertexAttribute> {
        wgpu::vertex_attr_array![0 => Float32x3].to_vec()
    }

    pub fn description(attributes: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes,
        }
    }
}

const VERTICES: [Vertex; 8] = [
    Vertex {
        position: nalgebra_glm::Vec3::new(-1.0, -1.0, -1.0),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(1.0, -1.0, -1.0),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(1.0, 1.0, -1.0),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(-1.0, 1.0, -1.0),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(-1.0, -1.0, 1.0),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(1.0, -1.0, 1.0),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(1.0, 1.0, 1.0),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(-1.0, 1.0, 1.0),
    },
];

const INDICES: [u16; 24] = [
    0, 1, 1, 2, 2, 3, 3, 0, // Front face
    4, 5, 5, 6, 6, 7, 7, 4, // Back face
    0, 4, 1, 5, 2, 6, 3, 7, // Connecting edges
];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceBinding {
    pub model: nalgebra_glm::Mat4,
    pub color: nalgebra_glm::Vec4,
}

impl InstanceBinding {
    pub fn vertex_attributes() -> Vec<wgpu::VertexAttribute> {
        wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4]
            .to_vec()
    }

    pub fn description(attributes: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes,
        }
    }
}

const SHADER_SOURCE: &str = "
struct Uniform {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    camera_position: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> ubo: Uniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct InstanceInput {
    @location(2) model_matrix_0: vec4<f32>,
    @location(3) model_matrix_1: vec4<f32>,
    @location(4) model_matrix_2: vec4<f32>,
    @location(5) model_matrix_3: vec4<f32>,
    @location(6) color: vec4<f32>,
};

@vertex
fn vertex_main(vert: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var out: VertexOutput;
    out.position = ubo.projection * ubo.view * model_matrix * vec4(vert.position, 1.0);
    out.color = instance.color;
    return out;
};

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
";

fn create_line_pipeline(
    gpu: &crate::gpu::Gpu,
    uniform_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader_module = gpu
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(LINE_SHADER_SOURCE)),
        });
    let pipeline_layout = gpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[uniform_bind_group_layout],
            push_constant_ranges: &[],
        });
    gpu.device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vertex_main",
                buffers: &[
                    LineVertex::description(&LineVertex::vertex_attributes()),
                    LineInstance::description(&LineInstance::vertex_attributes()),
                ],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LineVertex {
    position: glm::Vec3,
}

impl LineVertex {
    pub fn vertex_attributes() -> Vec<wgpu::VertexAttribute> {
        wgpu::vertex_attr_array![0 => Float32x3].to_vec()
    }

    pub fn description(attributes: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LineInstance {
    start: glm::Vec3,
    end: glm::Vec3,
    color: glm::Vec4,
}

impl LineInstance {
    pub fn vertex_attributes() -> Vec<wgpu::VertexAttribute> {
        wgpu::vertex_attr_array![1 => Float32x3, 2 => Float32x3, 3 => Float32x4].to_vec()
    }

    pub fn description(attributes: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes,
        }
    }
}

const GRID_SIZE: u32 = 1000;
const GRID_STEP: f32 = 1.0;

fn create_grid_buffers(gpu: &crate::gpu::Gpu) -> (wgpu::Buffer, wgpu::Buffer) {
    let vertices = [
        LineVertex {
            position: glm::vec3(0.0, 0.0, 0.0),
        },
        LineVertex {
            position: glm::vec3(1.0, 0.0, 0.0),
        },
    ];

    let vertex_buffer = gpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

    let mut instances = Vec::new();
    let grid_color = glm::vec4(0.5, 0.5, 0.5, 1.0);
    let half_size = (GRID_SIZE as f32 * GRID_STEP) / 2.0;

    for i in 0..=GRID_SIZE {
        let pos = i as f32 * GRID_STEP - half_size;
        instances.push(LineInstance {
            start: glm::vec3(pos, 0.0, -half_size),
            end: glm::vec3(pos, 0.0, half_size),
            color: grid_color,
        });
        instances.push(LineInstance {
            start: glm::vec3(-half_size, 0.0, pos),
            end: glm::vec3(half_size, 0.0, pos),
            color: grid_color,
        });
    }

    let instance_buffer = gpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Instance Buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX,
        });

    (vertex_buffer, instance_buffer)
}

const LINE_SHADER_SOURCE: &str = r#"
struct Uniform {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    camera_position: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> ubo: Uniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct InstanceInput {
    @location(1) start: vec3<f32>,
    @location(2) end: vec3<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex_main(
    vert: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let t = vert.position.x;
    let world_position = mix(instance.start, instance.end, t);

    var out: VertexOutput;
    out.clip_position = ubo.projection * ubo.view * vec4<f32>(world_position, 1.0);
    out.color = instance.color;
    return out;
}

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;
