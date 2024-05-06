pub fn global_transform(
    nodes: &[Node],
    transforms: &[Transform],
    scenegraph: &SceneGraph,
    graph_node_index: petgraph::graph::NodeIndex,
) -> nalgebra_glm::Mat4 {
    let node_index = scenegraph[graph_node_index];
    let transform_index = nodes[node_index].transform_index;
    let transform = transforms[transform_index].matrix();
    match scenegraph
        .neighbors_directed(graph_node_index, petgraph::Direction::Incoming)
        .next()
    {
        Some(parent_node_index) => {
            global_transform(nodes, transforms, scenegraph, parent_node_index) * transform
        }
        None => transform,
    }
}

pub fn add_node(
    nodes: &mut Vec<Node>,
    metadata: &mut Vec<NodeMetadata>,
    transforms: &mut Vec<Transform>,
) -> usize {
    let transform_index = add_transform(transforms);
    let metadata_index = add_metadata(metadata);
    let node_index = nodes.len();
    let node = crate::asset::Node {
        transform_index,
        metadata_index,
        ..Default::default()
    };
    nodes.push(node);
    node_index
}

pub fn add_metadata(metadata: &mut Vec<NodeMetadata>) -> usize {
    let metadata_index = metadata.len();
    metadata.push(crate::asset::NodeMetadata::default());
    metadata_index
}

pub fn add_transform(transforms: &mut Vec<Transform>) -> usize {
    let transform_index = transforms.len();
    transforms.push(crate::asset::Transform::default());
    transform_index
}

pub fn add_main_camera_to_scenegraph(
    scenes: &mut [Scene],
    metadata: &mut Vec<NodeMetadata>,
    nodes: &mut Vec<Node>,
    cameras: &mut Vec<Camera>,
    orientations: &mut Vec<Orientation>,
    transforms: &mut Vec<Transform>,
    scene_index: usize,
) {
    let node_index = add_node(nodes, metadata, transforms);
    add_camera_to_node(nodes, transforms, orientations, cameras, node_index);
    let camera_graph_node_index = add_child_node_to_scenegraph(
        scenes,
        scene_index,
        petgraph::graph::NodeIndex::new(0),
        node_index,
    );
    scenes[scene_index].default_camera_graph_node_index = Some(camera_graph_node_index);
    let node = &nodes[node_index];
    let metadata = &mut metadata[node.metadata_index];
    metadata.name = "Main Camera".to_string();
}

pub fn add_camera(cameras: &mut Vec<Camera>) -> usize {
    let camera_index = cameras.len();
    cameras.push(crate::asset::Camera::default());
    camera_index
}

pub fn add_camera_to_node(
    nodes: &mut [Node],
    transforms: &mut [Transform],
    orientations: &mut Vec<Orientation>,
    cameras: &mut Vec<Camera>,
    node_index: usize,
) -> usize {
    let orientation_index = add_orientation_to_node(nodes, orientations, node_index);
    let camera_index = add_camera(cameras);

    let node = &mut nodes[node_index];
    let transform = &mut transforms[node.transform_index];
    let orientation = &orientations[orientation_index];
    transform.translation = orientation.position();
    transform.rotation = orientation.look_at_offset();

    node.camera_index = Some(camera_index);
    camera_index
}

pub fn add_orientation_to_node(
    nodes: &mut [Node],
    orientations: &mut Vec<Orientation>,
    node_index: usize,
) -> usize {
    let orientation_index = add_orientation(orientations);
    nodes[node_index].orientation_index = Some(orientation_index);
    orientation_index
}

pub fn add_orientation(orientations: &mut Vec<Orientation>) -> usize {
    let orientation_index = orientations.len();
    orientations.push(crate::asset::Orientation::default());
    orientation_index
}

pub fn add_child_node_to_scenegraph(
    scenes: &mut [Scene],
    scene_index: usize,
    parent_graph_node_index: petgraph::graph::NodeIndex,
    node_index: usize,
) -> petgraph::graph::NodeIndex {
    let scene = &mut scenes[scene_index];
    let graph_node_index = scene.graph.add_node(node_index);
    scene
        .graph
        .add_edge(parent_graph_node_index, graph_node_index, ());
    graph_node_index
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Asset {
    pub name: String,
    pub physics: crate::physics::PhysicsWorld,
    pub scenes: Vec<Scene>,
    // Resources
    pub animations: Vec<Animation>,
    pub images: Vec<Image>,
    pub indices: Vec<u32>,
    pub instances: Vec<Instance>,
    pub materials: Vec<Material>,
    pub meshes: Vec<Mesh>,
    pub samplers: Vec<Sampler>,
    pub skins: Vec<Skin>,
    pub textures: Vec<Texture>,
    pub vertices: Vec<Vertex>,
    // Components
    pub cameras: crate::genvec::GenerationalVec<Camera>,
    pub lights: crate::genvec::GenerationalVec<Light>,
    pub metadata: crate::genvec::GenerationalVec<NodeMetadata>,
    pub orientations: crate::genvec::GenerationalVec<Orientation>,
    pub transforms: crate::genvec::GenerationalVec<Transform>,
    // Entities
    pub entity_allocator: crate::genvec::HandleAllocator,
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

pub type SceneGraph = petgraph::Graph<usize, ()>;

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Scene {
    pub default_camera_graph_node_index: Option<petgraph::graph::NodeIndex>,
    pub graph: SceneGraph,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Mesh {
    pub primitives: Vec<Primitive>,
}

#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize, bytemuck::Zeroable)]
pub struct Instance {
    pub transform: Transform,
}

#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize, bytemuck::Zeroable)]
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

pub fn decompose_matrix(
    matrix: &nalgebra_glm::Mat4,
) -> (nalgebra_glm::Vec3, nalgebra_glm::Quat, nalgebra_glm::Vec3) {
    let translation = nalgebra_glm::Vec3::new(matrix.m14, matrix.m24, matrix.m34);

    let (scale_x, scale_y, scale_z) = (
        nalgebra_glm::length(&nalgebra_glm::Vec3::new(matrix.m11, matrix.m12, matrix.m13)),
        nalgebra_glm::length(&nalgebra_glm::Vec3::new(matrix.m21, matrix.m22, matrix.m23)),
        nalgebra_glm::length(&nalgebra_glm::Vec3::new(matrix.m31, matrix.m32, matrix.m33)),
    );

    let scale = nalgebra_glm::Vec3::new(scale_x, scale_y, scale_z);

    // Normalize the matrix to extract rotation
    let rotation_matrix = nalgebra_glm::mat3(
        matrix.m11 / scale_x,
        matrix.m12 / scale_y,
        matrix.m13 / scale_z,
        matrix.m21 / scale_x,
        matrix.m22 / scale_y,
        matrix.m23 / scale_z,
        matrix.m31 / scale_x,
        matrix.m32 / scale_y,
        matrix.m33 / scale_z,
    );

    let rotation = nalgebra_glm::mat3_to_quat(&rotation_matrix);

    (translation, rotation, scale)
}

impl From<nalgebra_glm::Mat4> for Transform {
    fn from(matrix: nalgebra_glm::Mat4) -> Self {
        let (translation, rotation, scale) = decompose_matrix(&matrix);
        Self {
            translation,
            rotation,
            scale,
        }
    }
}

impl Transform {
    pub fn matrix(&self) -> nalgebra_glm::Mat4 {
        nalgebra_glm::Mat4::from(*self)
    }
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Camera {
    pub projection: Projection,
}

impl Camera {
    pub fn projection_matrix(&self, aspect_ratio: f32) -> nalgebra_glm::Mat4 {
        match &self.projection {
            Projection::Perspective(camera) => camera.matrix(aspect_ratio),
            Projection::Orthographic(camera) => camera.matrix(),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub enum Projection {
    Perspective(PerspectiveCamera),
    Orthographic(OrthographicCamera),
}

impl Default for Projection {
    fn default() -> Self {
        Self::Perspective(PerspectiveCamera::default())
    }
}

pub fn create_camera_matrices(
    asset: &crate::asset::Asset,
    scene: &crate::asset::Scene,
    aspect_ratio: f32,
) -> (nalgebra_glm::Vec3, nalgebra_glm::Mat4, nalgebra_glm::Mat4) {
    let camera_graph_node_index = scene
        .default_camera_graph_node_index
        .expect("No default camera is configured for the scene!");
    let camera_node_index = scene.graph[camera_graph_node_index];
    let camera_node = &asset.nodes[camera_node_index];
    let camera = &asset.cameras[camera_node
        .camera_index
        .expect("Every scene requires a camera")];
    let transform = Transform::from(global_transform(
        &asset.nodes,
        &asset.transforms,
        &scene.graph,
        camera_graph_node_index,
    ));
    (
        transform.translation,
        camera.projection_matrix(aspect_ratio),
        {
            let eye = transform.translation;
            let target = eye
                + nalgebra_glm::quat_rotate_vec3(
                    &transform.rotation.normalize(),
                    &(-nalgebra_glm::Vec3::z()),
                );
            let up = nalgebra_glm::quat_rotate_vec3(
                &transform.rotation.normalize(),
                &nalgebra_glm::Vec3::y(),
            );
            nalgebra_glm::look_at(&eye, &target, &up)
        },
    )
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct PerspectiveCamera {
    pub aspect_ratio: Option<f32>,
    pub y_fov_rad: f32,
    pub z_far: Option<f32>,
    pub z_near: f32,
}
impl Default for PerspectiveCamera {
    fn default() -> Self {
        Self {
            aspect_ratio: None,
            y_fov_rad: 90_f32.to_radians(),
            z_far: None,
            z_near: 0.01,
        }
    }
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

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Orientation {
    pub min_radius: f32,
    pub max_radius: f32,
    pub radius: f32,
    pub offset: nalgebra_glm::Vec3,
    pub sensitivity: nalgebra_glm::Vec2,
    pub direction: nalgebra_glm::Vec2,
}

impl Orientation {
    pub fn direction(&self) -> nalgebra_glm::Vec3 {
        nalgebra_glm::vec3(
            self.direction.y.sin() * self.direction.x.sin(),
            self.direction.y.cos(),
            self.direction.y.sin() * self.direction.x.cos(),
        )
    }

    pub fn rotate(&mut self, position_delta: &nalgebra_glm::Vec2) {
        let delta = position_delta.component_mul(&self.sensitivity);
        self.direction.x += delta.x;
        self.direction.y = nalgebra_glm::clamp_scalar(
            self.direction.y + delta.y,
            10.0_f32.to_radians(),
            170.0_f32.to_radians(),
        );
    }

    pub fn up(&self) -> nalgebra_glm::Vec3 {
        self.right().cross(&self.direction())
    }

    pub fn right(&self) -> nalgebra_glm::Vec3 {
        self.direction().cross(&nalgebra_glm::Vec3::y()).normalize()
    }

    pub fn pan(&mut self, offset: &nalgebra_glm::Vec2) {
        self.offset += self.right() * offset.x;
        self.offset += self.up() * offset.y;
    }

    pub fn position(&self) -> nalgebra_glm::Vec3 {
        (self.direction() * self.radius) + self.offset
    }

    pub fn zoom(&mut self, distance: f32) {
        self.radius -= distance;
        if self.radius < self.min_radius {
            self.radius = self.min_radius;
        }
        if self.radius > self.max_radius {
            self.radius = self.max_radius;
        }
    }

    pub fn look_at_offset(&self) -> nalgebra_glm::Quat {
        self.look(self.offset - self.position())
    }

    pub fn look_forward(&self) -> nalgebra_glm::Quat {
        self.look(-self.direction())
    }

    fn look(&self, point: nalgebra_glm::Vec3) -> nalgebra_glm::Quat {
        nalgebra_glm::quat_conjugate(&nalgebra_glm::quat_look_at(
            &point,
            &nalgebra_glm::Vec3::y(),
        ))
    }
}

impl Default for Orientation {
    fn default() -> Self {
        Self {
            min_radius: 1.0,
            max_radius: 100.0,
            radius: 5.0,
            offset: nalgebra_glm::vec3(0.0, 0.0, 0.0),
            sensitivity: nalgebra_glm::vec2(1.0, 1.0),
            direction: nalgebra_glm::Vec2::new(0.0, 1.0),
        }
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PrimitiveTopology {
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

#[derive(Default, Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct Light {
    pub intensity: f32,
    pub range: f32,
    pub color: nalgebra_glm::Vec3,
    pub kind: LightKind,
}

#[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub enum LightKind {
    Directional,
    Point,
    Spot {
        inner_cone_angle: f32,
        outer_cone_angle: f32,
    },
}

impl Default for LightKind {
    fn default() -> Self {
        Self::Directional
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Texture {
    pub image_index: usize,
    pub sampler_index: Option<usize>,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Primitive {
    pub vertex_offset: usize,
    pub index_offset: usize,
    pub number_of_vertices: usize,
    pub number_of_indices: usize,
    pub topology: PrimitiveTopology,
    pub material_index: Option<usize>,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub metadata_index: usize,
    pub transform_index: usize,
    pub camera_index: Option<usize>,
    pub orientation_index: Option<usize>,
    pub mesh_index: Option<usize>,
    pub light_index: Option<usize>,
    pub rigid_body_index: Option<usize>,
    pub primitive_mesh_index: Option<usize>,
    pub instances: Vec<usize>,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeMetadata {
    pub name: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Image {
    pub pixels: Vec<u8>,
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ImageFormat {
    R8,
    R8G8,
    R8G8B8,
    R8G8B8A8,
    B8G8R8,
    B8G8R8A8,
    R16,
    R16G16,
    R16G16B16,
    R16G16B16A16,
    R16F,
    R16G16F,
    R16G16B16F,
    R16G16B16A16F,
    R32,
    R32G32,
    R32G32B32,
    R32G32B32A32,
    R32F,
    R32G32F,
    R32G32B32F,
    R32G32B32A32F,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Sampler {
    pub min_filter: MinFilter,
    pub mag_filter: MagFilter,
    pub wrap_s: WrappingMode,
    pub wrap_t: WrappingMode,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum WrappingMode {
    ClampToEdge,
    MirroredRepeat,
    #[default]
    Repeat,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum MagFilter {
    Nearest = 1,
    #[default]
    Linear,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum MinFilter {
    Nearest = 1,
    #[default]
    Linear,
    NearestMipmapNearest,
    LinearMipmapNearest,
    NearestMipmapLinear,
    LinearMipmapLinear,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Material {
    pub base_color_factor: nalgebra_glm::Vec4,
    pub base_color_texture_index: usize,
    pub emissive_texture_index: usize,
    pub emissive_factor: nalgebra_glm::Vec3,
    pub alpha_mode: AlphaMode,
    pub alpha_cutoff: Option<f32>,
}

#[derive(Default, Copy, Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum AlphaMode {
    #[default]
    Opaque,
    Mask,
    Blend,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Animation {
    pub time: f32,
    pub channels: Vec<Channel>,
    pub max_animation_time: f32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Channel {
    pub target_node_index: usize,
    pub inputs: Vec<f32>,
    pub transformations: TransformationSet,
    pub interpolation: Interpolation,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Interpolation {
    #[default]
    Linear,
    Step,
    CubicSpline,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TransformationSet {
    Translations(Vec<nalgebra_glm::Vec3>),
    Rotations(Vec<nalgebra_glm::Vec4>),
    Scales(Vec<nalgebra_glm::Vec3>),
    MorphTargetWeights(Vec<f32>),
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Skin {
    pub joints: Vec<Joint>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Joint {
    pub target_node_index: usize,
    pub inverse_bind_matrix: nalgebra_glm::Mat4,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_transform() {
        let nodes = vec![
            Node {
                transform_index: 0,
                ..Default::default()
            },
            Node {
                transform_index: 1,
                ..Default::default()
            },
        ];
        let transforms = vec![
            Transform {
                translation: nalgebra_glm::vec3(1.0, 0.0, 0.0),
                ..Default::default()
            },
            Transform {
                translation: nalgebra_glm::vec3(0.0, 2.0, 0.0),
                ..Default::default()
            },
        ];
        let mut scenegraph: SceneGraph = SceneGraph::new();
        let root_node_index = scenegraph.add_node(0);
        let child_node_index = scenegraph.add_node(1);
        scenegraph.add_edge(root_node_index, child_node_index, ());

        let global_transform_root =
            global_transform(&nodes, &transforms, &scenegraph, root_node_index);
        let global_transform_child =
            global_transform(&nodes, &transforms, &scenegraph, child_node_index);

        assert_eq!(
            global_transform_root,
            nalgebra_glm::translation(&nalgebra_glm::vec3(1.0, 0.0, 0.0))
        );
        assert_eq!(
            global_transform_child,
            nalgebra_glm::translation(&nalgebra_glm::vec3(1.0, 0.0, 0.0))
                * nalgebra_glm::translation(&nalgebra_glm::vec3(0.0, 2.0, 0.0))
        );
    }

    #[test]
    fn test_add_node() {
        let mut nodes = vec![];
        let mut metadata = vec![];
        let mut transforms = vec![];

        let node_index = add_node(&mut nodes, &mut metadata, &mut transforms);
        assert_eq!(nodes.len(), 1);
        assert_eq!(metadata.len(), 1);
        assert_eq!(transforms.len(), 1);
        assert_eq!(node_index, 0);
    }

    #[test]
    fn test_add_metadata() {
        let mut metadata = vec![];
        let metadata_index = add_metadata(&mut metadata);
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata_index, 0);
    }

    #[test]
    fn test_add_transform() {
        let mut transforms = vec![];
        let transform_index = add_transform(&mut transforms);
        assert_eq!(transforms.len(), 1);
        assert_eq!(transform_index, 0);
    }

    #[test]
    fn test_add_main_camera_to_scenegraph() {
        let mut scenes = vec![Scene::default()];
        let mut metadata = vec![];
        let mut nodes = vec![];
        let mut cameras = vec![];
        let mut orientations = vec![];
        let mut transforms = vec![];

        add_main_camera_to_scenegraph(
            &mut scenes,
            &mut metadata,
            &mut nodes,
            &mut cameras,
            &mut orientations,
            &mut transforms,
            0,
        );

        assert_eq!(nodes.len(), 1);
        assert_eq!(metadata.len(), 1);
        assert_eq!(cameras.len(), 1);
        assert_eq!(orientations.len(), 1);
        assert_eq!(transforms.len(), 1);
        assert!(scenes[0].default_camera_graph_node_index.is_some());
    }

    #[test]
    fn test_add_camera() {
        let mut cameras = vec![];
        let camera_index = add_camera(&mut cameras);
        assert_eq!(cameras.len(), 1);
        assert_eq!(camera_index, 0);
    }

    #[test]
    fn test_add_camera_to_node() {
        let mut nodes = vec![Node::default()];
        let mut transforms = vec![Transform::default()];
        let mut orientations = vec![];
        let mut cameras = vec![];

        let camera_index = add_camera_to_node(
            &mut nodes,
            &mut transforms,
            &mut orientations,
            &mut cameras,
            0,
        );
        assert_eq!(orientations.len(), 1);
        assert_eq!(cameras.len(), 1);
        assert_eq!(camera_index, 0);
        assert_eq!(nodes[0].camera_index, Some(0));
    }

    #[test]
    fn test_add_orientation_to_node() {
        let mut nodes = vec![Node::default()];
        let mut orientations = vec![];

        let orientation_index = add_orientation_to_node(&mut nodes, &mut orientations, 0);
        assert_eq!(orientations.len(), 1);
        assert_eq!(orientation_index, 0);
        assert_eq!(nodes[0].orientation_index, Some(0));
    }

    #[test]
    fn test_add_orientation() {
        let mut orientations = vec![];
        let orientation_index = add_orientation(&mut orientations);
        assert_eq!(orientations.len(), 1);
        assert_eq!(orientation_index, 0);
    }
}
