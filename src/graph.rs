use crate::transform::Transform;
use nalgebra_glm as glm;
use petgraph::Graph;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    path::Path,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    // #[error("Failed to read animation channel outputs!")]
    // ReadChannelInputs,

    // #[error("Failed to read animation channel outputs!")]
    // ReadChannelOutputs,

    // #[error("Failed to lookup sampler specified by texture!")]
    // LookupSampler,

    // #[error("Failed to lookup image specified by texture!")]
    // LookupImage,

    // #[error("No primitive vertex positions for a primitive in the model.")]
    // ReadVertexPositions,
    #[error("Failed to import gltf asset!")]
    ImportGltfAsset(#[source] gltf::Error),
    // #[error("Failed to access entity!")]
    // AccessEntity(#[from] EntityAccessError),

    // #[error("Failed to get transform!")]
    // GetComponent(#[from] ComponentError),

    // #[error("Failed to create texture!")]
    // CreateTexture(#[from] TextureError),
}

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SceneGraph(Graph<Node, ()>);

impl Deref for SceneGraph {
    type Target = Graph<Node, ()>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SceneGraph {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl SceneGraph {
    pub fn new() -> Self {
        Self(Graph::new())
    }

    pub fn load_gltf(&mut self, path: impl AsRef<Path>) -> Result<()> {
        // let gltf_bytes = std::fs::read(&path).expect("Failed to load default gltf file!");
        // let _gltf = gltf::Gltf::from_slice(&gltf_bytes).expect("Failed to load GLTF!");
        let (gltf, _buffers, _images) =
            gltf::import(path.as_ref()).map_err(Error::ImportGltfAsset)?;

        log::info!("Loaded gltf");

        for material in gltf.materials() {
            log::info!("Material: {}", material.name().unwrap_or("Unnamed"));
        }

        for texture in gltf.textures() {
            log::info!("Texture: {}", texture.name().unwrap_or("Unnamed"));
        }

        for animation in gltf.animations() {
            log::info!("Animation: {}", animation.name().unwrap_or("Unnamed"));
        }

        for (index, node) in gltf.nodes().enumerate() {
            let name = node.name().unwrap_or("Unnamed");
            log::info!("Node: '{name}'");
            let transform = Transform::from(node.transform().decomposed());

            if let Some(camera) = node.camera() {
                log::info!("\tCamera");
            }

            if let Some(mesh) = node.mesh() {
                log::info!("\tMesh");
            }

            if let Some(skin) = node.skin() {
                log::info!("\tSkin");
            }

            if let Some(light) = node.light() {
                log::info!("\tLight");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node {
    //
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct NodeBase {
//     pub transform: glm::Mat4,
// }
