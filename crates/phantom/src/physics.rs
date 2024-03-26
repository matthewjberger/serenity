#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhysicsWorld {
    pub gravity: nalgebra_glm::Vec3,
    pub bodies: Vec<Particle>,
}

impl PhysicsWorld {
    pub fn integrate(&mut self, duration: f32) {
        self.bodies
            .iter_mut()
            .filter(|particle| particle.inverse_mass > 0.0)
            .for_each(|particle| {
                // Update linear position
                particle.position += particle.velocity * duration;

                // Work out the acceleration from the force
                let mut acceleration = particle.acceleration;
                acceleration += particle.force_accumulator * particle.inverse_mass;

                let drag = duration.powf(particle.damping);

                // Update linear velocity from the acceleration
                particle.velocity += acceleration * duration * drag;

                // Clear any accumulated forces
                particle.force_accumulator = nalgebra_glm::Vec3::zeros();
            });
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Particle {
    pub position: nalgebra_glm::Vec3,
    pub velocity: nalgebra_glm::Vec3,
    pub acceleration: nalgebra_glm::Vec3,
    pub damping: f32,
    pub inverse_mass: f32,
    pub force_accumulator: nalgebra_glm::Vec3,
}
