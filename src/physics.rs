#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhysicsWorld {
    pub gravity: nalgebra_glm::Vec3,
    pub bodies: Vec<RigidBody>,
    pub colliders: Vec<Collider>,
    pub collision_shapes: Vec<CollisionShape>,
    pub positions: Vec<nalgebra_glm::Vec3>,
    pub velocities: Vec<nalgebra_glm::Vec3>,
    pub forces: Vec<nalgebra_glm::Vec3>,
    pub masses: Vec<f32>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self {
            gravity: nalgebra_glm::vec3(0.0, -9.8, 0.0),
            positions: Vec::new(),
            velocities: Vec::new(),
            forces: Vec::new(),
            masses: Vec::new(),
            bodies: Vec::new(),
            colliders: Vec::new(),
            collision_shapes: Vec::new(),
        }
    }
}

impl PhysicsWorld {
    pub fn add_rigid_body(&mut self, position: nalgebra_glm::Vec3) -> usize {
        let position_index = self.positions.len();
        self.positions.push(position);

        let velocity_index = self.velocities.len();
        self.velocities.push(nalgebra_glm::vec3(0.0, 0.0, 0.0));

        let force_index = self.forces.len();
        self.forces.push(nalgebra_glm::vec3(0.0, 0.0, 0.0));

        let mass_index = self.masses.len();
        self.masses.push(1.0);

        let node_index = self.bodies.len();
        self.bodies.push(RigidBody {
            position_index,
            velocity_index,
            force_index,
            mass_index,
            shape_indices: Vec::new(),
        });
        node_index
    }

    pub fn add_collider(&mut self, shapes: &[CollisionShape]) -> usize {
        let shape_indices = shapes
            .iter()
            .cloned()
            .map(|shape| {
                let shape_index = self.collision_shapes.len();
                self.collision_shapes.push(shape);
                shape_index
            })
            .collect::<Vec<_>>();
        let collider_index = self.colliders.len();
        self.colliders.push(Collider { shape_indices });
        collider_index
    }

    pub fn step(&mut self, delta_time: f32) {
        self.bodies.iter().for_each(|node| {
            let force = self.forces[node.force_index];
            let mass = self.masses[node.mass_index];
            let acceleration = force / mass;

            let acceleration = acceleration + self.gravity;

            let velocity = self.velocities[node.velocity_index];
            let position = self.positions[node.position_index];

            let new_velocity = velocity + acceleration * delta_time;
            let new_position = position + velocity * delta_time;

            self.velocities[node.velocity_index] = new_velocity;
            self.positions[node.position_index] = new_position;

            self.forces[node.force_index] = nalgebra_glm::vec3(0.0, 0.0, 0.0);
        });
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RigidBody {
    pub position_index: usize,
    pub velocity_index: usize,
    pub force_index: usize,
    pub mass_index: usize,
    pub shape_indices: Vec<usize>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Collider {
    pub shape_indices: Vec<usize>,
}

#[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub enum CollisionShape {
    /// Axis-Aligned Bounding Box
    AABB(f32, f32, f32, f32),
}
