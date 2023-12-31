use nalgebra_glm as glm;
use petgraph::Graph;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug, path::Path};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to import gltf asset!")]
    ImportGltfAsset(#[source] gltf::Error),

    #[error("No primitive vertex positions for a primitive in the model.")]
    ReadVertexPositions,
}

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub geometry: Geometry,
    pub graph: SceneGraph,
}

impl Scene {
    pub fn import_gltf(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let (gltf, buffers, _images) =
            gltf::import(path.as_ref()).map_err(Error::ImportGltfAsset)?;
        gltf.nodes()
            .try_for_each(|gltf_node| self.import_node(gltf_node, &buffers))
    }

    fn import_node(
        &mut self,
        gltf_node: gltf::Node<'_>,
        buffers: &Vec<gltf::buffer::Data>,
    ) -> Result<(), Error> {
        let mut node = Node::default();
        let name = gltf_node.name().unwrap_or("Unnamed");
        log::info!("Imported node '{name}'...");
        node.transform = Transform::from(gltf_node.transform().decomposed());
        if let Some(mesh) = gltf_node.mesh() {
            self.import_mesh(mesh, buffers, node)?;
        }
        Ok(())
    }

    fn import_mesh(
        &mut self,
        mesh: gltf::Mesh<'_>,
        buffers: &Vec<gltf::buffer::Data>,
        mut node: Node,
    ) -> Result<(), Error> {
        let name = mesh.name().unwrap_or("Unnamed");
        log::info!("Importing mesh '{name}'...");
        let mut primitives = Vec::new();
        mesh.primitives()
            .try_for_each(|primitive| self.import_primitive(primitive, buffers, &mut primitives))?;
        let mesh = MeshGeometry {
            name: name.to_string(),
            primitives,
        };
        let mesh_key = mesh.name.to_string();
        self.geometry.meshes.insert(mesh_key.to_string(), mesh);
        node.mesh = Some(mesh_key);
        Ok(())
    }

    fn import_primitive(
        &mut self,
        gltf_primitive: gltf::Primitive<'_>,
        buffers: &Vec<gltf::buffer::Data>,
        primitives: &mut Vec<Primitive>,
    ) -> Result<(), Error> {
        let first_index = self.geometry.indices.len();
        let first_vertex = self.geometry.vertices.len();
        let number_of_vertices = self.import_primitive_vertices(&gltf_primitive, buffers)?;
        let number_of_indices = { self.import_primitive_indices(&gltf_primitive, buffers) };
        let primitive = Primitive {
            first_index,
            first_vertex,
            number_of_indices,
            number_of_vertices,
            material_index: gltf_primitive.material().index(),
        };
        log::info!("Constructed primitive: {primitive:#?}");
        primitives.push(primitive);
        Ok(())
    }

    fn import_primitive_indices(
        &mut self,
        gltf_primitive: &gltf::Primitive<'_>,
        buffers: &Vec<gltf::buffer::Data>,
    ) -> usize {
        let reader = gltf_primitive.reader(|buffer| Some(&*buffers[buffer.index()]));
        let vertex_count = self.geometry.vertices.len();
        if let Some(read_indices) = reader.read_indices().take() {
            let indices = read_indices
                .into_u32()
                .map(|x| x + vertex_count as u32)
                .collect::<Vec<_>>();
            let number_of_indices = indices.len();
            self.geometry.indices.extend_from_slice(&indices);
            number_of_indices
        } else {
            0
        }
    }

    fn import_primitive_vertices(
        &mut self,
        gltf_primitive: &gltf::Primitive<'_>,
        buffers: &Vec<gltf::buffer::Data>,
    ) -> Result<usize, Error> {
        let reader = gltf_primitive.reader(|buffer| Some(&*buffers[buffer.index()]));
        let mut positions = Vec::new();
        let read_positions = reader.read_positions().ok_or(Error::ReadVertexPositions)?;
        for position in read_positions {
            positions.push(glm::Vec3::from(position));
        }
        let number_of_vertices = positions.len();
        let normals = reader.read_normals().map_or(
            vec![glm::vec3(0.0, 0.0, 0.0); number_of_vertices],
            |normals| normals.map(glm::Vec3::from).collect::<Vec<_>>(),
        );
        let map_to_vec2 = |coords: gltf::mesh::util::ReadTexCoords<'_>| -> Vec<glm::Vec2> {
            coords.into_f32().map(glm::Vec2::from).collect::<Vec<_>>()
        };
        let uv_0 = reader
            .read_tex_coords(0)
            .map_or(vec![glm::vec2(0.0, 0.0); number_of_vertices], map_to_vec2);
        let uv_1 = reader
            .read_tex_coords(1)
            .map_or(vec![glm::vec2(0.0, 0.0); number_of_vertices], map_to_vec2);
        let convert_joints = |joints: gltf::mesh::util::ReadJoints<'_>| -> Vec<glm::Vec4> {
            joints
                .into_u16()
                .map(|joint| glm::vec4(joint[0] as _, joint[1] as _, joint[2] as _, joint[3] as _))
                .collect::<Vec<_>>()
        };
        let joints_0 = reader.read_joints(0).map_or(
            vec![glm::vec4(0.0, 0.0, 0.0, 0.0); number_of_vertices],
            convert_joints,
        );
        let convert_weights = |weights: gltf::mesh::util::ReadWeights<'_>| -> Vec<glm::Vec4> {
            weights.into_f32().map(glm::Vec4::from).collect::<Vec<_>>()
        };
        let weights_0 = reader.read_weights(0).map_or(
            vec![glm::vec4(1.0, 0.0, 0.0, 0.0); number_of_vertices],
            convert_weights,
        );
        let convert_colors = |colors: gltf::mesh::util::ReadColors<'_>| -> Vec<glm::Vec3> {
            colors
                .into_rgb_f32()
                .map(glm::Vec3::from)
                .collect::<Vec<_>>()
        };
        let colors_0 = reader.read_colors(0).map_or(
            vec![glm::vec3(1.0, 1.0, 1.0); number_of_vertices],
            convert_colors,
        );
        for (index, position) in positions.into_iter().enumerate() {
            self.geometry.vertices.push(Vertex {
                position,
                normal: normals[index],
                uv_0: uv_0[index],
                uv_1: uv_1[index],
                joint_0: joints_0[index],
                weight_0: weights_0[index],
                color_0: colors_0[index],
            });
        }
        Ok(number_of_vertices)
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Geometry {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub meshes: HashMap<Mesh, MeshGeometry>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Serialize, Deserialize, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: glm::Vec3,
    pub normal: glm::Vec3,
    pub uv_0: glm::Vec2,
    pub uv_1: glm::Vec2,
    pub joint_0: glm::Vec4,
    pub weight_0: glm::Vec4,
    pub color_0: glm::Vec3,
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: glm::Vec3::default(),
            normal: glm::Vec3::default(),
            uv_0: glm::Vec2::default(),
            uv_1: glm::Vec2::default(),
            joint_0: glm::Vec4::default(),
            weight_0: glm::Vec4::default(),
            color_0: glm::vec3(1.0, 1.0, 1.0),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SceneGraph(pub Graph<Node, ()>);

pub type Mesh = String;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct MeshGeometry {
    pub name: String,
    pub primitives: Vec<Primitive>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Primitive {
    pub first_vertex: usize,
    pub first_index: usize,
    pub number_of_vertices: usize,
    pub number_of_indices: usize,
    pub material_index: Option<usize>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    name: String,
    transform: Transform,
    mesh: Option<Mesh>,
}

#[derive(Default, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Transform {
    pub translation: glm::Vec3,
    pub rotation: glm::Quat,
    pub scale: glm::Vec3,
}

impl From<([f32; 3], [f32; 4], [f32; 3])> for Transform {
    fn from((translation, rotation, scale): ([f32; 3], [f32; 4], [f32; 3])) -> Self {
        Self {
            translation: glm::vec3(translation[0], translation[1], translation[2]),
            rotation: glm::quat(rotation[0], rotation[1], rotation[2], rotation[3]),
            scale: glm::vec3(scale[0], scale[1], scale[2]),
        }
    }
}

impl From<Transform> for glm::Mat4 {
    fn from(transform: Transform) -> Self {
        glm::translation(&transform.translation)
            * glm::quat_to_mat4(&transform.rotation)
            * glm::scaling(&transform.scale)
    }
}
