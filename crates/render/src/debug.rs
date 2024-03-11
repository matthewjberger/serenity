pub struct DebugRender {
    pub vertex_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub cube_pipeline: wgpu::RenderPipeline,
    pub number_of_instances: u32,
}

impl DebugRender {
    pub fn new(gpu: &crate::gpu::Gpu) -> Self {
        let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &gpu.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Line Vertex Buffer"),
                contents: bytemuck::cast_slice(&VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );

        let instance_bindings = vec![InstanceBinding {
            model: nalgebra_glm::Mat4::identity(),
            color: nalgebra_glm::vec4(1.0, 1.0, 1.0, 1.0),
        }];

        let instance_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &gpu.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("instance_buffer"),
                contents: bytemuck::cast_slice(&instance_bindings),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
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

        Self {
            vertex_buffer,
            instance_buffer,
            number_of_instances: 1,
            uniform_buffer,
            uniform_bind_group,
            cube_pipeline,
        }
    }

    pub fn sync_camera(
        &mut self,
        gpu: &crate::gpu::Gpu,
        projection: nalgebra_glm::Mat4,
        view: nalgebra_glm::Mat4,
        camera_position: nalgebra_glm::Vec3,
    ) {
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

    pub fn render<'rp>(&'rp mut self, render_pass: &mut wgpu::RenderPass<'rp>) {
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_pipeline(&self.cube_pipeline);
        render_pass.draw(0..24, 0..self.number_of_instances);
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
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
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
                    format: wgpu::TextureFormat::Rgba16Float,
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
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4].to_vec()
    }

    pub fn description(attributes: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes,
        }
    }
}
const VERTICES: [Vertex; 24] = [
    Vertex {
        position: nalgebra_glm::Vec3::new(-0.5, -0.5, -0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(0.5, -0.5, -0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(0.5, -0.5, -0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(0.5, 0.5, -0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(0.5, 0.5, -0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(-0.5, 0.5, -0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(-0.5, 0.5, -0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(-0.5, -0.5, -0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(-0.5, -0.5, 0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(0.5, -0.5, 0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(0.5, -0.5, 0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(0.5, 0.5, 0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(0.5, 0.5, 0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(-0.5, 0.5, 0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(-0.5, 0.5, 0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(-0.5, -0.5, 0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(-0.5, -0.5, -0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(-0.5, -0.5, 0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(0.5, -0.5, -0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(0.5, -0.5, 0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(0.5, 0.5, -0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(0.5, 0.5, 0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(-0.5, 0.5, -0.5),
    },
    Vertex {
        position: nalgebra_glm::Vec3::new(-0.5, 0.5, 0.5),
    },
];

#[repr(C)]
#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
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
