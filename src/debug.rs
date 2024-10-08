use crate::world::Shape;
use nalgebra_glm::{Mat4, Vec3, Vec4};
use std::f32::consts::PI;
use wgpu::util::DeviceExt;

pub struct DebugRender {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub shape_pipeline: wgpu::RenderPipeline,
    shape_index_ranges: ShapeIndexRanges,
}

struct ShapeIndexRanges {
    cube: (u32, u32),
    sphere: (u32, u32),
    capsule: (u32, u32),
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShapeInstance {
    model: Mat4,
    color: Vec4,
}

impl DebugRender {
    pub fn new(gpu: &crate::gpu::Gpu) -> Self {
        let (vertices, indices, shape_index_ranges) = create_shape_geometry();

        let vertex_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Debug Shape Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Debug Shape Index Buffer"),
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

        let shape_pipeline = create_shape_pipeline(gpu, &uniform_bind_group_layout);

        Self {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            uniform_buffer,
            uniform_bind_group,
            shape_pipeline,
            shape_index_ranges,
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
            render_pass.set_pipeline(&self.shape_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            let instance_count = self.get_instance_count(context);

            // Draw cubes
            render_pass.draw_indexed(
                self.shape_index_ranges.cube.0..self.shape_index_ranges.cube.1,
                0,
                0..instance_count,
            );

            // Draw spheres
            render_pass.draw_indexed(
                self.shape_index_ranges.sphere.0..self.shape_index_ranges.sphere.1,
                0,
                0..instance_count,
            );

            // Draw capsules
            render_pass.draw_indexed(
                self.shape_index_ranges.capsule.0..self.shape_index_ranges.capsule.1,
                0,
                0..instance_count,
            );
        }
    }

    pub fn sync_context(&mut self, context: &crate::app::Context, gpu: &crate::gpu::Gpu) {
        let mut shape_instances = Vec::new();

        if let Some(scene_index) = context.active_scene_index {
            let scene = &context.world.scenes[scene_index];
            scene.graph.node_indices().for_each(|graph_node_index| {
                let node_index = scene.graph[graph_node_index];
                let node = &context.world.nodes[node_index];

                if let Some(primitive_mesh_index) = node.primitive_mesh_index {
                    let primitive_mesh = &context.world.primitive_meshes[primitive_mesh_index];
                    let transform = context
                        .world
                        .global_transform(&scene.graph, graph_node_index);

                    let model = match primitive_mesh.shape {
                        Shape::Cube => transform,
                        Shape::Sphere { radius } => {
                            transform * nalgebra_glm::scaling(&Vec3::new(radius, radius, radius))
                        }
                        Shape::Capsule { radius, height } => {
                            let scale = nalgebra_glm::scaling(&Vec3::new(radius, height, radius));
                            transform * scale
                        }
                        Shape::Cuboid { half_extents } => {
                            transform * nalgebra_glm::scaling(&half_extents)
                        }
                    };

                    shape_instances.push(ShapeInstance {
                        model,
                        color: primitive_mesh.color,
                    });
                }
            });
        }

        if (self.instance_buffer.size() as usize)
            < shape_instances.len() * std::mem::size_of::<ShapeInstance>()
        {
            self.instance_buffer =
                gpu.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Shape Instance Buffer"),
                        contents: bytemuck::cast_slice(&shape_instances),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });
        } else {
            gpu.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&shape_instances),
            );
        }
    }

    fn get_instance_count(&self, context: &crate::app::Context) -> u32 {
        if let Some(scene_index) = context.active_scene_index {
            let scene = &context.world.scenes[scene_index];
            scene
                .graph
                .node_indices()
                .filter(|&graph_node_index| {
                    let node_index = scene.graph[graph_node_index];
                    let node = &context.world.nodes[node_index];
                    node.primitive_mesh_index.is_some()
                })
                .count() as u32
        } else {
            0
        }
    }
}

fn create_shape_geometry() -> (Vec<Vertex>, Vec<u16>, ShapeIndexRanges) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut ranges = ShapeIndexRanges {
        cube: (0, 0),
        sphere: (0, 0),
        capsule: (0, 0),
    };

    // Create cube geometry
    let cube_vertices = create_cube_vertices();
    let cube_indices = create_cube_indices(vertices.len() as u16);
    vertices.extend(cube_vertices);
    indices.extend(cube_indices);
    ranges.cube = (0, indices.len() as u32);

    // Create sphere geometry
    let sphere_vertices = create_sphere_vertices(32, 16);
    let sphere_indices = create_sphere_indices(32, 16, vertices.len() as u16);
    vertices.extend(sphere_vertices);
    indices.extend(sphere_indices);
    ranges.sphere = (ranges.cube.1, indices.len() as u32);

    // Create capsule geometry
    let capsule_vertices = create_capsule_vertices(32, 16);
    let capsule_indices = create_capsule_indices(32, 16, vertices.len() as u16);
    vertices.extend(capsule_vertices);
    indices.extend(capsule_indices);
    ranges.capsule = (ranges.sphere.1, indices.len() as u32);

    (vertices, indices, ranges)
}

fn create_cube_vertices() -> Vec<Vertex> {
    vec![
        // Front face
        Vertex {
            position: [-0.5, -0.5, 0.5],
            normal: [0.0, 0.0, 1.0],
        },
        Vertex {
            position: [0.5, -0.5, 0.5],
            normal: [0.0, 0.0, 1.0],
        },
        Vertex {
            position: [0.5, 0.5, 0.5],
            normal: [0.0, 0.0, 1.0],
        },
        Vertex {
            position: [-0.5, 0.5, 0.5],
            normal: [0.0, 0.0, 1.0],
        },
        // Back face
        Vertex {
            position: [-0.5, -0.5, -0.5],
            normal: [0.0, 0.0, -1.0],
        },
        Vertex {
            position: [0.5, -0.5, -0.5],
            normal: [0.0, 0.0, -1.0],
        },
        Vertex {
            position: [0.5, 0.5, -0.5],
            normal: [0.0, 0.0, -1.0],
        },
        Vertex {
            position: [-0.5, 0.5, -0.5],
            normal: [0.0, 0.0, -1.0],
        },
    ]
}

fn create_cube_indices(start_index: u16) -> Vec<u16> {
    vec![
        0, 1, 1, 2, 2, 3, 3, 0, // Front face
        4, 5, 5, 6, 6, 7, 7, 4, // Back face
        0, 4, 1, 5, 2, 6, 3, 7, // Connecting edges
    ]
    .into_iter()
    .map(|i| i + start_index)
    .collect()
}

fn create_sphere_vertices(longitude_lines: u32, latitude_lines: u32) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    for lat in 0..=latitude_lines {
        let theta = lat as f32 * PI / latitude_lines as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=longitude_lines {
            let phi = lon as f32 * 2.0 * PI / longitude_lines as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = cos_phi * sin_theta;
            let y = cos_theta;
            let z = sin_phi * sin_theta;

            vertices.push(Vertex {
                position: [x * 0.5, y * 0.5, z * 0.5],
                normal: [x, y, z],
            });
        }
    }
    vertices
}

fn create_sphere_indices(longitude_lines: u32, latitude_lines: u32, start_index: u16) -> Vec<u16> {
    let mut indices = Vec::new();
    for lat in 0..latitude_lines {
        for lon in 0..longitude_lines {
            let first = lat * (longitude_lines + 1) + lon;
            let second = first + longitude_lines + 1;

            indices.push(first as u16 + start_index);
            indices.push(second as u16 + start_index);

            if lon != longitude_lines - 1 {
                indices.push((first + 1) as u16 + start_index);
                indices.push((second + 1) as u16 + start_index);
            }
        }
    }
    indices
}

fn create_capsule_vertices(longitude_lines: u32, latitude_lines: u32) -> Vec<Vertex> {
    let mut vertices = Vec::new();

    // Top hemisphere
    for lat in 0..=latitude_lines / 2 {
        let theta = lat as f32 * PI / latitude_lines as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=longitude_lines {
            let phi = lon as f32 * 2.0 * PI / longitude_lines as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = cos_phi * sin_theta;
            let y = cos_theta;
            let z = sin_phi * sin_theta;

            vertices.push(Vertex {
                position: [x * 0.5, y * 0.5 + 0.5, z * 0.5],
                normal: [x, y, z],
            });
        }
    }

    // Cylinder body
    for lat in 0..=1 {
        let y = lat as f32 - 0.5;
        for lon in 0..=longitude_lines {
            let phi = lon as f32 * 2.0 * PI / longitude_lines as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            vertices.push(Vertex {
                position: [cos_phi * 0.5, y, sin_phi * 0.5],
                normal: [cos_phi, 0.0, sin_phi],
            });
        }
    }

    // Bottom hemisphere
    for lat in latitude_lines / 2..=latitude_lines {
        let theta = lat as f32 * PI / latitude_lines as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=longitude_lines {
            let phi = lon as f32 * 2.0 * PI / longitude_lines as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = cos_phi * sin_theta;
            let y = cos_theta;
            let z = sin_phi * sin_theta;

            vertices.push(Vertex {
                position: [x * 0.5, y * 0.5 - 0.5, z * 0.5],
                normal: [x, y, z],
            });
        }
    }

    vertices
}

fn create_capsule_indices(longitude_lines: u32, latitude_lines: u32, start_index: u16) -> Vec<u16> {
    let mut indices = Vec::new();
    let half_latitude = latitude_lines / 2;

    // Top hemisphere
    for lat in 0..half_latitude {
        for lon in 0..longitude_lines {
            let first = lat * (longitude_lines + 1) + lon;
            let second = first + longitude_lines + 1;

            indices.push(first as u16 + start_index);
            indices.push(second as u16 + start_index);

            if lon != longitude_lines - 1 {
                indices.push((first + 1) as u16 + start_index);
                indices.push((second + 1) as u16 + start_index);
            }
        }
    }

    // Cylinder body
    let cylinder_start = (half_latitude + 1) * (longitude_lines + 1);
    for lon in 0..longitude_lines {
        indices.push((cylinder_start + lon) as u16 + start_index);
        indices.push((cylinder_start + longitude_lines + 1 + lon) as u16 + start_index);
    }

    // Bottom hemisphere
    let bottom_start = cylinder_start + 2 * (longitude_lines + 1);
    for lat in 0..half_latitude {
        for lon in 0..longitude_lines {
            let first = bottom_start + lat * (longitude_lines + 1) + lon;
            let second = first + longitude_lines + 1;

            indices.push(first as u16 + start_index);
            indices.push(second as u16 + start_index);

            if lon != longitude_lines - 1 {
                indices.push((first + 1) as u16 + start_index);
                indices.push((second + 1) as u16 + start_index);
            }
        }
    }

    indices
}

fn create_shape_pipeline(
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
                buffers: &[Vertex::desc(), ShapeInstance::desc()],
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

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

impl ShapeInstance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ShapeInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniform {
    pub view: Mat4,
    pub projection: Mat4,
    pub camera_position: Vec4,
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
    @location(1) normal: vec3<f32>,
};

struct InstanceInput {
    @location(2) model_matrix_0: vec4<f32>,
    @location(3) model_matrix_1: vec4<f32>,
    @location(4) model_matrix_2: vec4<f32>,
    @location(5) model_matrix_3: vec4<f32>,
    @location(6) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var out: VertexOutput;
    out.clip_position = ubo.projection * ubo.view * model_matrix * vec4<f32>(vertex.position, 1.0);
    out.color = instance.color;
    return out;
}

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;
