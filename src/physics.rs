use rapier3d::{
    na::Vector3,
    prelude::{
        CCDSolver, ColliderSet, DefaultBroadPhase, ImpulseJointSet, IntegrationParameters,
        IslandManager, MultibodyJointSet, NarrowPhase, PhysicsPipeline, QueryPipeline,
        RigidBodySet,
    },
};

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct PhysicsWorld {
    pub rigid_bodies: RigidBodySet,
    pub colliders: ColliderSet,
    gravity: Vector3<f32>,
    integration_parameters: IntegrationParameters,
    #[serde(skip)]
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self {
            gravity: [0.0, -9.81, 0.0].into(),
            ..Default::default()
        }
    }

    pub fn step(&mut self, delta_time: f32) {
        let Self {
            rigid_bodies,
            colliders,
            gravity,
            integration_parameters,
            physics_pipeline,
            island_manager,
            broad_phase,
            narrow_phase,
            impulse_joint_set,
            multibody_joint_set,
            ccd_solver,
            query_pipeline,
        } = self;
        integration_parameters.dt = delta_time;
        physics_pipeline.step(
            gravity,
            integration_parameters,
            island_manager,
            broad_phase,
            narrow_phase,
            rigid_bodies,
            colliders,
            impulse_joint_set,
            multibody_joint_set,
            ccd_solver,
            Some(query_pipeline),
            &(),
            &(),
        );
    }
}
