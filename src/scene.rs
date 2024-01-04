#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Scene {
    pub name: String,
    pub graph: SceneGraph,
    pub children: Vec<Scene>,
}

impl Scene {
    pub fn walk_dfs(&self, mut visit_node: impl FnMut(&Node)) {
        if self.graph.0.node_count() == 0 {
            return;
        }
        let mut dfs = petgraph::visit::Dfs::new(&self.graph.0, petgraph::graph::NodeIndex::new(0));
        while let Some(node_index) = dfs.next(&self.graph.0) {
            visit_node(&self.graph.0[node_index]);
        }
    }

    pub fn flatten(
        &self,
    ) -> (
        Vec<crate::scene::Vertex>,
        Vec<u16>,
        std::collections::HashMap<String, Vec<PrimitiveDrawCommand>>,
    ) {
        let (mut vertices, mut indices, mut meshes) =
            (Vec::new(), Vec::new(), std::collections::HashMap::new());

        self.walk_dfs(|node| {
            if let Some(mesh) = node.mesh.as_ref() {
                let (vertex_offset, index_offset) = (vertices.len(), indices.len());
                meshes.insert(mesh.id.to_string(), {
                    mesh.primitives
                        .iter()
                        .map(|primitive| {
                            let primitive_vertices = primitive.vertices.to_vec();
                            let number_of_vertices = primitive_vertices.len();
                            vertices.extend_from_slice(&primitive_vertices);

                            let primitive_indices = primitive
                                .indices
                                .iter()
                                .map(|x| *x as u16)
                                .collect::<Vec<_>>();
                            let number_of_indices = primitive_indices.len();
                            indices.extend_from_slice(&primitive_indices);

                            PrimitiveDrawCommand {
                                vertex_offset,
                                index_offset,
                                vertices: number_of_vertices,
                                indices: number_of_indices,
                            }
                        })
                        .collect::<Vec<_>>()
                });
            }
        });

        (vertices, indices, meshes)
    }
}

#[repr(C)]
#[derive(
    Debug, Copy, Clone, serde::Serialize, serde::Deserialize, bytemuck::Pod, bytemuck::Zeroable,
)]
pub struct Vertex {
    pub position: nalgebra_glm::Vec3,
    pub normal: nalgebra_glm::Vec3,
    pub uv_0: nalgebra_glm::Vec2,
    pub uv_1: nalgebra_glm::Vec2,
    pub joint_0: nalgebra_glm::Vec4,
    pub weight_0: nalgebra_glm::Vec4,
    pub color_0: nalgebra_glm::Vec3,
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: nalgebra_glm::Vec3::default(),
            normal: nalgebra_glm::Vec3::default(),
            uv_0: nalgebra_glm::Vec2::default(),
            uv_1: nalgebra_glm::Vec2::default(),
            joint_0: nalgebra_glm::Vec4::default(),
            weight_0: nalgebra_glm::Vec4::default(),
            color_0: nalgebra_glm::vec3(1.0, 1.0, 1.0),
        }
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneGraph(pub petgraph::Graph<Node, ()>);

impl SceneGraph {
    pub fn as_dotviz(&self) -> String {
        format!(
            "{:?}",
            petgraph::dot::Dot::with_config(&self.0, &[petgraph::dot::Config::EdgeNoLabel])
        )
    }
}

impl std::ops::Deref for SceneGraph {
    type Target = petgraph::Graph<Node, ()>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SceneGraph {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Mesh {
    pub id: String,
    pub primitives: Vec<Primitive>,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Primitive {
    pub mode: PrimitiveMode,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub id: String,
    pub transform: Transform,
    pub camera: Option<Camera>,
    pub mesh: Option<Mesh>,
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Transform {
    pub translation: nalgebra_glm::Vec3,
    pub rotation: nalgebra_glm::Quat,
    pub scale: nalgebra_glm::Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: nalgebra_glm::Vec3::new(0.0, 0.0, 0.0),
            rotation: nalgebra_glm::Quat::identity(),
            scale: nalgebra_glm::Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl From<([f32; 3], [f32; 4], [f32; 3])> for Transform {
    fn from((translation, rotation, scale): ([f32; 3], [f32; 4], [f32; 3])) -> Self {
        Self {
            translation: nalgebra_glm::vec3(translation[0], translation[1], translation[2]),
            rotation: nalgebra_glm::quat(rotation[0], rotation[1], rotation[2], rotation[3]),
            scale: nalgebra_glm::vec3(scale[0], scale[1], scale[2]),
        }
    }
}

impl From<Transform> for nalgebra_glm::Mat4 {
    fn from(transform: Transform) -> Self {
        nalgebra_glm::translation(&transform.translation)
            * nalgebra_glm::quat_to_mat4(&transform.rotation)
            * nalgebra_glm::scaling(&transform.scale)
    }
}

impl Transform {
    pub fn as_view_matrix(&self) -> nalgebra_glm::Mat4 {
        let eye = self.translation;
        let target = self.translation + self.forward();
        let up = self.up();
        nalgebra_glm::look_at(&eye, &target, &up)
    }

    pub fn right(&self) -> nalgebra_glm::Vec3 {
        nalgebra_glm::quat_rotate_vec3(&self.rotation.normalize(), &nalgebra_glm::Vec3::x())
    }

    pub fn up(&self) -> nalgebra_glm::Vec3 {
        nalgebra_glm::quat_rotate_vec3(&self.rotation.normalize(), &nalgebra_glm::Vec3::y())
    }

    pub fn forward(&self) -> nalgebra_glm::Vec3 {
        nalgebra_glm::quat_rotate_vec3(&self.rotation.normalize(), &(nalgebra_glm::Vec3::z()))
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Camera {
    pub id: String,
    pub projection: Projection,
    pub enabled: bool,
}

impl Camera {
    pub fn projection_matrix(&self, aspect_ratio: f32) -> nalgebra_glm::Mat4 {
        match &self.projection {
            Projection::Perspective(camera) => camera.matrix(aspect_ratio),
            Projection::Orthographic(camera) => camera.matrix(),
        }
    }

    #[allow(dead_code)]
    pub fn is_orthographic(&self) -> bool {
        match self.projection {
            Projection::Perspective(_) => false,
            Projection::Orthographic(_) => true,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub enum Projection {
    Perspective(PerspectiveCamera),
    Orthographic(OrthographicCamera),
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct PerspectiveCamera {
    pub aspect_ratio: Option<f32>,
    pub y_fov_rad: f32,
    pub z_far: Option<f32>,
    pub z_near: f32,
}

impl PerspectiveCamera {
    pub fn matrix(&self, viewport_aspect_ratio: f32) -> nalgebra_glm::Mat4 {
        let aspect_ratio = if let Some(aspect_ratio) = self.aspect_ratio {
            aspect_ratio
        } else {
            viewport_aspect_ratio
        };

        if let Some(z_far) = self.z_far {
            nalgebra_glm::perspective_zo(aspect_ratio, self.y_fov_rad, self.z_near, z_far)
        } else {
            nalgebra_glm::infinite_perspective_rh_zo(aspect_ratio, self.y_fov_rad, self.z_near)
        }
    }
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct OrthographicCamera {
    pub x_mag: f32,
    pub y_mag: f32,
    pub z_far: f32,
    pub z_near: f32,
}

impl OrthographicCamera {
    pub fn matrix(&self) -> nalgebra_glm::Mat4 {
        let z_sum = self.z_near + self.z_far;
        let z_diff = self.z_near - self.z_far;
        nalgebra_glm::Mat4::new(
            1.0 / self.x_mag,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0 / self.y_mag,
            0.0,
            0.0,
            0.0,
            0.0,
            2.0 / z_diff,
            0.0,
            0.0,
            0.0,
            z_sum / z_diff,
            1.0,
        )
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PrimitiveMode {
    Points,
    Lines,
    LineLoop,
    LineStrip,
    #[default]
    Triangles,
    TriangleStrip,
    TriangleFan,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrimitiveDrawCommand {
    pub vertex_offset: usize,
    pub index_offset: usize,
    pub vertices: usize,
    pub indices: usize,
}
