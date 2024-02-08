pub struct WorldRender {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub dynamic_uniform_buffer: wgpu::Buffer,
    pub dynamic_uniform_bind_group: wgpu::BindGroup,
    pub texture_array_bind_group: wgpu::BindGroup,
    pub samplers: Vec<wgpu::Sampler>,
    pub textures: Vec<wgpu::Texture>,
    pub triangle_filled_pipeline: wgpu::RenderPipeline,
    pub triangle_blended_pipeline: wgpu::RenderPipeline,
    pub line_pipeline: wgpu::RenderPipeline,
    pub line_strip_pipeline: wgpu::RenderPipeline,
    pub triangle_strip_pipeline: wgpu::RenderPipeline,
}

impl WorldRender {
    pub fn new(gpu: &crate::gpu::Gpu, world: &crate::world::World) -> Self {
        let (vertex_buffer, index_buffer) =
            create_geometry_buffers(&gpu.device, &world.vertices, &world.indices);
        let (uniform_buffer, uniform_bind_group_layout, uniform_bind_group) = create_uniform(gpu);
        let (dynamic_uniform_buffer, dynamic_uniform_bind_group_layout, dynamic_uniform_bind_group) =
            create_dynamic_uniform(gpu, world.transforms.len() as _);

        let mut samplers = world
            .samplers
            .iter()
            .map(|sampler| {
                gpu.device.create_sampler(&wgpu::SamplerDescriptor {
                    address_mode_u: match sampler.wrap_s {
                        crate::world::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
                        crate::world::WrappingMode::MirroredRepeat => {
                            wgpu::AddressMode::MirrorRepeat
                        }
                        crate::world::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
                    },
                    address_mode_v: match sampler.wrap_t {
                        crate::world::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
                        crate::world::WrappingMode::MirroredRepeat => {
                            wgpu::AddressMode::MirrorRepeat
                        }
                        crate::world::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
                    },
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    mag_filter: match sampler.mag_filter {
                        crate::world::MagFilter::Nearest => wgpu::FilterMode::Nearest,
                        crate::world::MagFilter::Linear => wgpu::FilterMode::Linear,
                    },
                    min_filter: match sampler.min_filter {
                        crate::world::MinFilter::Nearest
                        | crate::world::MinFilter::NearestMipmapLinear
                        | crate::world::MinFilter::NearestMipmapNearest => {
                            wgpu::FilterMode::Nearest
                        }
                        crate::world::MinFilter::Linear
                        | crate::world::MinFilter::LinearMipmapLinear
                        | crate::world::MinFilter::LinearMipmapNearest => wgpu::FilterMode::Linear,
                    },
                    mipmap_filter: match sampler.min_filter {
                        crate::world::MinFilter::Nearest
                        | crate::world::MinFilter::NearestMipmapLinear
                        | crate::world::MinFilter::NearestMipmapNearest => {
                            wgpu::FilterMode::Nearest
                        }
                        crate::world::MinFilter::Linear
                        | crate::world::MinFilter::LinearMipmapLinear
                        | crate::world::MinFilter::LinearMipmapNearest => wgpu::FilterMode::Linear,
                    },
                    compare: Some(wgpu::CompareFunction::LessEqual),
                    lod_min_clamp: 0.0,
                    lod_max_clamp: 100.0,
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();

        let mut textures = world
            .textures
            .iter()
            .map(|texture| {
                let image = &world.images[texture.image_index];
                let size = wgpu::Extent3d {
                    width: image.width,
                    height: image.height,
                    depth_or_array_layers: 1,
                };
                let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    // TODO: map these formats
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                gpu.queue.write_texture(
                    wgpu::ImageCopyTexture {
                        aspect: wgpu::TextureAspect::All,
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                    },
                    &image.pixels,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(image.width * 4),
                        rows_per_image: Some(image.height),
                    },
                    size,
                );
                texture
            })
            .collect::<Vec<_>>();

        if textures.is_empty() {
            let image = crate::world::Image {
                width: 1,
                height: 1,
                pixels: vec![0x00, 0xFF, 0xFF, 0x00],
                format: crate::world::ImageFormat::R8G8B8A8,
            };
            let size = wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            };
            let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                // TODO: map these formats
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            gpu.queue.write_texture(
                wgpu::ImageCopyTexture {
                    aspect: wgpu::TextureAspect::All,
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                &image.pixels,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(image.width * 4),
                    rows_per_image: Some(image.height),
                },
                size,
            );
            textures.push(texture);
        }

        if samplers.is_empty() {
            samplers.push(
                gpu.device
                    .create_sampler(&wgpu::SamplerDescriptor::default()),
            );
        }

        let (texture_array_bind_group, texture_array_bind_group_layout) = {
            let texture_array_bind_group_layout =
                gpu.device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("bind group layout"),
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    multisampled: false,
                                },
                                count: std::num::NonZeroU32::new(textures.len() as _),
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: std::num::NonZeroU32::new(samplers.len() as _),
                            },
                        ],
                    });
            let texture_views = textures
                .iter()
                .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()))
                .collect::<Vec<_>>();
            let texture_array_bind_group =
                gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureViewArray(
                                &texture_views.iter().collect::<Vec<_>>(),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::SamplerArray(
                                &samplers.iter().collect::<Vec<_>>(),
                            ),
                        },
                    ],
                    layout: &texture_array_bind_group_layout,
                    label: Some("texture array bind group"),
                });

            (texture_array_bind_group, texture_array_bind_group_layout)
        };

        let line_pipeline = create_pipeline(
            gpu,
            &[
                &uniform_bind_group_layout,
                &dynamic_uniform_bind_group_layout,
                &texture_array_bind_group_layout,
            ],
            false,
            wgpu::PrimitiveTopology::LineList,
            wgpu::PolygonMode::Fill,
        );

        let line_strip_pipeline = create_pipeline(
            gpu,
            &[
                &uniform_bind_group_layout,
                &dynamic_uniform_bind_group_layout,
                &texture_array_bind_group_layout,
            ],
            false,
            wgpu::PrimitiveTopology::LineStrip,
            wgpu::PolygonMode::Fill,
        );

        let triangle_filled_pipeline = create_pipeline(
            gpu,
            &[
                &uniform_bind_group_layout,
                &dynamic_uniform_bind_group_layout,
                &texture_array_bind_group_layout,
            ],
            false,
            wgpu::PrimitiveTopology::TriangleList,
            wgpu::PolygonMode::Fill,
        );

        let triangle_blended_pipeline = create_pipeline(
            gpu,
            &[
                &uniform_bind_group_layout,
                &dynamic_uniform_bind_group_layout,
                &texture_array_bind_group_layout,
            ],
            true,
            wgpu::PrimitiveTopology::TriangleList,
            wgpu::PolygonMode::Fill,
        );

        let triangle_strip_pipeline = create_pipeline(
            gpu,
            &[
                &uniform_bind_group_layout,
                &dynamic_uniform_bind_group_layout,
                &texture_array_bind_group_layout,
            ],
            true,
            wgpu::PrimitiveTopology::TriangleStrip,
            wgpu::PolygonMode::Fill,
        );

        Self {
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group,
            dynamic_uniform_buffer,
            dynamic_uniform_bind_group,
            texture_array_bind_group,
            triangle_filled_pipeline,
            triangle_blended_pipeline,
            textures,
            samplers,
            line_pipeline,
            line_strip_pipeline,
            triangle_strip_pipeline,
        }
    }

    pub fn render<'rp>(
        &'rp mut self,
        render_pass: &mut wgpu::RenderPass<'rp>,
        gpu: &crate::gpu::Gpu,
        context: &crate::app::Context,
    ) {
        let scene_index = context.active_scene_index;
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

        let mut mesh_ubos = vec![DynamicUniform::default(); context.world.transforms.len()];
        scene
            .graph
            .node_indices()
            .enumerate()
            .for_each(|(ubo_index, graph_node_index)| {
                mesh_ubos[ubo_index] = DynamicUniform {
                    model: context
                        .world
                        .global_transform(&scene.graph, graph_node_index),
                };
            });
        gpu.queue
            .write_buffer(&self.dynamic_uniform_buffer, 0, unsafe {
                std::slice::from_raw_parts(
                    mesh_ubos.as_ptr() as *const u8,
                    mesh_ubos.len() * gpu.alignment() as usize,
                )
            });

        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_bind_group(2, &self.texture_array_bind_group, &[]);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        for alpha_mode in [
            crate::world::AlphaMode::Opaque,
            crate::world::AlphaMode::Mask,
            crate::world::AlphaMode::Blend,
        ]
        .iter()
        {
            scene
                .graph
                .node_indices()
                .enumerate()
                .for_each(|(ubo_index, graph_node_index)| {
                    let node_index = scene.graph[graph_node_index];
                    let node = &context.world.nodes[node_index];
                    if let Some(mesh_index) = node.mesh_index {
                        let offset = (ubo_index as u64 * gpu.alignment()) as wgpu::DynamicOffset;
                        render_pass.set_bind_group(1, &self.dynamic_uniform_bind_group, &[offset]);
                        let mesh = &context.world.meshes[mesh_index];

                        for primitive in mesh.primitives.iter() {
                            match primitive.topology {
                                crate::world::PrimitiveTopology::Lines => {
                                    render_pass.set_pipeline(&self.line_pipeline);
                                }
                                crate::world::PrimitiveTopology::LineStrip => {
                                    render_pass.set_pipeline(&self.line_strip_pipeline);
                                }
                                crate::world::PrimitiveTopology::Triangles => match alpha_mode {
                                    crate::world::AlphaMode::Opaque
                                    | crate::world::AlphaMode::Mask => {
                                        render_pass.set_pipeline(&self.triangle_filled_pipeline);
                                    }
                                    crate::world::AlphaMode::Blend => {
                                        render_pass.set_pipeline(&self.triangle_blended_pipeline);
                                    }
                                },
                                crate::world::PrimitiveTopology::TriangleStrip => {
                                    render_pass.set_pipeline(&self.triangle_strip_pipeline);
                                }

                                // wgpu does not support line loops or triangle fans
                                // and Point primitive topology is unsupported on Metal so it is omitted here
                                _ => continue,
                            }

                            let mut shader_material = Material::default();

                            match primitive.material_index {
                                Some(material_index) => {
                                    let material = &context.world.materials[material_index];
                                    if material.alpha_mode != *alpha_mode {
                                        continue;
                                    }
                                    shader_material.base_color = material.base_color_factor;
                                    shader_material.base_texture_index =
                                        material.base_color_texture_index as _;
                                    shader_material.alpha_mode = material.alpha_mode as _;
                                    shader_material.alpha_cutoff =
                                        material.alpha_cutoff.unwrap_or(0.5);
                                }
                                None => {
                                    shader_material.base_color =
                                        nalgebra_glm::vec4(0.5, 0.5, 0.5, 1.0);
                                    shader_material.base_texture_index = -1;
                                    shader_material.alpha_mode = 0;
                                    shader_material.alpha_cutoff = 0.5;
                                }
                            };

                            render_pass.set_push_constants(
                                wgpu::ShaderStages::VERTEX_FRAGMENT,
                                0,
                                bytemuck::cast_slice(&[shader_material]),
                            );

                            if primitive.number_of_indices > 0 {
                                let index_offset = primitive.index_offset as u32;
                                let number_of_indices =
                                    index_offset + primitive.number_of_indices as u32;
                                render_pass.draw_indexed(
                                    index_offset..number_of_indices,
                                    primitive.vertex_offset as i32,
                                    0..1, // TODO: support multiple instances per primitive
                                );
                            } else {
                                let vertex_offset = primitive.vertex_offset as u32;
                                let number_of_vertices =
                                    vertex_offset + primitive.number_of_vertices as u32;
                                render_pass.draw(
                                    vertex_offset..number_of_vertices,
                                    0..1, // TODO: support multiple instances per primitive
                                );
                            }
                        }
                    }
                });
        }
    }
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

fn create_geometry_buffers(
    device: &wgpu::Device,
    vertices: &[crate::world::Vertex],
    indices: &[u32],
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

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

fn create_pipeline(
    gpu: &crate::gpu::Gpu,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    blending_enabled: bool,
    topology: wgpu::PrimitiveTopology,
    polygon_mode: wgpu::PolygonMode,
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
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
                range: 0..32, // 1 byte
            }],
        });

    gpu.device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vertex_main",
                buffers: &[crate::world::Vertex::description(
                    &crate::world::Vertex::attributes(),
                )],
            },
            primitive: wgpu::PrimitiveState {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode,
                topology,
                strip_index_format: matches!(
                    topology,
                    wgpu::PrimitiveTopology::TriangleStrip | wgpu::PrimitiveTopology::LineStrip
                )
                .then(|| wgpu::IndexFormat::Uint32),
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
                    blend: if blending_enabled {
                        Some(wgpu::BlendState::ALPHA_BLENDING)
                    } else {
                        None
                    },
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        })
}

impl crate::world::Vertex {
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
            array_stride: std::mem::size_of::<crate::world::Vertex>() as wgpu::BufferAddress,
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

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Material {
    pub base_color: nalgebra_glm::Vec4,
    pub base_texture_index: i32,
    pub alpha_mode: i32,
    pub alpha_cutoff: f32,
    pub sampler_index: i32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            base_color: nalgebra_glm::vec4(0.0, 1.0, 0.0, 1.0),
            base_texture_index: -1,
            alpha_mode: 0,
            alpha_cutoff: 0.5,
            sampler_index: 0,
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

struct DynamicUniform {
    model: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> mesh_ubo: DynamicUniform;

@group(2) @binding(0)
var texture_array: binding_array<texture_2d<f32>>;
@group(2) @binding(1)
var sampler_array: binding_array<sampler>;

struct Material {
    base_color: vec4<f32>,
    base_texture_index: i32,
    alpha_mode: i32,
    alpha_cutoff: f32,
    sampler_index: i32,
}
var<push_constant> material: Material;

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
    @location(2) tex_coord: vec2<f32>,
};

@vertex
fn vertex_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let mvp = ubo.projection * ubo.view * mesh_ubo.model;
    out.position = mvp * vec4(vert.position, 1.0);
    out.normal = vec4((mvp * vec4(vert.normal, 0.0)).xyz, 1.0).xyz;
    out.color = vert.color_0;
    out.tex_coord = vert.uv_0;
    return out;
};

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var base_color = material.base_color;

    if material.base_texture_index > -1 {
        base_color *= textureSampleLevel(texture_array[material.base_texture_index], sampler_array[material.sampler_index], in.tex_coord, 0.0);
    } 

    if material.alpha_mode == 1 && base_color.a < material.alpha_cutoff {
        discard;
    }

    var color = base_color.rgb * in.color;

    return vec4(color, base_color.a);
}
";

impl From<wgpu::PrimitiveTopology> for crate::world::PrimitiveTopology {
    fn from(value: wgpu::PrimitiveTopology) -> Self {
        match value {
            wgpu::PrimitiveTopology::PointList => Self::Points,
            wgpu::PrimitiveTopology::LineList => Self::Lines,
            wgpu::PrimitiveTopology::LineStrip => Self::LineStrip,
            wgpu::PrimitiveTopology::TriangleList => Self::Triangles,
            wgpu::PrimitiveTopology::TriangleStrip => Self::TriangleStrip,
        }
    }
}
