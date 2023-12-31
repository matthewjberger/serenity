use nalgebra_glm as glm;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to import gltf scene!")]
    ImportGltfscene(#[source] gltf::Error),

    #[error("No primitive vertex positions for a primitive in the model.")]
    ReadVertexPositions,
}

type Result<T, E = Error> = std::result::Result<T, E>;

pub fn import_gltf(path: impl AsRef<std::path::Path>) -> Result<crate::scene::Scene> {
    let mut scene = crate::scene::Scene::default();
    let root_node = scene.graph.add_node(crate::scene::Node {
        id: "Root".to_string(),
        ..Default::default()
    });
    let (gltf, buffers, _images) = gltf::import(path.as_ref()).map_err(Error::ImportGltfscene)?;
    gltf.scenes().for_each(|gltf_scene| {
        gltf_scene.nodes().for_each(|node| {
            import_node(root_node, node, &buffers, &mut scene);
        });
    });
    Ok(scene)
}

fn import_node(
    parent_node_index: petgraph::graph::NodeIndex,
    gltf_node: gltf::Node,
    buffers: &[gltf::buffer::Data],
    scene: &mut crate::scene::Scene,
) {
    let name = gltf_node.name().unwrap_or("Unnamed node");
    log::info!("Importing node '{name}'...");

    let mut scene_node = crate::scene::Node {
        id: name.to_string(),
        transform: crate::scene::Transform::from(gltf_node.transform().decomposed()),
        ..Default::default()
    };

    gltf_node
        .mesh()
        .map(|gltf_mesh| import_mesh(gltf_mesh, buffers))
        .map(|mesh| {
            mesh.map(|mesh| {
                let name = mesh.id.to_string();
                scene_node.mesh = Some(name.to_string());
                scene.meshes.insert(name, mesh);
            })
        });

    gltf_node.camera().map(|camera| import_camera(camera));

    let node_index = scene.graph.add_node(scene_node.clone());

    log::info!("Added scene node '{name}'");
    scene.graph.add_node(scene_node.clone());
    if parent_node_index != node_index {
        log::info!(
            "Connected scene node '{}' to parent node '{}'",
            node_index.index(),
            parent_node_index.index()
        );
        scene.graph.add_edge(parent_node_index, node_index, ());
    }

    gltf_node.children().for_each(|child| {
        import_node(parent_node_index, child, buffers, scene);
    });
}

fn import_mesh(mesh: gltf::Mesh, buffers: &[gltf::buffer::Data]) -> Result<crate::scene::Mesh> {
    let id = mesh.name().unwrap_or("Unnamed mesh");
    log::info!("Importing mesh '{id}'...");
    let mut primitives = Vec::new();
    mesh.primitives().try_for_each(|primitive| {
        let primitive = import_primitive(id, primitive, buffers)?;
        primitives.push(primitive);
        Ok(())
    })?;
    Ok(crate::scene::Mesh {
        id: id.to_string(),
        primitives,
    })
}

fn import_primitive(
    mesh: &str,
    primitive: gltf::Primitive,
    buffers: &[gltf::buffer::Data],
) -> Result<crate::scene::Primitive> {
    log::info!(
        "Importing primitive '{}' for mesh '{mesh}'...",
        primitive.index(),
    );
    let vertices = import_primitive_vertices(&primitive, buffers)?;
    let indices = import_primitive_indices(&primitive, buffers);
    Ok(crate::scene::Primitive { vertices, indices })
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
        positions.push(glm::Vec3::from(position));
    });
    let number_of_vertices = positions.len();
    let normals = reader.read_normals().map_or(
        vec![glm::vec3(0.0, 0.0, 0.0); number_of_vertices],
        |normals| normals.map(glm::Vec3::from).collect::<Vec<_>>(),
    );
    let map_to_vec2 = |coords: gltf::mesh::util::ReadTexCoords| -> Vec<glm::Vec2> {
        coords.into_f32().map(glm::Vec2::from).collect::<Vec<_>>()
    };
    let uv_0 = reader
        .read_tex_coords(0)
        .map_or(vec![glm::vec2(0.0, 0.0); number_of_vertices], map_to_vec2);
    let uv_1 = reader
        .read_tex_coords(1)
        .map_or(vec![glm::vec2(0.0, 0.0); number_of_vertices], map_to_vec2);
    let convert_joints = |joints: gltf::mesh::util::ReadJoints| -> Vec<glm::Vec4> {
        joints
            .into_u16()
            .map(|joint| glm::vec4(joint[0] as _, joint[1] as _, joint[2] as _, joint[3] as _))
            .collect::<Vec<_>>()
    };
    let joints_0 = reader.read_joints(0).map_or(
        vec![glm::vec4(0.0, 0.0, 0.0, 0.0); number_of_vertices],
        convert_joints,
    );
    let convert_weights = |weights: gltf::mesh::util::ReadWeights| -> Vec<glm::Vec4> {
        weights.into_f32().map(glm::Vec4::from).collect::<Vec<_>>()
    };
    let weights_0 = reader.read_weights(0).map_or(
        vec![glm::vec4(1.0, 0.0, 0.0, 0.0); number_of_vertices],
        convert_weights,
    );
    let convert_colors = |colors: gltf::mesh::util::ReadColors| -> Vec<glm::Vec3> {
        colors
            .into_rgb_f32()
            .map(glm::Vec3::from)
            .collect::<Vec<_>>()
    };
    let colors_0 = reader.read_colors(0).map_or(
        vec![glm::vec3(1.0, 1.0, 1.0); number_of_vertices],
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

fn import_camera(camera: gltf::Camera) -> crate::scene::Camera {
    let projection = match camera.projection() {
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
    };

    crate::scene::Camera {
        id: camera.name().unwrap_or("Unnamed camera").to_string(),
        projection,
        enabled: false,
    }
}
