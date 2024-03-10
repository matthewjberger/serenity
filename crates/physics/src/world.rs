#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhysicsWorld {
    pub gravity: nalgebra_glm::Vec3,
    pub graph: petgraph::Graph<Node, ()>,
    pub isometries: Vec<Isometry>,
    pub rigid_bodies: Vec<RigidBody>,
    pub bounding_boxes: Vec<AxisAlignedBoundingBox>,
}

#[derive(Default, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Isometry {
    pub position: nalgebra_glm::Vec3,
    pub rotation: nalgebra_glm::Quat,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub isometry_index: usize,
    pub rigid_body_index: Option<usize>,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RigidBody {
    pub orientation: nalgebra_glm::Quat,
    pub inverse_inertia_tensor: nalgebra_glm::Mat3,
    pub inverse_mass: f32,
    pub linear_damping: f32,
}

#[derive(Default, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct AxisAlignedBoundingBox {
    pub min: nalgebra_glm::Vec3,
    pub max: nalgebra_glm::Vec3,
}
