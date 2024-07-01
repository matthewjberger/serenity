use image::DynamicImage;

pub struct HdriLoader {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl HdriLoader {
    pub fn new(gpu: &crate::render::gpu::Gpu) -> Self {
        let shader_module = gpu
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/hdri.wgsl"));
        let texture_format = wgpu::TextureFormat::Rgba32Float;
        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("hdri_bind_group_layout"),
                    entries: &[
                        // Input equirectangular texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // Output cubemap
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: texture_format,
                                view_dimension: wgpu::TextureViewDimension::D2Array,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("hdri_pipeline_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = gpu
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("hdri_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: "compute_equirect_to_cubemap",
            });

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    pub fn convert_equirectangular_map_to_cubemap(
        &self,
        gpu: &crate::render::gpu::Gpu,
        dimension: u32,
    ) -> (wgpu::Texture, wgpu::TextureView, wgpu::Sampler) {
        let (metadata, pixels) = load_hdri_bytes(include_bytes!("hdr/pure-sky.hdr"));
        let texture = {
            let image = &crate::world::Image {
                pixels: bytemuck::cast_slice(
                    &pixels
                        .iter()
                        // Adding an alpha channel here is required for aligment
                        // because the loaded hdris use 24-bit pixels but the shader expects 32-bit pixels.
                        .flat_map(|pixel| vec![pixel[0], pixel[1], pixel[2], 1.0])
                        .collect::<Vec<_>>(),
                )
                .to_vec(),
                format: crate::world::ImageFormat::R32G32B32A32F,
                width: metadata.width,
                height: metadata.height,
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
                format: wgpu::TextureFormat::Rgba32Float,
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
                    bytes_per_row: Some(image.width * std::mem::size_of::<[f32; 4]>() as u32),
                    rows_per_image: Some(image.height),
                },
                size,
            );

            texture
        };

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let cubemap = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: dimension,
                height: dimension,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let cubemap_view = cubemap.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        let cubemap_sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&cubemap_view),
                },
            ],
        });

        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("hdri_compute_pass"),
            timestamp_writes: None,
        });

        let num_workgroups = (dimension + 15) / 16;
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(num_workgroups, num_workgroups, 6);

        (cubemap, cubemap_view, cubemap_sampler)
    }
}

fn load_hdri_bytes(bytes: &[u8]) -> (image::codecs::hdr::HdrMetadata, Vec<image::Rgb<f32>>) {
    let decoder = image::codecs::hdr::HdrDecoder::new(std::io::Cursor::new(bytes))
        .expect("Failed to decode HDR");
    let metadata = decoder.metadata();
    let pixels = decoder.read_image_hdr().expect("Failed to read HDR image");
    (metadata, pixels)
}
