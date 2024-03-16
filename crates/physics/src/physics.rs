pub type Real = f32;

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct World {
    pub particles: Vec<Particle>,
}

impl World {
    /// Integrates the particles forward in time by the given amount.
    /// This function uses a Newton-Euler integration method, which is a
    /// linear approximation to the correct integral. For this reason it
    /// may be inaccurate in some cases.
    pub fn integrate(&mut self, duration: Real) {
        self.particles.iter_mut().for_each(|particle| {
            // Infinite mass should not be integrated
            if particle.inverse_mass <= 0.0 || duration <= 0.0 {
                return;
            }

            // Update linear position
            particle.position += particle.velocity * duration;

            // Update linear velocity from the acceleration
            let acceleration =
                particle.acceleration + particle.force_accumulator * particle.inverse_mass;
            particle.velocity += acceleration * duration;

            // Impose drag
            particle.velocity *= particle.damping.powf(duration);

            // Clear any accumulated forces
            particle.force_accumulator = nalgebra_glm::Vec3::zeros();
        });
    }
}

#[derive(Debug, Default, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Particle {
    /// Holds the linear position of the particle in world space
    pub position: nalgebra_glm::Vec3,

    /// Holds the linear velocity of the particle in world space
    pub velocity: nalgebra_glm::Vec3,

    /// Holds the acceleration of the particle.
    /// This value can be used to set the acceleration
    /// due to gravity (its primary use) or any other constant acceleration.
    pub acceleration: nalgebra_glm::Vec3,

    /// Holds the amount of damping applied to linear
    /// motion. Damping is required to remove energy added
    /// through numerical instability in the integrator.
    pub damping: Real,

    /// Holds the inverse of the mass of the body.
    ///
    /// It is more useful to hold the inverse mass because
    /// integration is simpler, and because in real-time
    /// simulation it is more useful to have objects with
    /// infinite mass (immovable) than zero mass
    /// (completely unstable in numerical simulation).
    pub inverse_mass: Real,

    /// Holds the accumulated force to be applied at the next
    /// simulation iteration only. This value is zeroed at each
    /// integration step.
    pub force_accumulator: nalgebra_glm::Vec3,
}

impl Particle {
    #[must_use]
    pub fn mass(&self) -> Real {
        self.inverse_mass.recip()
    }

    #[must_use]
    pub fn has_finite_mass(&self) -> bool {
        self.inverse_mass != 0.0
    }
}
