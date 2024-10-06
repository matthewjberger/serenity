use nalgebra_glm as glm;
use wgpu::util::DeviceExt;

const TEXTURE_SIZE: u32 = 256;
const CHECKER_SIZE: u32 = 32;

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniform {
    pub proj: glm::Mat4,
    pub proj_inv: glm::Mat4,
    pub view: glm::Mat4,
    pub cam_pos: glm::Vec4,
}

pub struct SkyRender {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
    uniform: Uniform,
}

impl SkyRender {
    pub fn new(gpu: &crate::gpu::Gpu) -> Self {
        let device = &gpu.device;
        let queue = &gpu.queue;

        // Create checkerboard cubemap texture
        let texture = create_checkerboard_cubemap(device, queue);

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Create uniform buffer
        let uniform = Uniform::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Skybox Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout and pipeline layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("sky_bind_group_layout"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sky"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sky Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sky"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_sky",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_sky",
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("sky_bind_group"),
        });

        Self {
            pipeline,
            bind_group,
            _texture: texture,
            _view: view,
            _sampler: sampler,
            uniform_buffer,
            uniform,
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

            // Update uniform with camera data from context
            self.uniform.proj = projection;
            self.uniform.proj_inv = nalgebra_glm::inverse(&projection);
            self.uniform.view = view;
            self.uniform.cam_pos =
                nalgebra_glm::vec4(camera_position.x, camera_position.y, camera_position.z, 1.0);

            // Write updated uniform to the GPU
            gpu.queue.write_buffer(
                &self.uniform_buffer,
                0,
                bytemuck::cast_slice(&[self.uniform]),
            );

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}

fn create_checkerboard_face(color1: [u8; 4], color2: [u8; 4]) -> Vec<u8> {
    let mut data = Vec::with_capacity((TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize);
    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let color = if ((x / CHECKER_SIZE) + (y / CHECKER_SIZE)) % 2 == 0 {
                color1
            } else {
                color2
            };
            data.extend_from_slice(&color);
        }
    }
    data
}

fn create_checkerboard_cubemap(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    let faces = [
        create_checkerboard_face([255, 0, 0, 255], [128, 0, 0, 255]), // Right face (Red)
        create_checkerboard_face([0, 255, 0, 255], [0, 128, 0, 255]), // Left face (Green)
        create_checkerboard_face([0, 0, 255, 255], [0, 0, 128, 255]), // Top face (Blue)
        create_checkerboard_face([255, 255, 0, 255], [128, 128, 0, 255]), // Bottom face (Yellow)
        create_checkerboard_face([0, 255, 255, 255], [0, 128, 128, 255]), // Front face (Cyan)
        create_checkerboard_face([255, 0, 255, 255], [128, 0, 128, 255]), // Back face (Magenta)
    ];

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: TEXTURE_SIZE,
            height: TEXTURE_SIZE,
            depth_or_array_layers: 6,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("checkerboard_cubemap"),
        view_formats: &[],
    });

    for (i, face) in faces.iter().enumerate() {
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: i as u32,
                },
                aspect: wgpu::TextureAspect::All,
            },
            face,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(TEXTURE_SIZE * 4),
                rows_per_image: Some(TEXTURE_SIZE),
            },
            wgpu::Extent3d {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
        );
    }

    texture
}

const SHADER_SOURCE: &str = r#"
struct Uniform {
    proj: mat4x4<f32>,
    proj_inv: mat4x4<f32>,
    view: mat4x4<f32>,
    cam_pos: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniform;

@group(0) @binding(1)
var t_diffuse: texture_cube<f32>;

@group(0) @binding(2)
var s_diffuse: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec3<f32>,
};

@vertex
fn vs_sky(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let tmp1 = i32(vertex_index) / 2;
    let tmp2 = i32(vertex_index) & 1;
    let pos = vec4<f32>(
        f32(tmp1) * 4.0 - 1.0,
        f32(tmp2) * 4.0 - 1.0,
        1.0,
        1.0
    );

    // transposition = inversion for this orthonormal matrix
    let inv_model_view = transpose(mat3x3<f32>(u.view[0].xyz, u.view[1].xyz, u.view[2].xyz));
    let unprojected = u.proj_inv * pos;

    var result: VertexOutput;
    result.uv = inv_model_view * unprojected.xyz;
    result.position = pos;
    return result;
}

@fragment
fn fs_sky(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, normalize(in.uv));
}
"#;
