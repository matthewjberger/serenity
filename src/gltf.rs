pub fn import_gltf(path: impl AsRef<std::path::Path>) -> crate::world::World {
    let (gltf, buffers, raw_images) = gltf::import(path.as_ref()).expect("Failed to import gltf");
    let (samplers, sampler_ids) = import_samplers(&gltf);
    let (images, image_ids) = import_images(&raw_images);
    let (textures, texture_ids) = import_textures(&gltf, sampler_ids, image_ids);
    let (materials, material_ids) = import_materials(&gltf, texture_ids);
    let (meshes, mesh_ids) = import_meshes(&gltf, &buffers, material_ids);
    let (node_ids, scene) = import_graph(&gltf, &mesh_ids);
    let (animations, _animation_ids) = import_animations(&gltf, &node_ids, &buffers);
    let (skins, _skin_ids) = import_skins(&gltf, &buffers, &node_ids);

    let linear_images = raw_images
        .into_iter()
        .map(crate::world::Image::from)
        .collect::<Vec<_>>();
    let linear_samplers = gltf
        .samplers()
        .map(crate::world::Sampler::from)
        .collect::<Vec<_>>();
    let linear_textures = gltf
        .textures()
        .map(|texture| crate::world::LinearTexture {
            label: texture.name().unwrap_or("Unnamed texture").to_string(),
            image_index: texture.source().index(),
            sampler_index: texture.sampler().index(),
        })
        .collect::<Vec<_>>();
    let linear_materials = gltf
        .materials()
        .map(|material| crate::world::LinearMaterial {
            base_color_factor: nalgebra_glm::Vec4::from(
                material.pbr_metallic_roughness().base_color_factor(),
            ),
            base_color_texture_index: material
                .pbr_metallic_roughness()
                .base_color_texture()
                .map(|texture| texture.texture().index())
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>();

    let (linear_meshes, linear_vertices, linear_indices) = {
        let (mut vertices, mut indices) = (vec![], vec![]);
        let meshes = gltf
            .meshes()
            .map(|mesh| {
                crate::world::LinearMesh {
                    primitives: mesh
                        .primitives()
                        .map(|primitive| {
                            let primitive_vertices: Vec<crate::world::Vertex> = {
                                let reader =
                                    primitive.reader(|buffer| Some(&*buffers[buffer.index()]));

                                let mut positions = Vec::new();
                                let read_positions = reader
                                    .read_positions()
                                    .expect("Failed to read gltf vertex positions");
                                read_positions.for_each(|position| {
                                    positions.push(nalgebra_glm::Vec3::from(position));
                                });
                                let number_of_vertices = positions.len();
                                let normals = reader.read_normals().map_or(
                                    vec![nalgebra_glm::vec3(0.0, 0.0, 0.0); number_of_vertices],
                                    |normals| {
                                        normals.map(nalgebra_glm::Vec3::from).collect::<Vec<_>>()
                                    },
                                );
                                let map_to_vec2 =
                            |coords: gltf::mesh::util::ReadTexCoords| -> Vec<nalgebra_glm::Vec2> {
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
                                let convert_joints =
                                |joints: gltf::mesh::util::ReadJoints| -> Vec<nalgebra_glm::Vec4> {
                                    joints
                                        .into_u16()
                                        .map(|joint| {
                                            nalgebra_glm::vec4(
                                                joint[0] as _,
                                                joint[1] as _,
                                                joint[2] as _,
                                                joint[3] as _,
                                            )
                                        })
                                        .collect::<Vec<_>>()
                                };
                                let joints_0 = reader.read_joints(0).map_or(
                                    vec![
                                        nalgebra_glm::vec4(0.0, 0.0, 0.0, 0.0);
                                        number_of_vertices
                                    ],
                                    convert_joints,
                                );
                                let convert_weights =
                            |weights: gltf::mesh::util::ReadWeights| -> Vec<nalgebra_glm::Vec4> {
                                weights.into_f32().map(nalgebra_glm::Vec4::from).collect()
                            };
                                let weights_0 = reader.read_weights(0).map_or(
                                    vec![
                                        nalgebra_glm::vec4(1.0, 0.0, 0.0, 0.0);
                                        number_of_vertices
                                    ],
                                    convert_weights,
                                );
                                let convert_colors =
                                |colors: gltf::mesh::util::ReadColors| -> Vec<nalgebra_glm::Vec3> {
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

                                positions
                                    .into_iter()
                                    .enumerate()
                                    .map(|(index, position)| crate::world::Vertex {
                                        position,
                                        normal: normals[index],
                                        uv_0: uv_0[index],
                                        uv_1: uv_1[index],
                                        joint_0: joints_0[index],
                                        weight_0: weights_0[index],
                                        color_0: colors_0[index],
                                    })
                                    .collect()
                            };

                            let primitive_indices: Vec<u32> = primitive
                                .reader(|buffer| Some(&*buffers[buffer.index()]))
                                .read_indices()
                                .take()
                                .map(|read_indices| read_indices.into_u32().collect())
                                .unwrap_or_default();

                            let primitive = crate::world::LinearPrimitive {
                                mode: primitive.mode().into(),
                                material: primitive.material().index(),
                                vertex_offset: vertices.len(),
                                index_offset: indices.len(),
                                number_of_vertices: primitive_vertices.len(),
                                number_of_indices: primitive_indices.len(),
                            };

                            vertices.extend(primitive_vertices);
                            indices.extend(primitive_indices);

                            primitive
                        })
                        .collect::<Vec<_>>(),
                }
            })
            .collect::<Vec<_>>();
        (meshes, vertices, indices)
    };

    let linear_cameras = {
        let cameras = gltf
            .cameras()
            .map(crate::world::Camera::from)
            .collect::<Vec<_>>();
        if cameras.is_empty() {
            vec![crate::world::Camera::default()]
        } else {
            cameras
        }
    };

    let (mut linear_scenes, linear_nodes, linear_transforms) = {
        let mut nodes = Vec::new();
        let mut transforms = Vec::new();
        let scenes = gltf
            .scenes()
            .map(|gltf_scene| {
                fn visit_node(
                    parent_node_index: petgraph::graph::NodeIndex,
                    node: &gltf::Node,
                    scene: &mut crate::world::LinearScene,
                    nodes: &mut Vec<crate::world::LinearNode>,
                    transforms: &mut Vec<crate::world::Transform>,
                ) {
                    transforms.push(crate::world::Transform::from(node.transform().decomposed()));
                    nodes.push(crate::world::LinearNode {
                        transform_index: Some(transforms.len()),
                        camera_index: node.camera().map(|camera| camera.index()),
                        mesh_index: node.mesh().map(|mesh| mesh.index()),
                        light_index: node.light().map(|light| light.index()),
                    });
                    let node_index = scene.graph.add_node(nodes.len());
                    if parent_node_index != node_index {
                        scene.graph.add_edge(parent_node_index, node_index, ());
                    }
                    node.children().for_each(|child| {
                        visit_node(node_index, &child, scene, nodes, transforms);
                    });
                }

                let mut scene = crate::world::LinearScene::default();
                let root_node_index = scene.graph.add_node(0);

                gltf_scene.nodes().for_each(|root_node| {
                    visit_node(
                        root_node_index,
                        &root_node,
                        &mut scene,
                        &mut nodes,
                        &mut transforms,
                    );
                });

                scene
            })
            .collect::<Vec<_>>();
        (scenes, nodes, transforms)
    };

    if linear_scenes.is_empty() {
        linear_scenes.push(crate::world::LinearScene::default());
    };

    crate::world::World {
        scene,
        images,
        samplers,
        textures,
        materials,
        meshes,
        animations,
        skins,
        active_scene_index: 0,
        linear_images,
        linear_samplers,
        linear_textures,
        linear_materials,
        linear_meshes,
        linear_vertices,
        linear_indices,
        linear_animations: vec![],
        linear_cameras,
        linear_nodes,
        linear_scenes,
        linear_skins: vec![],
        linear_transforms,
    }
}

fn import_graph(gltf: &gltf::Document, mesh_ids: &[String]) -> (Vec<String>, crate::world::Scene) {
    let node_ids = gltf
        .nodes()
        .map(|_| uuid::Uuid::new_v4().to_string())
        .collect::<Vec<_>>();
    let mut first_scenegraph = None;
    let mut scenegraphs = std::collections::HashMap::new();
    scenegraphs.insert("Main".to_string(), crate::world::Scene::default());
    gltf.scenes().for_each(|gltf_scene| {
        let id = uuid::Uuid::new_v4().to_string();
        if first_scenegraph.is_none() {
            first_scenegraph = Some(id.to_string());
        }
        let mut graph = crate::world::Scene::default();
        let root_node = graph.add_node(crate::world::Node {
            label: "Root".to_string(),
            ..Default::default()
        });
        gltf_scene.nodes().for_each(|node| {
            import_node(root_node, node, &mut graph, mesh_ids, &node_ids);
        });
        scenegraphs.insert(id.to_string(), graph);
    });
    let graph = scenegraphs[&first_scenegraph.unwrap_or("Main".to_string())].clone();
    (node_ids, graph)
}

fn import_samplers(
    gltf: &gltf::Document,
) -> (
    std::collections::HashMap<String, crate::world::Sampler>,
    Vec<String>,
) {
    let mut samplers = std::collections::HashMap::new();
    samplers.insert("default".to_string(), crate::world::Sampler::default());
    let sampler_ids = gltf
        .samplers()
        .map(crate::world::Sampler::from)
        .map(|sampler| {
            let id = uuid::Uuid::new_v4().to_string();
            samplers.insert(id.to_string(), sampler);
            id
        })
        .collect::<Vec<_>>();
    (samplers, sampler_ids)
}

fn import_images(
    raw_images: &[gltf::image::Data],
) -> (
    std::collections::HashMap<String, crate::world::Image>,
    Vec<String>,
) {
    let mut images = std::collections::HashMap::new();
    let image_ids = raw_images
        .iter()
        .cloned()
        .map(crate::world::Image::from)
        .map(|image| {
            let id = uuid::Uuid::new_v4().to_string();
            images.insert(id.to_string(), image);
            id
        })
        .collect::<Vec<_>>();
    (images, image_ids)
}

fn import_textures(
    gltf: &gltf::Document,
    sampler_ids: Vec<String>,
    image_ids: Vec<String>,
) -> (
    std::collections::HashMap<String, crate::world::Texture>,
    Vec<String>,
) {
    let mut textures = std::collections::HashMap::new();
    let texture_ids = gltf
        .textures()
        .map(|texture| {
            let id = uuid::Uuid::new_v4().to_string();
            let sampler = match texture.sampler().index() {
                Some(index) => sampler_ids[index].to_string(),
                None => "default".to_string(),
            };
            textures.insert(
                id.to_string(),
                crate::world::Texture {
                    label: texture.name().unwrap_or("Unnamed texture").to_string(),
                    image: image_ids[texture.source().index()].to_string(),
                    sampler,
                },
            );
            id
        })
        .collect::<Vec<_>>();
    (textures, texture_ids)
}

fn import_materials(
    gltf: &gltf::Document,
    texture_ids: Vec<String>,
) -> (
    std::collections::HashMap<String, crate::world::Material>,
    Vec<String>,
) {
    let mut materials = std::collections::HashMap::new();
    materials.insert("default".to_string(), crate::world::Material::default());
    let material_ids = gltf
        .materials()
        .map(|primitive_material| {
            let pbr = primitive_material.pbr_metallic_roughness();
            let id = uuid::Uuid::new_v4().to_string();
            let mut material = crate::world::Material {
                base_color_factor: nalgebra_glm::Vec4::from(pbr.base_color_factor()),
                ..Default::default()
            };
            if let Some(base_color_texture) = pbr.base_color_texture() {
                material.base_color_texture =
                    texture_ids[base_color_texture.texture().index()].to_string();
            }
            materials.insert(id.to_string(), material);
            id
        })
        .collect::<Vec<_>>();
    (materials, material_ids)
}

fn import_meshes(
    gltf: &gltf::Document,
    buffers: &[gltf::buffer::Data],
    material_ids: Vec<String>,
) -> (
    std::collections::HashMap<String, crate::world::Mesh>,
    Vec<String>,
) {
    let mut meshes = std::collections::HashMap::new();
    meshes.insert("default".to_string(), crate::world::Mesh::default());
    let mesh_ids = gltf
        .meshes()
        .map(|primitive_mesh| {
            let id = uuid::Uuid::new_v4().to_string();
            let mesh = import_mesh(primitive_mesh, buffers, &material_ids);
            meshes.insert(id.to_string(), mesh);
            id
        })
        .collect::<Vec<_>>();
    (meshes, mesh_ids)
}

fn import_skins(
    gltf: &gltf::Document,
    buffers: &[gltf::buffer::Data],
    node_ids: &[String],
) -> (
    std::collections::HashMap<String, crate::world::Skin>,
    Vec<String>,
) {
    let mut skins = std::collections::HashMap::new();
    let skin_ids = gltf
        .skins()
        .map(|gltf_skin| {
            let id = uuid::Uuid::new_v4().to_string();
            let reader = gltf_skin.reader(|buffer| Some(&buffers[buffer.index()]));
            let inverse_bind_matrices = reader
                .read_inverse_bind_matrices()
                .map_or(Vec::new(), |matrices| {
                    matrices.map(nalgebra_glm::Mat4::from).collect::<Vec<_>>()
                });
            let joints = gltf_skin
                .joints()
                .enumerate()
                .map(|(index, joint_node)| {
                    let inverse_bind_matrix = *inverse_bind_matrices
                        .get(index)
                        .unwrap_or(&nalgebra_glm::Mat4::identity());
                    crate::world::Joint {
                        inverse_bind_matrix,
                        target: node_ids[joint_node.index()].to_string(),
                    }
                })
                .collect();
            let label = gltf_skin.name().unwrap_or("Unnamed Skin").to_string();
            skins.insert(id.to_string(), crate::world::Skin { label, joints });
            id
        })
        .collect::<Vec<_>>();
    (skins, skin_ids)
}

fn import_animations(
    gltf: &gltf::Document,
    node_ids: &[String],
    buffers: &[gltf::buffer::Data],
) -> (
    std::collections::HashMap<String, crate::world::Animation>,
    Vec<String>,
) {
    let mut animations = std::collections::HashMap::new();
    let ids = gltf
        .animations()
        .map(|gltf_animation| {
            let id = uuid::Uuid::new_v4().to_string();
            let channels = gltf_animation
                .channels()
                .map(|channel| {
                    let target_node = channel.target().node().index();
                    let target = node_ids[target_node].to_string();
                    let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));
                    let inputs = reader
                        .read_inputs()
                        .expect("Failed to read animation channel inputs!")
                        .collect::<Vec<_>>();
                    let outputs = reader
                        .read_outputs()
                        .expect("Failed to read animation channel outputs!");
                    let transformations = match outputs {
                        gltf::animation::util::ReadOutputs::Translations(translations) => {
                            let translations = translations
                                .map(nalgebra_glm::Vec3::from)
                                .collect::<Vec<_>>();
                            crate::world::TransformationSet::Translations(translations)
                        }
                        gltf::animation::util::ReadOutputs::Rotations(rotations) => {
                            let rotations = rotations
                                .into_f32()
                                .map(nalgebra_glm::Vec4::from)
                                .collect::<Vec<_>>();
                            crate::world::TransformationSet::Rotations(rotations)
                        }
                        gltf::animation::util::ReadOutputs::Scales(scales) => {
                            let scales = scales.map(nalgebra_glm::Vec3::from).collect::<Vec<_>>();
                            crate::world::TransformationSet::Scales(scales)
                        }
                        gltf::animation::util::ReadOutputs::MorphTargetWeights(weights) => {
                            let morph_target_weights = weights.into_f32().collect::<Vec<_>>();
                            crate::world::TransformationSet::MorphTargetWeights(
                                morph_target_weights,
                            )
                        }
                    };
                    crate::world::Channel {
                        target,
                        inputs,
                        transformations,
                        interpolation: crate::world::Interpolation::default(),
                    }
                })
                .collect::<Vec<_>>();

            let max_animation_time = channels
                .iter()
                .flat_map(|channel| channel.inputs.iter().copied())
                .fold(0.0, f32::max);
            animations.insert(
                id.to_string(),
                crate::world::Animation {
                    label: gltf_animation
                        .name()
                        .unwrap_or("Unnamed animation")
                        .to_string(),
                    channels,
                    time: 0.0,
                    max_animation_time,
                },
            );
            id
        })
        .collect::<Vec<_>>();
    (animations, ids)
}

impl From<gltf::material::AlphaMode> for crate::world::AlphaMode {
    fn from(mode: gltf::material::AlphaMode) -> Self {
        match mode {
            gltf::material::AlphaMode::Opaque => crate::world::AlphaMode::Opaque,
            gltf::material::AlphaMode::Mask => crate::world::AlphaMode::Mask,
            gltf::material::AlphaMode::Blend => crate::world::AlphaMode::Blend,
        }
    }
}

impl From<gltf::texture::Sampler<'_>> for crate::world::Sampler {
    fn from(sampler: gltf::texture::Sampler<'_>) -> Self {
        let min_filter = sampler
            .min_filter()
            .map(|filter| match filter {
                gltf::texture::MinFilter::Linear
                | gltf::texture::MinFilter::LinearMipmapLinear
                | gltf::texture::MinFilter::LinearMipmapNearest => crate::world::Filter::Linear,
                gltf::texture::MinFilter::Nearest
                | gltf::texture::MinFilter::NearestMipmapLinear
                | gltf::texture::MinFilter::NearestMipmapNearest => crate::world::Filter::Nearest,
            })
            .unwrap_or_default();

        let mag_filter = sampler
            .mag_filter()
            .map(|filter| match filter {
                gltf::texture::MagFilter::Linear => crate::world::Filter::Linear,
                gltf::texture::MagFilter::Nearest => crate::world::Filter::Nearest,
            })
            .unwrap_or_default();

        let wrap_s = match sampler.wrap_s() {
            gltf::texture::WrappingMode::ClampToEdge => crate::world::WrappingMode::ClampToEdge,
            gltf::texture::WrappingMode::MirroredRepeat => {
                crate::world::WrappingMode::MirroredRepeat
            }
            gltf::texture::WrappingMode::Repeat => crate::world::WrappingMode::Repeat,
        };

        let wrap_t = match sampler.wrap_t() {
            gltf::texture::WrappingMode::ClampToEdge => crate::world::WrappingMode::ClampToEdge,
            gltf::texture::WrappingMode::MirroredRepeat => {
                crate::world::WrappingMode::MirroredRepeat
            }
            gltf::texture::WrappingMode::Repeat => crate::world::WrappingMode::Repeat,
        };

        Self {
            min_filter,
            mag_filter,
            wrap_s,
            wrap_t,
        }
    }
}

impl From<gltf::image::Data> for crate::world::Image {
    fn from(data: gltf::image::Data) -> Self {
        Self {
            pixels: data.pixels.to_vec(),
            format: data.format.into(),
            width: data.width,
            height: data.height,
        }
    }
}

impl From<gltf::image::Format> for crate::world::ImageFormat {
    fn from(value: gltf::image::Format) -> Self {
        match value {
            gltf::image::Format::R8 => crate::world::ImageFormat::R8,
            gltf::image::Format::R8G8 => crate::world::ImageFormat::R8G8,
            gltf::image::Format::R8G8B8 => crate::world::ImageFormat::R8G8B8,
            gltf::image::Format::R8G8B8A8 => crate::world::ImageFormat::R8G8B8A8,
            gltf::image::Format::R16 => crate::world::ImageFormat::R16,
            gltf::image::Format::R16G16 => crate::world::ImageFormat::R16G16,
            gltf::image::Format::R16G16B16 => crate::world::ImageFormat::R16G16B16,
            gltf::image::Format::R16G16B16A16 => crate::world::ImageFormat::R16G16B16A16,
            gltf::image::Format::R32G32B32FLOAT => crate::world::ImageFormat::R32G32B32,
            gltf::image::Format::R32G32B32A32FLOAT => crate::world::ImageFormat::R32G32B32A32,
        }
    }
}

fn import_node(
    parent_node_index: petgraph::graph::NodeIndex,
    gltf_node: gltf::Node,
    scenegraph: &mut crate::world::Scene,
    mesh_handles: &[String],
    node_ids: &[String],
) {
    let mut components = Vec::new();

    if let Some(mesh) = gltf_node.mesh() {
        let mesh_id = mesh_handles[mesh.index()].to_string();
        components.push(crate::world::NodeComponent::Mesh(mesh_id));
    }

    if let Some(camera) = gltf_node.camera() {
        components.push(crate::world::NodeComponent::Camera(camera.into()));
    }

    if let Some(light) = gltf_node.light() {
        components.push(crate::world::NodeComponent::Light(light.into()));
    }

    let scene_node = crate::world::Node {
        id: node_ids[gltf_node.index()].to_string().to_string(),
        label: gltf_node.name().unwrap_or("Unnamed node").to_string(),
        transform: crate::world::Transform::from(gltf_node.transform().decomposed()),
        components,
    };

    let node_index = scenegraph.add_node(scene_node);

    if parent_node_index != node_index {
        scenegraph.add_edge(parent_node_index, node_index, ());
    }

    gltf_node.children().for_each(|child| {
        import_node(node_index, child, scenegraph, mesh_handles, node_ids);
    });
}

fn import_mesh(
    mesh: gltf::Mesh,
    buffers: &[gltf::buffer::Data],
    material_handles: &[String],
) -> crate::world::Mesh {
    crate::world::Mesh {
        label: mesh.name().unwrap_or("Unnamed mesh").to_string(),
        linear_index: mesh.index(),
        primitives: mesh
            .primitives()
            .map(|primitive| import_primitive(primitive, buffers, material_handles))
            .collect(),
    }
}

fn import_primitive(
    primitive: gltf::Primitive,
    buffers: &[gltf::buffer::Data],
    material_handles: &[String],
) -> crate::world::Primitive {
    let material = match primitive.material().index() {
        Some(index) => material_handles[index].to_string(),
        None => "default".to_string(),
    };
    crate::world::Primitive {
        mode: primitive.mode().into(),
        material,
        vertices: import_primitive_vertices(&primitive, buffers),
        indices: import_primitive_indices(&primitive, buffers),
    }
}

fn import_primitive_indices(
    gltf_primitive: &gltf::Primitive,
    buffers: &[gltf::buffer::Data],
) -> Vec<u32> {
    gltf_primitive
        .reader(|buffer| Some(&*buffers[buffer.index()]))
        .read_indices()
        .take()
        .map(|read_indices| read_indices.into_u32().collect())
        .unwrap_or_default()
}

fn import_primitive_vertices(
    gltf_primitive: &gltf::Primitive,
    buffers: &[gltf::buffer::Data],
) -> Vec<crate::world::Vertex> {
    let reader = gltf_primitive.reader(|buffer| Some(&*buffers[buffer.index()]));

    let mut positions = Vec::new();
    let read_positions = reader
        .read_positions()
        .expect("Failed to read gltf vertex positions");
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
        weights.into_f32().map(nalgebra_glm::Vec4::from).collect()
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

    positions
        .into_iter()
        .enumerate()
        .map(|(index, position)| crate::world::Vertex {
            position,
            normal: normals[index],
            uv_0: uv_0[index],
            uv_1: uv_1[index],
            joint_0: joints_0[index],
            weight_0: weights_0[index],
            color_0: colors_0[index],
        })
        .collect()
}

impl From<gltf::Camera<'_>> for crate::world::Camera {
    fn from(camera: gltf::Camera) -> Self {
        Self {
            projection: match camera.projection() {
                gltf::camera::Projection::Perspective(camera) => {
                    crate::world::Projection::Perspective(crate::world::PerspectiveCamera {
                        aspect_ratio: camera.aspect_ratio(),
                        y_fov_rad: camera.yfov(),
                        z_far: camera.zfar(),
                        z_near: camera.znear(),
                    })
                }
                gltf::camera::Projection::Orthographic(camera) => {
                    crate::world::Projection::Orthographic(crate::world::OrthographicCamera {
                        x_mag: camera.xmag(),
                        y_mag: camera.ymag(),
                        z_far: camera.zfar(),
                        z_near: camera.znear(),
                    })
                }
            },
            orientation: crate::world::Orientation::default(),
        }
    }
}

impl From<gltf::khr_lights_punctual::Light<'_>> for crate::world::Light {
    fn from(light: gltf::khr_lights_punctual::Light) -> Self {
        Self {
            color: light.color().into(),
            intensity: light.intensity(),
            range: light.range().unwrap_or(0.0),
            kind: light.kind().into(),
        }
    }
}

impl From<gltf::khr_lights_punctual::Kind> for crate::world::LightKind {
    fn from(kind: gltf::khr_lights_punctual::Kind) -> Self {
        match kind {
            gltf::khr_lights_punctual::Kind::Directional => crate::world::LightKind::Directional,
            gltf::khr_lights_punctual::Kind::Point => crate::world::LightKind::Point,
            gltf::khr_lights_punctual::Kind::Spot {
                inner_cone_angle,
                outer_cone_angle,
            } => crate::world::LightKind::Spot {
                inner_cone_angle,
                outer_cone_angle,
            },
        }
    }
}

impl From<gltf::mesh::Mode> for crate::world::PrimitiveMode {
    fn from(mode: gltf::mesh::Mode) -> Self {
        match mode {
            gltf::mesh::Mode::Points => crate::world::PrimitiveMode::Points,
            gltf::mesh::Mode::Lines => crate::world::PrimitiveMode::Lines,
            gltf::mesh::Mode::LineLoop => crate::world::PrimitiveMode::LineLoop,
            gltf::mesh::Mode::LineStrip => crate::world::PrimitiveMode::LineStrip,
            gltf::mesh::Mode::Triangles => crate::world::PrimitiveMode::Triangles,
            gltf::mesh::Mode::TriangleStrip => crate::world::PrimitiveMode::TriangleStrip,
            gltf::mesh::Mode::TriangleFan => crate::world::PrimitiveMode::TriangleFan,
        }
    }
}

#[cfg(test)]
mod tests {
    #[ignore]
    #[test]
    fn import() {
        let scene = crate::gltf::import_gltf("resources/models/DamagedHelmet.glb");
        dbg!(scene.textures);
        dbg!(scene.materials);
    }
}
