#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct World {
    pub gravity: nalgebra_glm::Vec3,
    pub bodies: Vec<RigidBody>,
    pub positions: Vec<nalgebra_glm::Vec3>,
    pub velocities: Vec<nalgebra_glm::Vec3>,
    pub forces: Vec<nalgebra_glm::Vec3>,
    pub masses: Vec<f32>,
}

impl Default for World {
    fn default() -> Self {
        Self {
            gravity: nalgebra_glm::vec3(0.0, -2.8, 0.0),
            positions: Vec::new(),
            velocities: Vec::new(),
            forces: Vec::new(),
            masses: Vec::new(),
            bodies: Vec::new(),
        }
    }
}

impl World {
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
        });
        node_index
    }

    pub fn step(&mut self, delta_time: f32) {
        // Integrate bodies
        self.bodies.iter().for_each(|body| {
            let force = self.forces[body.force_index];
            let mass = self.masses[body.mass_index];
            let acceleration = force / mass;

            let acceleration = acceleration + self.gravity;

            let velocity = self.velocities[body.velocity_index];
            let position = self.positions[body.position_index];

            let new_velocity = velocity + acceleration * delta_time;
            let new_position = position + velocity * delta_time;

            self.velocities[body.velocity_index] = new_velocity;
            self.positions[body.position_index] = new_position;

            self.forces[body.force_index] = nalgebra_glm::vec3(0.0, 0.0, 0.0);
        });
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RigidBody {
    pub position_index: usize,
    pub velocity_index: usize,
    pub force_index: usize,
    pub mass_index: usize,
}