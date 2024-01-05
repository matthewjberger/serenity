pub struct View {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub depth_texture_view: wgpu::TextureView,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub _uniform_bind_group_layout: wgpu::BindGroupLayout,
    pub pipeline: wgpu::RenderPipeline,
    pub meshes: std::collections::HashMap<String, Vec<crate::scene::PrimitiveDrawCommand>>,
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
            _uniform_bind_group_layout: uniform_bind_group_layout,
            pipeline,
        }
    }

    pub fn import_scene(&mut self, scene: &crate::scene::Scene, gpu: &crate::gpu::Gpu) {
        let (vertices, indices, meshes) = scene.flatten();
        let (vertex_buffer, index_buffer) = create_geometry_buffers(&gpu.device, vertices, indices);
        self.vertex_buffer = vertex_buffer;
        self.index_buffer = index_buffer;
        self.meshes = meshes;
    }

    pub fn resize(&mut self, gpu: &crate::gpu::Gpu, width: u32, height: u32) {
        self.depth_texture_view = gpu.create_depth_texture(width, height);
    }
}

pub fn create_camera_matrices(
    scene: &crate::scene::Scene,
    aspect_ratio: f32,
) -> Option<(nalgebra_glm::Mat4, nalgebra_glm::Mat4)> {
    let mut result = None;
    scene.walk_dfs(|node| {
        for component in node.components.iter() {
            if let crate::scene::NodeComponent::Camera(camera) = component {
                result = Some((camera.projection_matrix(aspect_ratio), {
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
                }));
            }
        }
    });
    result
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
pub struct UniformBuffer {
    pub mvp: nalgebra_glm::Mat4,
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
                polygon_mode: wgpu::PolygonMode::Line,
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
