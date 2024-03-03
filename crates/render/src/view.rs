/// webgpu allows four bind groups, so we'll use:
///
/// - a uniform buffer for camera, time, and other global stuff
/// - a dynamic uniform buffer for object indices
/// - material bind group, which makes textures and material properties available to the shader
/// - a large flat ssbo for all objects, which are structs containing transforms and other per-instance properties
pub struct WorldRender {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,

    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,

    pub dynamic_uniform_buffer: wgpu::Buffer,
    pub dynamic_uniform_bind_group: wgpu::BindGroup,

    pub object_buffer: wgpu::Buffer,
    pub object_buffer_bind_group: wgpu::BindGroup,

    pub material_bind_groups: Vec<wgpu::BindGroup>,

    pub triangle_filled_pipeline: wgpu::RenderPipeline,
    pub triangle_blended_pipeline: wgpu::RenderPipeline,
    pub line_pipeline: wgpu::RenderPipeline,
    pub line_strip_pipeline: wgpu::RenderPipeline,
    pub triangle_strip_pipeline: wgpu::RenderPipeline,
}

impl WorldRender {
    pub fn new(gpu: &crate::gpu::Gpu, asset: &asset::Asset) -> Self {
        let (vertex_buffer, index_buffer) =
            create_geometry_buffers(&gpu.device, &asset.vertices, &asset.indices);

        let mut instances = asset
            .instances
            .iter()
            .map(|instance| instance.transform.matrix())
            .collect::<Vec<_>>();
        if instances.is_empty() {
            instances.push(nalgebra_glm::Mat4::identity());
        }
        let instance_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &gpu.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("instance_buffer"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );

        let (uniform_buffer, uniform_bind_group_layout, uniform_bind_group) = create_uniform(gpu);
        let (dynamic_uniform_buffer, dynamic_uniform_bind_group_layout, dynamic_uniform_bind_group) =
            create_dynamic_uniform(gpu, asset.transforms.len() as _);

        let samplers = asset
            .samplers
            .iter()
            .map(|sampler| {
                gpu.device.create_sampler(&wgpu::SamplerDescriptor {
                    address_mode_u: match sampler.wrap_s {
                        asset::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
                        asset::WrappingMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
                        asset::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
                    },
                    address_mode_v: match sampler.wrap_t {
                        asset::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
                        asset::WrappingMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
                        asset::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
                    },
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    mag_filter: match sampler.mag_filter {
                        asset::MagFilter::Nearest => wgpu::FilterMode::Nearest,
                        asset::MagFilter::Linear => wgpu::FilterMode::Linear,
                    },
                    min_filter: match sampler.min_filter {
                        asset::MinFilter::Nearest
                        | asset::MinFilter::NearestMipmapLinear
                        | asset::MinFilter::NearestMipmapNearest => wgpu::FilterMode::Nearest,
                        asset::MinFilter::Linear
                        | asset::MinFilter::LinearMipmapLinear
                        | asset::MinFilter::LinearMipmapNearest => wgpu::FilterMode::Linear,
                    },
                    mipmap_filter: match sampler.min_filter {
                        asset::MinFilter::Nearest
                        | asset::MinFilter::NearestMipmapLinear
                        | asset::MinFilter::NearestMipmapNearest => wgpu::FilterMode::Nearest,
                        asset::MinFilter::Linear
                        | asset::MinFilter::LinearMipmapLinear
                        | asset::MinFilter::LinearMipmapNearest => wgpu::FilterMode::Linear,
                    },
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();

        let textures = asset
            .textures
            .iter()
            .map(|texture| {
                let image = &asset.images[texture.image_index];
                create_texture(gpu, image)
            })
            .collect::<Vec<_>>();

        let texture_views = textures
            .iter()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()))
            .collect::<Vec<_>>();

        let material_bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("material_bind_group_layout"),
                    entries: &[
                        // Material properties
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Base color texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        // Base color sampler
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let material_bind_groups = asset
            .materials
            .iter()
            .map(|material| {
                let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("material_buffer"),
                    size: std::mem::size_of::<Material>() as wgpu::BufferAddress,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                gpu.queue.write_buffer(
                    &buffer,
                    0,
                    bytemuck::cast_slice(&[Material {
                        base_color: material.base_color_factor,
                        alpha_mode: material.alpha_mode as _,
                        alpha_cutoff: material.alpha_cutoff.unwrap_or(0.5),
                        ..Default::default()
                    }]),
                );

                gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &material_bind_group_layout,
                    entries: &[
                        // Material properties
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &buffer,
                                offset: 0,
                                size: None,
                            }),
                        },
                        // Base color texture
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(
                                &texture_views[material.base_color_texture_index],
                            ),
                        },
                        // Base color sampler
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(
                                &samplers[asset.textures[material.base_color_texture_index]
                                    .sampler_index
                                    .unwrap()],
                            ),
                        },
                    ],
                    label: Some("material_bind_group"),
                })
            })
            .collect::<Vec<_>>();

        let object_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("object_buffer"),
            size: (std::mem::size_of::<nalgebra_glm::Mat4>() * asset.transforms.len())
                as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let object_buffer_bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: None,
                });
        let object_buffer_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &object_buffer_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: object_buffer.as_entire_binding(),
            }],
            label: None,
        });

        let bind_group_layouts = &[
            &uniform_bind_group_layout,
            &dynamic_uniform_bind_group_layout,
            &material_bind_group_layout,
            &object_buffer_bind_group_layout,
        ];

        let line_pipeline = create_pipeline(
            gpu,
            bind_group_layouts,
            false,
            wgpu::PrimitiveTopology::LineList,
            wgpu::PolygonMode::Fill,
        );

        let line_strip_pipeline = create_pipeline(
            gpu,
            bind_group_layouts,
            false,
            wgpu::PrimitiveTopology::LineStrip,
            wgpu::PolygonMode::Fill,
        );

        let triangle_filled_pipeline = create_pipeline(
            gpu,
            bind_group_layouts,
            false,
            wgpu::PrimitiveTopology::TriangleList,
            wgpu::PolygonMode::Fill,
        );

        let triangle_blended_pipeline = create_pipeline(
            gpu,
            bind_group_layouts,
            true,
            wgpu::PrimitiveTopology::TriangleList,
            wgpu::PolygonMode::Fill,
        );

        let triangle_strip_pipeline = create_pipeline(
            gpu,
            bind_group_layouts,
            true,
            wgpu::PrimitiveTopology::TriangleStrip,
            wgpu::PolygonMode::Fill,
        );

        Self {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            uniform_buffer,
            uniform_bind_group,
            material_bind_groups,
            object_buffer,
            object_buffer_bind_group,
            dynamic_uniform_bind_group,
            dynamic_uniform_buffer,
            triangle_filled_pipeline,
            triangle_blended_pipeline,
            line_pipeline,
            line_strip_pipeline,
            triangle_strip_pipeline,
        }
    }

    pub fn render<'rp>(
        &'rp mut self,
        render_pass: &mut wgpu::RenderPass<'rp>,
        gpu: &crate::gpu::Gpu,
        asset: &asset::Asset,
    ) {
        let Some(scene_index) = asset.default_scene_index else {
            return;
        };
        let scene = &asset.scenes[scene_index];

        let (camera_position, projection, view) =
            asset::create_camera_matrices(asset, scene, gpu.aspect_ratio());

        gpu.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[Uniform {
                view,
                projection,
                camera_position: nalgebra_glm::vec3_to_vec4(&camera_position),
            }]),
        );

        let mut mesh_ubos = vec![DynamicUniform::default(); asset.transforms.len()];
        scene
            .graph
            .node_indices()
            .enumerate()
            .for_each(|(index, graph_node_index)| {
                mesh_ubos[index] = DynamicUniform {
                    object_index: index as u32,
                    number_of_instances: asset.nodes[scene.graph[graph_node_index]].instances.len()
                        as _,
                    ..Default::default()
                };
            });
        gpu.queue
            .write_buffer(&self.dynamic_uniform_buffer, 0, unsafe {
                std::slice::from_raw_parts(
                    mesh_ubos.as_ptr() as *const u8,
                    mesh_ubos.len() * gpu.alignment() as usize,
                )
            });

        for (ubo_index, graph_node_index) in scene.graph.node_indices().enumerate() {
            let transform = asset.global_transform(&scene.graph, graph_node_index);
            let offset =
                (ubo_index * std::mem::size_of::<nalgebra_glm::Mat4>()) as wgpu::BufferAddress;
            gpu.queue.write_buffer(
                &self.object_buffer,
                offset,
                bytemuck::cast_slice(&[transform]),
            );
        }

        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_bind_group(3, &self.object_buffer_bind_group, &[]);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // TODO: Refactor this to first sort by material, then sort materials by alpha mode and render Opaque, Mask, then Blend
        for alpha_mode in [
            asset::AlphaMode::Opaque,
            asset::AlphaMode::Mask,
            asset::AlphaMode::Blend,
        ]
        .iter()
        {
            scene
                .graph
                .node_indices()
                .enumerate()
                .for_each(|(ubo_index, graph_node_index)| {
                    let node_index = scene.graph[graph_node_index];
                    let node = &asset.nodes[node_index];
                    if let Some(mesh_index) = node.mesh_index {
                        let offset = (ubo_index as u64 * gpu.alignment()) as wgpu::DynamicOffset;
                        render_pass.set_bind_group(1, &self.dynamic_uniform_bind_group, &[offset]);

                        let mesh = &asset.meshes[mesh_index];

                        for primitive in mesh.primitives.iter() {
                            match primitive.topology {
                                asset::PrimitiveTopology::Lines => {
                                    render_pass.set_pipeline(&self.line_pipeline);
                                }
                                asset::PrimitiveTopology::LineStrip => {
                                    render_pass.set_pipeline(&self.line_strip_pipeline);
                                }
                                asset::PrimitiveTopology::Triangles => match alpha_mode {
                                    asset::AlphaMode::Opaque | asset::AlphaMode::Mask => {
                                        render_pass.set_pipeline(&self.triangle_filled_pipeline);
                                    }
                                    asset::AlphaMode::Blend => {
                                        render_pass.set_pipeline(&self.triangle_blended_pipeline);
                                    }
                                },
                                asset::PrimitiveTopology::TriangleStrip => {
                                    render_pass.set_pipeline(&self.triangle_strip_pipeline);
                                }

                                // wgpu does not support line loops or triangle fans
                                // and Point primitive topology is unsupported on Metal so it is omitted here
                                _ => continue,
                            }

                            render_pass.set_bind_group(
                                2,
                                &self.material_bind_groups[primitive.material_index.unwrap_or(0)],
                                &[],
                            );

                            let instance_range = if node.instances.is_empty() {
                                0..1
                            } else {
                                0..(node.instances.len() as u32)
                            };

                            if primitive.number_of_indices > 0 {
                                let index_offset = primitive.index_offset as u32;
                                let number_of_indices =
                                    index_offset + primitive.number_of_indices as u32;
                                render_pass.draw_indexed(
                                    index_offset..number_of_indices,
                                    primitive.vertex_offset as i32,
                                    instance_range,
                                );
                            } else {
                                let vertex_offset = primitive.vertex_offset as u32;
                                let number_of_vertices =
                                    vertex_offset + primitive.number_of_vertices as u32;
                                render_pass.draw(vertex_offset..number_of_vertices, instance_range);
                            }
                        }
                    }
                });
        }
    }
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
    vertices: &[asset::Vertex],
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
pub struct Uniform {
    pub view: nalgebra_glm::Mat4,
    pub projection: nalgebra_glm::Mat4,
    pub camera_position: nalgebra_glm::Vec4,
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
                    vertex_description(&vertex_attributes()),
                    instance_description(&instance_attributes()),
                ],
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
                    format: wgpu::TextureFormat::Rgba16Float,
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

pub fn vertex_attributes() -> Vec<wgpu::VertexAttribute> {
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

pub fn vertex_description(attributes: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<asset::Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes,
    }
}

pub fn instance_attributes() -> Vec<wgpu::VertexAttribute> {
    wgpu::vertex_attr_array![7 => Float32x4, 8 => Float32x4, 9 => Float32x4, 10 => Float32x4]
        .to_vec()
}

pub fn instance_description(attributes: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<nalgebra_glm::Mat4>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes,
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Material {
    pub base_color: nalgebra_glm::Vec4,
    pub alpha_mode: i32,
    pub alpha_cutoff: f32,
    pub padding: nalgebra_glm::Vec2,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            base_color: nalgebra_glm::vec4(0.0, 1.0, 0.0, 1.0),
            alpha_mode: 0,
            alpha_cutoff: 0.5,
            padding: nalgebra_glm::vec2(0.0, 0.0),
        }
    }
}

pub fn create_texture(gpu: &crate::gpu::Gpu, image: &asset::Image) -> wgpu::Texture {
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

#[repr(C, align(256))]
#[derive(Default, Copy, Clone, Debug, bytemuck::Zeroable)]
pub struct DynamicUniform {
    pub object_index: u32,
    pub number_of_instances: u32,
    pub padding: nalgebra_glm::Vec2,
}

const SHADER_SOURCE: &str = "
struct Uniform {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    camera_position: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> ubo: Uniform;

struct DynamicUniform {
    object_index: u32,
    number_of_instances: u32,
};

@group(1) @binding(0)
var<uniform> mesh_ubo: DynamicUniform;

struct Material {
    base_color: vec4<f32>,
    alpha_mode: i32,
    alpha_cutoff: f32,
}

@group(2) @binding(0)
var<uniform> material: Material;

@group(2) @binding(1)
var base_color_texture: texture_2d<f32>;

@group(2) @binding(2)
var base_color_sampler: sampler;

struct Object {
    matrix: mat4x4<f32>,
}

@group(3) @binding(0)
var<storage, read> objects: array<Object>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv_0: vec2<f32>,
    @location(3) uv_1: vec2<f32>,
    @location(4) joint_0: vec4<f32>,
    @location(5) weight_0: vec4<f32>,
    @location(6) color_0: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

struct InstanceInput {
    @location(7) model_matrix_0: vec4<f32>,
    @location(8) model_matrix_1: vec4<f32>,
    @location(9) model_matrix_2: vec4<f32>,
    @location(10) model_matrix_3: vec4<f32>,
}

@vertex
fn vertex_main(vert: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    var mvp = ubo.projection * ubo.view * objects[mesh_ubo.object_index].matrix; 
    if mesh_ubo.number_of_instances > 0 {
        let model_matrix = mat4x4<f32>(
            instance.model_matrix_0,
            instance.model_matrix_1,
            instance.model_matrix_2,
            instance.model_matrix_3,
        );
        mvp *=  model_matrix;
    }
    out.position = mvp * vec4(vert.position, 1.0);
    out.normal = vec4((mvp * vec4(vert.normal, 0.0)).xyz, 1.0).xyz;
    out.color = vert.color_0;
    out.tex_coord = vert.uv_0;
    return out;
};

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var base_color = material.base_color * textureSampleLevel(base_color_texture, base_color_sampler, in.tex_coord, 0.0);

    let light_position = vec3<f32>(2.0, 2.0, 2.0);
    let light_color = vec3<f32>(1.0, 1.0, 1.0);

    let ambient_strength = 0.1;
    let ambient_color = light_color * ambient_strength;
    let light_dir = normalize(light_position - in.position.xyz);
    let diffuse_strength =  max(dot(in.normal, light_dir), 0.0);
    let diffuse_color = light_color * diffuse_strength;
    let result = (ambient_color + diffuse_color) * base_color.rgb * in.color;

    return vec4<f32>(result.xyz, 1.0);
}
";
