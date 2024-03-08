pub struct SkyRender {
    pub pipeline: wgpu::RenderPipeline,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub environment_bind_group: wgpu::BindGroup,
}
impl SkyRender {
    pub fn new(gpu: &crate::gpu::Gpu) -> Self {
        let environment_bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("environment_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::Cube,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                            count: None,
                        },
                    ],
                });
        let hdri = crate::hdri::HdriLoader::new(&gpu);
        let (sky_texture, sky_texture_view, sky_texture_sampler) =
            hdri.convert_equirectangular_map_to_cubemap(&gpu, 1024);
        let cubemap_view = sky_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("cubemap_view"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            aspect: wgpu::TextureAspect::default(),
            base_array_layer: 0,
            array_layer_count: Some(6),
            ..Default::default()
        });
        let environment_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &environment_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&cubemap_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sky_texture_sampler),
                },
            ],
            label: Some("environment_bind_group"),
        });

        let uniform_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &gpu.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("sky_uniform_buffer"),
                contents: bytemuck::cast_slice(&[SkyUniform::default()]),
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
                    label: Some("sky_uniform_bind_group_layout"),
                });
        let uniform_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("sky_uniform_bind_group"),
        });

        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sky_pipeline_layout"),
                bind_group_layouts: &[&uniform_bind_group_layout, &environment_bind_group_layout],
                push_constant_ranges: &[],
            });
        let sky_shader_module = gpu
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/sky.wgsl"));
        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &sky_shader_module,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &sky_shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
            });
        Self {
            uniform_buffer,
            pipeline,
            uniform_bind_group,
            environment_bind_group,
        }
    }

    pub fn render<'rp>(&'rp mut self, render_pass: &mut wgpu::RenderPass<'rp>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &self.environment_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkyUniform {
    pub view_position: nalgebra_glm::Vec4,
    pub view: nalgebra_glm::Mat4,
    pub view_projection: nalgebra_glm::Mat4,
    pub inverse_projection: nalgebra_glm::Mat4,
    pub inverse_view: nalgebra_glm::Mat4,
}
