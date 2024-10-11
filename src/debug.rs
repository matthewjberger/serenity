use crate::world::Shape;
use nalgebra_glm::{pi, Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;

pub struct DebugRender {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub debug_pipeline: wgpu::RenderPipeline,
    cube_indices_count: u32,
    sphere_indices_count: u32,
    capsule_indices_count: u32,
}

impl DebugRender {
    pub fn new(gpu: &crate::gpu::Gpu) -> Self {
        let (vertices, indices, cube_indices_count, sphere_indices_count, capsule_indices_count) =
            create_debug_geometry();

        let vertex_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Debug Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Debug Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        let instance_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: &[],
                usage: wgpu::BufferUsages::VERTEX,
            });

        let uniform_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("uniform_buffer"),
                contents: bytemuck::cast_slice(&[Uniform::default()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

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

        let debug_pipeline = create_debug_pipeline(gpu, &uniform_bind_group_layout);

        Self {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            uniform_buffer,
            uniform_bind_group,
            debug_pipeline,
            cube_indices_count,
            sphere_indices_count,
            capsule_indices_count,
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
                    camera_position: Vec4::new(
                        camera_position.x,
                        camera_position.y,
                        camera_position.z,
                        1.0,
                    ),
                }]),
            );

            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            render_pass.set_pipeline(&self.debug_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            let instance_data_size = std::mem::size_of::<InstanceBinding>();
            let instance_count = (self.instance_buffer.size() / instance_data_size as u64) as u32;

            println!("Debug: Instance buffer size: {}, Instance data size: {}, Calculated instance count: {}",
                        self.instance_buffer.size(), instance_data_size, instance_count);

            [Shape::Cube, Shape::Sphere, Shape::Capsule].iter().for_each(|shape| {
                let mut shape_instance_count = 0;
                scene.graph.node_indices().for_each(|graph_node_index| {
                    let node_index = scene.graph[graph_node_index];
                    let node = &context.world.nodes[node_index];

                    if node.mesh_index.is_none() {
                        return;
                    }

                    if let Some(primitive_mesh_index) = node.primitive_mesh_index {
                        let primitive_mesh = &context.world.primitive_meshes[primitive_mesh_index];

                        if &primitive_mesh.shape != shape {
                            return;
                        }

                        if shape_instance_count >= instance_count {
                            println!("Debug: Skipping instance due to buffer limit. Shape: {:?}, Instance count: {}", shape, shape_instance_count);
                            return;
                        }

                        match primitive_mesh.shape {
                            Shape::Cube => {
                                render_pass.draw_indexed(
                                    0..self.cube_indices_count,
                                    0,
                                    shape_instance_count..(shape_instance_count + 1),
                                );
                            }
                            Shape::Sphere => {
                                render_pass.draw_indexed(
                                    self.cube_indices_count..(self.cube_indices_count + self.sphere_indices_count),
                                    0,
                                    shape_instance_count..(shape_instance_count + 1),
                                );
                            }
                            Shape::Capsule => {
                                render_pass.draw_indexed(
                                    (self.cube_indices_count + self.sphere_indices_count)..
                                        (self.cube_indices_count + self.sphere_indices_count + self.capsule_indices_count),
                                    0,
                                    shape_instance_count..(shape_instance_count + 1),
                                );
                            }
                        }
                        shape_instance_count += 1;
                    }
                });
                println!("Debug: Total instances drawn for {:?}: {}", shape, shape_instance_count);
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

                if let (Some(primitive_mesh_index), Some(aabb_index)) =
                    (node.primitive_mesh_index, node.aabb_index)
                {
                    let primitive_mesh = &context.world.primitive_meshes[primitive_mesh_index];
                    let aabb = &context.world.aabbs[aabb_index];
                    let half_extents = aabb.half_extents();
                    let dimension = half_extents.x.max(half_extents.y).max(half_extents.z);
                    match node.aabb_index {
                        Some(aabb) => {
                            let aabb = &context.world.aabbs[aabb];
                            let transform = context
                                .world
                                .global_transform(&scene.graph, graph_node_index);
                            let instance_binding = InstanceBinding {
                                model: transform
                                    * Mat4::new_translation(&aabb.center())
                                    * Mat4::new_scaling(dimension),
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
            self.instance_buffer =
                gpu.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Instance Buffer"),
                        contents: bytemuck::cast_slice(&instance_bindings),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });
        } else {
            gpu.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&instance_bindings),
            );
        }
    }
}

fn create_debug_geometry() -> (Vec<Vertex>, Vec<u16>, u32, u32, u32) {
    let cube_vertices = vec![
        Vertex {
            position: Vec3::new(-1.0, -1.0, -1.0),
        },
        Vertex {
            position: Vec3::new(1.0, -1.0, -1.0),
        },
        Vertex {
            position: Vec3::new(1.0, 1.0, -1.0),
        },
        Vertex {
            position: Vec3::new(-1.0, 1.0, -1.0),
        },
        Vertex {
            position: Vec3::new(-1.0, -1.0, 1.0),
        },
        Vertex {
            position: Vec3::new(1.0, -1.0, 1.0),
        },
        Vertex {
            position: Vec3::new(1.0, 1.0, 1.0),
        },
        Vertex {
            position: Vec3::new(-1.0, 1.0, 1.0),
        },
    ];

    let cube_indices = vec![
        0, 1, 1, 2, 2, 3, 3, 0, // Front face
        4, 5, 5, 6, 6, 7, 7, 4, // Back face
        0, 4, 1, 5, 2, 6, 3, 7, // Connecting edges
    ];

    let (sphere_vertices, sphere_indices) =
        generate_sphere_wireframe(1.0, SPHERE_SEGMENTS, SPHERE_RINGS);

    // TODO: rename capsule->cylinder
    let (capsule_vertices, capsule_indices) =
        generate_cylinder_wireframe(1.0, 2.0, CAPSULE_SEGMENTS);

    let mut vertices = cube_vertices;
    let cube_vertices_count = vertices.len() as u16;
    vertices.extend(sphere_vertices);
    let sphere_vertices_count = vertices.len() as u16 - cube_vertices_count;
    vertices.extend(capsule_vertices);

    let mut indices = cube_indices;
    let cube_indices_count = indices.len() as u32;
    indices.extend(sphere_indices.into_iter().map(|i| i + cube_vertices_count));
    let sphere_indices_count = indices.len() as u32 - cube_indices_count;
    indices.extend(
        capsule_indices
            .into_iter()
            .map(|i| i + cube_vertices_count + sphere_vertices_count),
    );
    let capsule_indices_count = indices.len() as u32 - cube_indices_count - sphere_indices_count;

    (
        vertices,
        indices,
        cube_indices_count,
        sphere_indices_count,
        capsule_indices_count,
    )
}

fn generate_sphere_wireframe(radius: f32, segments: u32, rings: u32) -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Generate vertices
    for ring in 0..=rings {
        let phi = core::f32::consts::PI * ring as f32 / rings as f32;
        for segment in 0..=segments {
            let theta = 2.0 * core::f32::consts::PI * segment as f32 / segments as f32;
            let x = radius * phi.sin() * theta.cos();
            let y = radius * phi.cos();
            let z = radius * phi.sin() * theta.sin();
            vertices.push(Vertex {
                position: Vec3::new(x, y, z),
            });
        }
    }

    // Generate indices for latitude lines
    for ring in 0..=rings {
        for segment in 0..segments {
            let current = ring * (segments + 1) + segment;
            let next = current + 1;
            indices.push(current as u16);
            indices.push(next as u16);
        }
    }

    // Generate indices for longitude lines
    for segment in 0..=segments {
        for ring in 0..rings {
            let current = ring * (segments + 1) + segment;
            let next = current + (segments + 1);
            indices.push(current as u16);
            indices.push(next as u16);
        }
    }

    (vertices, indices)
}

fn generate_cylinder_wireframe(radius: f32, height: f32, segments: u32) -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let half_radius = radius / 2.0;
    let half_height = height / 2.0; // Use the full half-height

    // Generate vertices
    // Top circle
    for segment in 0..segments {
        let theta = 2.0 * pi::<f32>() * segment as f32 / segments as f32;
        let x = half_radius * theta.cos();
        let z = half_radius * theta.sin();
        vertices.push(Vertex {
            position: Vec3::new(x, half_height, z),
        });
    }

    // Bottom circle
    for segment in 0..segments {
        let theta = 2.0 * pi::<f32>() * segment as f32 / segments as f32;
        let x = half_radius * theta.cos();
        let z = half_radius * theta.sin();
        vertices.push(Vertex {
            position: Vec3::new(x, -half_height, z),
        });
    }

    // Generate indices
    // Top circle
    for segment in 0..segments {
        let next = (segment + 1) % segments;
        indices.push(segment as u16);
        indices.push(next as u16);
    }

    // Bottom circle
    for segment in 0..segments {
        let next = (segment + 1) % segments;
        indices.push((segments + segment) as u16);
        indices.push((segments + next) as u16);
    }

    // Vertical lines
    for segment in 0..segments {
        indices.push(segment as u16);
        indices.push((segments + segment) as u16);
    }

    (vertices, indices)
}

const SPHERE_SEGMENTS: u32 = 32;
const SPHERE_RINGS: u32 = 16;

const CAPSULE_SEGMENTS: u32 = 32;

fn create_debug_pipeline(
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
                polygon_mode: wgpu::PolygonMode::Line,
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
    pub view: Mat4,
    pub projection: Mat4,
    pub camera_position: Vec4,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: Vec3,
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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceBinding {
    pub model: Mat4,
    pub color: Vec4,
}

impl InstanceBinding {
    pub fn vertex_attributes() -> Vec<wgpu::VertexAttribute> {
        wgpu::vertex_attr_array![
            2 => Float32x4,
            3 => Float32x4,
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4
        ]
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

const SHADER_SOURCE: &str = r#"
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
            }

            @fragment
            fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
                return in.color;
            }
            "#;
