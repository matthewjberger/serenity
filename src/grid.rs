use nalgebra_glm as glm;
use wgpu::util::DeviceExt;

pub struct GridRender {
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub line_pipeline: wgpu::RenderPipeline,
    pub grid_vertices: wgpu::Buffer,
    pub grid_instances: wgpu::Buffer,
}

impl GridRender {
    pub fn new(gpu: &crate::gpu::Gpu) -> Self {
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

        let line_pipeline = create_line_pipeline(gpu, &uniform_bind_group_layout);

        let (grid_vertices, grid_instances) = create_grid_buffers(gpu);

        Self {
            uniform_buffer,
            uniform_bind_group,
            line_pipeline,
            grid_vertices,
            grid_instances,
        }
    }

    pub fn sync_context(&mut self, context: &crate::app::Context, gpu: &crate::gpu::Gpu) {
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
        }
    }
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniform {
    pub view: nalgebra_glm::Mat4,
    pub projection: nalgebra_glm::Mat4,
    pub camera_position: nalgebra_glm::Vec4,
}

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

    // X-axis (red)
    instances.push(LineInstance {
        start: glm::vec3(-half_size, 0.0, 0.0),
        end: glm::vec3(half_size, 0.0, 0.0),
        color: glm::vec4(1.0, 0.0, 0.0, 1.0),
    });

    // Y-axis (green)
    instances.push(LineInstance {
        start: glm::vec3(0.0, -half_size, 0.0),
        end: glm::vec3(0.0, half_size, 0.0),
        color: glm::vec4(0.0, 1.0, 0.0, 1.0),
    });

    // Z-axis (blue)
    instances.push(LineInstance {
        start: glm::vec3(0.0, 0.0, -half_size),
        end: glm::vec3(0.0, 0.0, half_size),
        color: glm::vec4(0.0, 0.0, 1.0, 1.0),
    });

    // Create grid lines
    for i in 0..=GRID_SIZE {
        let pos = i as f32 * GRID_STEP - half_size;
        if pos != 0.0 {
            // Skip the center lines as they're covered by the axes
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
