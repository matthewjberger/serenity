#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to import gltf scene!")]
    ImportGltfScene(#[source] gltf::Error),

    #[error("No primitive vertex positions specified for a mesh primitive.")]
    ReadVertexPositions,
}

type Result<T, E = Error> = std::result::Result<T, E>;

pub fn import_gltf(path: impl AsRef<std::path::Path>) -> Result<crate::scene::Scene> {
    let (gltf, buffers, raw_images) =
        gltf::import(path.as_ref()).map_err(Error::ImportGltfScene)?;

    let mut samplers = std::collections::HashMap::new();
    samplers.insert("default".to_string(), crate::scene::Sampler::default());
    let sampler_handles = gltf
        .samplers()
        .map(crate::scene::Sampler::from)
        .map(|sampler| {
            let id = uuid::Uuid::new_v4().to_string();
            samplers.insert(id.to_string(), sampler);
            id
        })
        .collect::<Vec<_>>();

    let mut images = std::collections::HashMap::new();
    let image_handles = raw_images
        .into_iter()
        .map(crate::scene::Image::from)
        .map(|image| {
            let id = uuid::Uuid::new_v4().to_string();
            images.insert(id.to_string(), image);
            id
        })
        .collect::<Vec<_>>();

    let mut textures = std::collections::HashMap::new();
    let texture_handles = gltf
        .textures()
        .map(|texture| {
            let id = uuid::Uuid::new_v4().to_string();
            let sampler = match texture.sampler().index() {
                Some(index) => sampler_handles[index].to_string(),
                None => "default".to_string(),
            };
            textures.insert(
                id.to_string(),
                crate::scene::Texture {
                    label: texture.name().unwrap_or("Unnamed texture").to_string(),
                    image: image_handles[texture.source().index()].to_string(),
                    sampler,
                },
            );
            id
        })
        .collect::<Vec<_>>();

    // TODO: nodes can reference these
    let mut materials = std::collections::HashMap::new();
    materials.insert("default".to_string(), crate::scene::Material::default());
    let material_handles = gltf
        .materials()
        .map(|primitive_material| {
            let pbr = primitive_material.pbr_metallic_roughness();
            let id = uuid::Uuid::new_v4().to_string();
            let mut material = crate::scene::Material {
                base_color_factor: nalgebra_glm::Vec4::from(pbr.base_color_factor()),
                ..Default::default()
            };
            if let Some(base_color_texture) = pbr.base_color_texture() {
                material.base_color_texture =
                    texture_handles[base_color_texture.texture().index()].to_string();
            }
            materials.insert(id.to_string(), material);
            id
        })
        .collect::<Vec<_>>();

    let graph = gltf
        .scenes()
        .map(|gltf_scene| import_scene(gltf_scene, &buffers, &material_handles))
        .next() // Only take the first scene, even though gltf can store multiple
        .unwrap_or_default();

    Ok(crate::scene::Scene {
        graph,
        images,
        samplers,
        textures,
        materials,
    })
}

impl From<gltf::material::AlphaMode> for crate::scene::AlphaMode {
    fn from(mode: gltf::material::AlphaMode) -> Self {
        match mode {
            gltf::material::AlphaMode::Opaque => crate::scene::AlphaMode::Opaque,
            gltf::material::AlphaMode::Mask => crate::scene::AlphaMode::Mask,
            gltf::material::AlphaMode::Blend => crate::scene::AlphaMode::Blend,
        }
    }
}

impl From<gltf::texture::Sampler<'_>> for crate::scene::Sampler {
    fn from(sampler: gltf::texture::Sampler<'_>) -> Self {
        let min_filter = sampler
            .min_filter()
            .map(|filter| match filter {
                gltf::texture::MinFilter::Linear
                | gltf::texture::MinFilter::LinearMipmapLinear
                | gltf::texture::MinFilter::LinearMipmapNearest => crate::scene::Filter::Linear,
                gltf::texture::MinFilter::Nearest
                | gltf::texture::MinFilter::NearestMipmapLinear
                | gltf::texture::MinFilter::NearestMipmapNearest => crate::scene::Filter::Nearest,
            })
            .unwrap_or_default();

        let mag_filter = sampler
            .mag_filter()
            .map(|filter| match filter {
                gltf::texture::MagFilter::Linear => crate::scene::Filter::Linear,
                gltf::texture::MagFilter::Nearest => crate::scene::Filter::Nearest,
            })
            .unwrap_or_default();

        let wrap_s = match sampler.wrap_s() {
            gltf::texture::WrappingMode::ClampToEdge => crate::scene::WrappingMode::ClampToEdge,
            gltf::texture::WrappingMode::MirroredRepeat => {
                crate::scene::WrappingMode::MirroredRepeat
            }
            gltf::texture::WrappingMode::Repeat => crate::scene::WrappingMode::Repeat,
        };

        let wrap_t = match sampler.wrap_t() {
            gltf::texture::WrappingMode::ClampToEdge => crate::scene::WrappingMode::ClampToEdge,
            gltf::texture::WrappingMode::MirroredRepeat => {
                crate::scene::WrappingMode::MirroredRepeat
            }
            gltf::texture::WrappingMode::Repeat => crate::scene::WrappingMode::Repeat,
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            min_filter,
            mag_filter,
            wrap_s,
            wrap_t,
        }
    }
}

impl From<gltf::image::Data> for crate::scene::Image {
    fn from(data: gltf::image::Data) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            pixels: data.pixels.to_vec(),
            format: data.format.into(),
            width: data.width,
            height: data.height,
        }
    }
}

impl From<gltf::image::Format> for crate::scene::ImageFormat {
    fn from(value: gltf::image::Format) -> Self {
        match value {
            gltf::image::Format::R8 => crate::scene::ImageFormat::R8,
            gltf::image::Format::R8G8 => crate::scene::ImageFormat::R8G8,
            gltf::image::Format::R8G8B8 => crate::scene::ImageFormat::R8G8B8,
            gltf::image::Format::R8G8B8A8 => crate::scene::ImageFormat::R8G8B8A8,
            gltf::image::Format::R16 => crate::scene::ImageFormat::R16,
            gltf::image::Format::R16G16 => crate::scene::ImageFormat::R16G16,
            gltf::image::Format::R16G16B16 => crate::scene::ImageFormat::R16G16B16,
            gltf::image::Format::R16G16B16A16 => crate::scene::ImageFormat::R16G16B16A16,
            gltf::image::Format::R32G32B32FLOAT => crate::scene::ImageFormat::R32G32B32,
            gltf::image::Format::R32G32B32A32FLOAT => crate::scene::ImageFormat::R32G32B32A32,
        }
    }
}

fn import_scene(
    gltf_scene: gltf::Scene,
    buffers: &[gltf::buffer::Data],
    material_handles: &[String],
) -> crate::scene::SceneGraph {
    let mut scenegraph = crate::scene::SceneGraph::default();
    let root_node = scenegraph.add_node(crate::scene::Node {
        label: "Root".to_string(),
        ..Default::default()
    });
    gltf_scene.nodes().for_each(|node| {
        import_node(root_node, node, buffers, &mut scenegraph, material_handles);
    });
    scenegraph
}

fn import_node(
    parent_node_index: petgraph::graph::NodeIndex,
    gltf_node: gltf::Node,
    buffers: &[gltf::buffer::Data],
    scenegraph: &mut crate::scene::SceneGraph,
    material_handles: &[String],
) {
    let name = gltf_node.name().unwrap_or("Unnamed node");

    let mut scene_node = crate::scene::Node {
        label: name.to_string(),
        transform: crate::scene::Transform::from(gltf_node.transform().decomposed()),
        ..Default::default()
    };

    gltf_node
        .mesh()
        .map(|gltf_mesh| import_mesh(gltf_mesh, buffers, material_handles))
        .map(|mesh| {
            mesh.map(|mesh| {
                scene_node
                    .components
                    .push(crate::scene::NodeComponent::Mesh(mesh));
            })
        });

    if let Some(camera) = gltf_node.camera() {
        scene_node
            .components
            .push(crate::scene::NodeComponent::Camera(camera.into()));
    }

    if let Some(light) = gltf_node.light() {
        scene_node
            .components
            .push(crate::scene::NodeComponent::Light(light.into()));
    }

    let node_index = scenegraph.add_node(scene_node);

    if parent_node_index != node_index {
        scenegraph.add_edge(parent_node_index, node_index, ());
    }

    gltf_node.children().for_each(|child| {
        import_node(node_index, child, buffers, scenegraph, material_handles);
    });
}

fn import_mesh(
    mesh: gltf::Mesh,
    buffers: &[gltf::buffer::Data],
    material_handles: &[String],
) -> Result<crate::scene::Mesh> {
    Ok(crate::scene::Mesh {
        id: uuid::Uuid::new_v4().to_string(),
        label: mesh.name().unwrap_or("Unnamed mesh").to_string(),
        primitives: mesh
            .primitives()
            .map(|primitive| import_primitive(primitive, buffers, material_handles))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn import_primitive(
    primitive: gltf::Primitive,
    buffers: &[gltf::buffer::Data],
    material_handles: &[String],
) -> Result<crate::scene::Primitive> {
    let material = match primitive.material().index() {
        Some(index) => material_handles[index].to_string(),
        None => "default".to_string(),
    };
    Ok(crate::scene::Primitive {
        mode: primitive.mode().into(),
        material,
        vertices: import_primitive_vertices(&primitive, buffers)?,
        indices: import_primitive_indices(&primitive, buffers),
    })
}

fn import_primitive_indices(
    gltf_primitive: &gltf::Primitive,
    buffers: &[gltf::buffer::Data],
) -> Vec<u32> {
    gltf_primitive
        .reader(|buffer| Some(&*buffers[buffer.index()]))
        .read_indices()
        .take()
        .map(|read_indices| read_indices.into_u32().collect::<Vec<_>>())
        .unwrap_or_default()
}

fn import_primitive_vertices(
    gltf_primitive: &gltf::Primitive,
    buffers: &[gltf::buffer::Data],
) -> Result<Vec<crate::scene::Vertex>> {
    let reader = gltf_primitive.reader(|buffer| Some(&*buffers[buffer.index()]));

    let mut positions = Vec::new();
    let read_positions = reader.read_positions().ok_or(Error::ReadVertexPositions)?;
    read_positions.for_each(|position| {
        positions.push(nalgebra_glm::Vec3::from(position));
    });
    let number_of_vertices = positions.len();
    let normals = reader.read_normals().map_or(
        vec![nalgebra_glm::vec3(0.0, 0.0, 0.0); number_of_vertices],
        |normals| normals.map(nalgebra_glm::Vec3::from).collect::<Vec<_>>(),
    );
    let map_to_vec2 = |coords: gltf::mesh::util::ReadTexCoords| -> Vec<nalgebra_glm::Vec2> {
        coords
            .into_f32()
            .map(nalgebra_glm::Vec2::from)
            .collect::<Vec<_>>()
    };
    let uv_0 = reader.read_tex_coords(0).map_or(
        vec![nalgebra_glm::vec2(0.0, 0.0); number_of_vertices],
        map_to_vec2,
    );
    let uv_1 = reader.read_tex_coords(1).map_or(
        vec![nalgebra_glm::vec2(0.0, 0.0); number_of_vertices],
        map_to_vec2,
    );
    let convert_joints = |joints: gltf::mesh::util::ReadJoints| -> Vec<nalgebra_glm::Vec4> {
        joints
            .into_u16()
            .map(|joint| {
                nalgebra_glm::vec4(joint[0] as _, joint[1] as _, joint[2] as _, joint[3] as _)
            })
            .collect::<Vec<_>>()
    };
    let joints_0 = reader.read_joints(0).map_or(
        vec![nalgebra_glm::vec4(0.0, 0.0, 0.0, 0.0); number_of_vertices],
        convert_joints,
    );
    let convert_weights = |weights: gltf::mesh::util::ReadWeights| -> Vec<nalgebra_glm::Vec4> {
        weights
            .into_f32()
            .map(nalgebra_glm::Vec4::from)
            .collect::<Vec<_>>()
    };
    let weights_0 = reader.read_weights(0).map_or(
        vec![nalgebra_glm::vec4(1.0, 0.0, 0.0, 0.0); number_of_vertices],
        convert_weights,
    );
    let convert_colors = |colors: gltf::mesh::util::ReadColors| -> Vec<nalgebra_glm::Vec3> {
        colors
            .into_rgb_f32()
            .map(nalgebra_glm::Vec3::from)
            .collect::<Vec<_>>()
    };
    let colors_0 = reader.read_colors(0).map_or(
        vec![nalgebra_glm::vec3(1.0, 1.0, 1.0); number_of_vertices],
        convert_colors,
    );

    // every vertex is guaranteed to have a position attribute,
    // so we can use the position attribute array to index into the other attribute arrays
    let vertices = positions
        .into_iter()
        .enumerate()
        .map(|(index, position)| crate::scene::Vertex {
            position,
            normal: normals[index],
            uv_0: uv_0[index],
            uv_1: uv_1[index],
            joint_0: joints_0[index],
            weight_0: weights_0[index],
            color_0: colors_0[index],
        })
        .collect();

    Ok(vertices)
}

impl From<gltf::Camera<'_>> for crate::scene::Camera {
    fn from(camera: gltf::Camera) -> Self {
        Self {
            projection: match camera.projection() {
                gltf::camera::Projection::Perspective(camera) => {
                    crate::scene::Projection::Perspective(crate::scene::PerspectiveCamera {
                        aspect_ratio: camera.aspect_ratio(),
                        y_fov_rad: camera.yfov(),
                        z_far: camera.zfar(),
                        z_near: camera.znear(),
                    })
                }
                gltf::camera::Projection::Orthographic(camera) => {
                    crate::scene::Projection::Orthographic(crate::scene::OrthographicCamera {
                        x_mag: camera.xmag(),
                        y_mag: camera.ymag(),
                        z_far: camera.zfar(),
                        z_near: camera.znear(),
                    })
                }
            },
            orientation: crate::scene::Orientation::default(),
        }
    }
}

impl From<gltf::khr_lights_punctual::Light<'_>> for crate::scene::Light {
    fn from(light: gltf::khr_lights_punctual::Light) -> Self {
        Self {
            color: light.color().into(),
            intensity: light.intensity(),
            range: light.range().unwrap_or(0.0),
            kind: light.kind().into(),
        }
    }
}

impl From<gltf::khr_lights_punctual::Kind> for crate::scene::LightKind {
    fn from(kind: gltf::khr_lights_punctual::Kind) -> Self {
        match kind {
            gltf::khr_lights_punctual::Kind::Directional => crate::scene::LightKind::Directional,
            gltf::khr_lights_punctual::Kind::Point => crate::scene::LightKind::Point,
            gltf::khr_lights_punctual::Kind::Spot {
                inner_cone_angle,
                outer_cone_angle,
            } => crate::scene::LightKind::Spot {
                inner_cone_angle,
                outer_cone_angle,
            },
        }
    }
}

impl From<gltf::mesh::Mode> for crate::scene::PrimitiveMode {
    fn from(mode: gltf::mesh::Mode) -> Self {
        match mode {
            gltf::mesh::Mode::Points => crate::scene::PrimitiveMode::Points,
            gltf::mesh::Mode::Lines => crate::scene::PrimitiveMode::Lines,
            gltf::mesh::Mode::LineLoop => crate::scene::PrimitiveMode::LineLoop,
            gltf::mesh::Mode::LineStrip => crate::scene::PrimitiveMode::LineStrip,
            gltf::mesh::Mode::Triangles => crate::scene::PrimitiveMode::Triangles,
            gltf::mesh::Mode::TriangleStrip => crate::scene::PrimitiveMode::TriangleStrip,
            gltf::mesh::Mode::TriangleFan => crate::scene::PrimitiveMode::TriangleFan,
        }
    }
}
