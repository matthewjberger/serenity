#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhysicsWorld {
    pub gravity: nalgebra_glm::Vec3,
    pub bodies: Vec<RigidBody>,
    pub aabbs: Vec<AxisAlignedBoundingBox>,
    pub positions: Vec<nalgebra_glm::Vec3>,
    pub velocities: Vec<nalgebra_glm::Vec3>,
    pub forces: Vec<nalgebra_glm::Vec3>,
    pub masses: Vec<f32>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self {
            gravity: nalgebra_glm::vec3(0.0, -2.8, 0.0),
            positions: Vec::new(),
            velocities: Vec::new(),
            forces: Vec::new(),
            masses: Vec::new(),
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

    pub fn add_rigid_body(&mut self, position: nalgebra_glm::Vec3) -> usize {
        let position_index = self.positions.len();
        self.positions.push(position);

        let velocity_index = self.velocities.len();
        self.velocities.push(nalgebra_glm::vec3(0.0, 0.0, 0.0));

        let force_index = self.forces.len();
        self.forces.push(nalgebra_glm::vec3(0.0, 0.0, 0.0));

        let mass_index = self.masses.len();
        self.masses.push(1.0);

        let aabb_index = self.aabbs.len();
        self.aabbs.push(AxisAlignedBoundingBox::default());

        let node_index = self.bodies.len();
        self.bodies.push(RigidBody {
            position_index,
            velocity_index,
            force_index,
            mass_index,
            aabb_index,
            dynamic: true,
        });
        node_index
    }

    pub fn step(&mut self, delta_time: f32) {
        for index_a in 0..self.bodies.iter().len() {
            for index_b in 0..self.bodies.iter().len() {
                if index_a == index_b {
                    continue;
                }

                let aabb_index_a = self.bodies[index_a].aabb_index;
                let aabb_index_b = self.bodies[index_b].aabb_index;
                let aabb_a = &self.aabbs[aabb_index_a];
                let aabb_b = &self.aabbs[aabb_index_b];

                let position_index_a = self.bodies[index_a].position_index;
                let position_index_b = self.bodies[index_b].position_index;
                let position_a = &self.positions[position_index_a];
                let position_b = &self.positions[position_index_b];

                let min_a = aabb_a.min + position_a;
                let max_a = aabb_a.max + position_a;

                let min_b = aabb_b.min + position_b;
                let max_b = aabb_b.max + position_b;

                let overlap_x = min_a.x <= max_b.x && max_a.x >= min_b.x;
                let overlap_y = min_a.y <= max_b.y && max_a.y >= min_b.y;
                let overlap_z = min_a.z <= max_b.z && max_a.z >= min_b.z;

                if overlap_x && overlap_y && overlap_z {
                    return;
                }
            }
        }

        // Integrate bodies
        self.bodies.iter().for_each(|body| {
            if body.dynamic == false {
                return;
            }

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
    pub dynamic: bool,
    pub position_index: usize,
    pub velocity_index: usize,
    pub force_index: usize,
    pub mass_index: usize,
    pub aabb_index: usize,
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

    pub fn from_vertices(vertices: &[crate::world::Vertex]) -> Self {
        let mut min = vertices[0].position;
        let mut max = vertices[0].position;

        for point in vertices.iter().skip(1) {
            min = nalgebra_glm::min2(&min, &point.position);
            max = nalgebra_glm::max2(&max, &point.position);
        }

        Self { min, max }
    }

    pub fn expand_to_include(&mut self, other: &AxisAlignedBoundingBox) {
        self.min = nalgebra_glm::min2(&self.min, &other.min);
        self.max = nalgebra_glm::max2(&self.max, &other.max);
    }
}
