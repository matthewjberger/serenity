#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhysicsWorld {
    pub gravity: nalgebra_glm::Vec3,
    pub bodies: Vec<RigidBody>,
    pub aabbs: Vec<AxisAlignedBoundingBox>,
    pub isometries: Vec<Isometry>,
    pub velocities: Vec<nalgebra_glm::Vec3>,
    pub forces: Vec<nalgebra_glm::Vec3>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self {
            gravity: nalgebra_glm::vec3(0.0, -2.8, 0.0),
            isometries: Vec::new(),
            velocities: Vec::new(),
            forces: Vec::new(),
            bodies: Vec::new(),
            aabbs: Vec::new(),
        }
    }
}

impl PhysicsWorld {
    pub fn add_aabb(&mut self, aabb: AxisAlignedBoundingBox) -> usize {
        let aabb_index = self.aabbs.len();
        self.aabbs.push(aabb);
        aabb_index
    }

    pub fn add_rigid_body(&mut self, isometry: Isometry) -> usize {
        let isometry_index = self.isometries.len();
        self.isometries.push(isometry);

        let velocity_index = self.velocities.len();
        self.velocities.push(nalgebra_glm::vec3(0.0, 0.0, 0.0));

        let force_index = self.forces.len();
        self.forces.push(nalgebra_glm::vec3(0.0, 0.0, 0.0));

        let node_index = self.bodies.len();
        self.bodies.push(RigidBody {
            isometry_index,
            velocity_index,
            force_index,
            ..Default::default()
        });
        node_index
    }

    pub fn step(&mut self, delta_time: f32) {
        self.bodies.iter().for_each(|body| {
            let force = self.forces[body.force_index];
            let acceleration = force / body.inverse_mass;

            let acceleration = acceleration + self.gravity;

            let velocity = self.velocities[body.velocity_index];
            let isometry = self.isometries[body.isometry_index];

            let new_velocity = velocity + acceleration * delta_time;
            let new_position = isometry.position + velocity * delta_time;

            self.velocities[body.velocity_index] = new_velocity;
            self.isometries[body.isometry_index].position = new_position;
            // TODO: update rotations

            self.forces[body.force_index] = nalgebra_glm::vec3(0.0, 0.0, 0.0);
        });
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RigidBody {
    pub inverse_mass: f32,
    pub linear_damping: f32,
    pub isometry_index: usize,
    pub velocity_index: usize,
    pub force_index: usize,
    pub inverse_mass_index: usize,
}

#[derive(Default, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct AxisAlignedBoundingBox {
    pub min: nalgebra_glm::Vec3,
    pub max: nalgebra_glm::Vec3,
}

impl AxisAlignedBoundingBox {
    pub fn new(min: nalgebra_glm::Vec3, max: nalgebra_glm::Vec3) -> Self {
        Self { min, max }
    }

    pub fn extents(&self) -> nalgebra_glm::Vec3 {
        self.max - self.min
    }

    pub fn center(&self) -> nalgebra_glm::Vec3 {
        (self.min + self.max) / 2.0
    }
}

#[derive(Default, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Isometry {
    pub position: nalgebra_glm::Vec3,
    pub rotation: nalgebra_glm::Quat,
}
