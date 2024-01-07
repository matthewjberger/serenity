#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to import gltf scene!")]
    ImportGltfScene(#[source] gltf::Error),

    #[error("No primitive vertex positions specified for a mesh primitive.")]
    ReadVertexPositions,
}

type Result<T, E = Error> = std::result::Result<T, E>;

pub fn import_gltf(path: impl AsRef<std::path::Path>) -> Result<Vec<crate::scene::Scene>> {
    let (gltf, buffers, _images) = gltf::import(path.as_ref()).map_err(Error::ImportGltfScene)?;
    Ok(gltf
        .scenes()
        .map(|gltf_scene| import_scene(gltf_scene, &buffers))
        .collect())
}

fn import_scene(gltf_scene: gltf::Scene, buffers: &[gltf::buffer::Data]) -> crate::scene::Scene {
    let mut scene = crate::scene::Scene {
        name: gltf_scene.name().unwrap_or("Unnamed scene").to_string(),
        ..Default::default()
    };
    let root_node = scene.graph.add_node(crate::scene::Node {
        name: "Root".to_string(),
        ..Default::default()
    });
    gltf_scene.nodes().for_each(|node| {
        import_node(root_node, node, buffers, &mut scene);
    });
    scene
}

fn import_node(
    parent_node_index: petgraph::graph::NodeIndex,
    gltf_node: gltf::Node,
    buffers: &[gltf::buffer::Data],
    scene: &mut crate::scene::Scene,
) {
    let name = gltf_node.name().unwrap_or("Unnamed node");

    let mut scene_node = crate::scene::Node {
        name: name.to_string(),
        transform: crate::scene::Transform::from(gltf_node.transform().decomposed()),
        ..Default::default()
    };

    gltf_node
        .mesh()
        .map(|gltf_mesh| import_mesh(gltf_mesh, buffers))
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

    let node_index = scene.graph.add_node(scene_node);

    if parent_node_index != node_index {
        scene.graph.add_edge(parent_node_index, node_index, ());
    }

    gltf_node.children().for_each(|child| {
        import_node(node_index, child, buffers, scene);
    });
}

fn import_mesh(mesh: gltf::Mesh, buffers: &[gltf::buffer::Data]) -> Result<crate::scene::Mesh> {
    let id = mesh.name().unwrap_or("Unnamed mesh");
    Ok(crate::scene::Mesh {
        name: id.to_string(),
        primitives: mesh
            .primitives()
            .map(|primitive| import_primitive(primitive, buffers))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn import_primitive(
    primitive: gltf::Primitive,
    buffers: &[gltf::buffer::Data],
) -> Result<crate::scene::Primitive> {
    Ok(crate::scene::Primitive {
        mode: primitive.mode().into(),
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
            name: camera.name().unwrap_or("Unnamed camera").to_string(),
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
            name: light.name().unwrap_or("Unnamed light").to_string(),
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
