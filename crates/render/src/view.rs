pub struct WorldRender {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub indirect_draw_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub object_buffer: wgpu::Buffer,
    pub object_buffer_bind_group: wgpu::BindGroup,
    pub pipeline: wgpu::RenderPipeline,
    pub instance_id_buffer: wgpu::Buffer,
}

impl WorldRender {
    pub fn new(gpu: &crate::gpu::Gpu, world: &world::World) -> Self {
        let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &gpu.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&world.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );

        let index_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &gpu.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&world.indices),
                usage: wgpu::BufferUsages::INDEX,
            },
        );

        let (uniform_buffer, uniform_bind_group_layout, uniform_bind_group) = create_uniform(gpu);

        let mut objects = Vec::new();
        let mut draw_commands = Vec::new();
        if let Some(scene_index) = world.default_scene_index {
            let scene = &world.scenes[scene_index];
            for graph_node_index in scene.graph.node_indices() {
                let node_index = scene.graph[graph_node_index];
                let node = &world.nodes[node_index];
                if let Some(mesh_index) = node.mesh_index {
                    let mesh = &world.meshes[mesh_index];
                    let transform = world.global_transform(&scene.graph, graph_node_index);
                    for primitive in mesh.primitives.iter() {
                        let draw_command = DrawIndexedIndirectArgs {
                            index_count: primitive.number_of_indices as _,
                            instance_count: 1,
                            first_index: primitive.index_offset as _,
                            base_vertex: primitive.vertex_offset as _,
                            first_instance: 0,
                        };
                        draw_commands.push(draw_command);
                        objects.push(transform);
                    }
                }
            }
        }

        let indirect_draw_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &gpu.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Indirect Draw Buffer"),
                contents: bytemuck::cast_slice(&draw_commands),
                usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
            },
        );

        let object_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &gpu.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Object Buffer"),
                contents: bytemuck::cast_slice(&objects),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            },
        );
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

        let bind_group_layouts = &[&uniform_bind_group_layout, &object_buffer_bind_group_layout];

        let pipeline = create_pipeline(
            gpu,
            bind_group_layouts,
            false,
            wgpu::PrimitiveTopology::TriangleList,
            wgpu::PolygonMode::Fill,
        );

        let instance_ids: Vec<u32> = (0..objects.len() as u32).collect();
        let instance_id_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &gpu.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance ID Buffer"),
                contents: bytemuck::cast_slice(&instance_ids),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );

        Self {
            vertex_buffer,
            index_buffer,
            indirect_draw_buffer,
            instance_id_buffer,
            uniform_buffer,
            uniform_bind_group,
            object_buffer,
            object_buffer_bind_group,
            pipeline,
        }
    }

    pub fn render<'rp>(
        &'rp mut self,
        render_pass: &mut wgpu::RenderPass<'rp>,
        gpu: &crate::gpu::Gpu,
        world: &world::World,
    ) {
        let Some(scene_index) = world.default_scene_index else {
            return;
        };
        let scene = &world.scenes[scene_index];

        let (camera_position, projection, view) =
            world::create_camera_matrices(world, scene, gpu.aspect_ratio());

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
        render_pass.set_bind_group(1, &self.object_buffer_bind_group, &[]);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_id_buffer.slice(..));

        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_pipeline(&self.pipeline);

        let mut offset = 0;
        for graph_node_index in scene.graph.node_indices() {
            if let Some(mesh_index) = world.nodes[scene.graph[graph_node_index]].mesh_index {
                let mesh = &world.meshes[mesh_index];
                for _ in mesh.primitives.iter() {
                    render_pass.draw_indexed_indirect(&self.indirect_draw_buffer, offset as _);
                    offset += std::mem::size_of::<DrawIndexedIndirectArgs>() as u64;
                }
            }
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
                    index_description(&index_attributes()),
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
        array_stride: std::mem::size_of::<world::Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes,
    }
}

pub fn index_attributes() -> Vec<wgpu::VertexAttribute> {
    wgpu::vertex_attr_array![
        7 => Uint32, // instance_id
    ]
    .to_vec()
}

pub fn index_description(attributes: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<u32>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
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

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawIndexedIndirectArgs {
    /// The number of indices to draw.
    pub index_count: u32,

    /// The number of instances to draw.
    pub instance_count: u32,

    /// The first index within the index buffer.
    pub first_index: u32,

    /// The value added to the vertex index before indexing into the vertex buffer.
    pub base_vertex: i32,

    /// The instance ID of the first instance to draw.
    ///
    /// Has to be 0, unless [`Features::INDIRECT_FIRST_INSTANCE`](crate::Features::INDIRECT_FIRST_INSTANCE) is enabled.
    pub first_instance: u32,
}

const SHADER_SOURCE: &str = "
struct Uniform {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    camera_position: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> ubo: Uniform;

struct Object {
    matrix: mat4x4<f32>,
}

@group(1) @binding(0)
var<storage, read> objects: array<Object>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv_0: vec2<f32>,
    @location(3) uv_1: vec2<f32>,
    @location(4) joint_0: vec4<f32>,
    @location(5) weight_0: vec4<f32>,
    @location(6) color_0: vec3<f32>,
    @location(7) instance_id: u32,

}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

@vertex
fn vertex_main(vert: VertexInput) -> VertexOutput {
    let mvp = ubo.projection * ubo.view * objects[vert.instance_id].matrix; 
    var out: VertexOutput;
    out.position = mvp * vec4(vert.position, 1.0);
    out.normal = vec4((mvp * vec4(vert.normal, 0.0)).xyz, 1.0).xyz;
    out.color = vert.color_0;
    out.tex_coord = vert.uv_0;
    return out;
};

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // var base_color = material.base_color * textureSampleLevel(base_color_texture, base_color_sampler, in.tex_coord, 0.0);
    var base_color = in.color;

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
