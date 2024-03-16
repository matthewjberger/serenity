mod physics;

pub use physics::*;

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceId(pub usize);
