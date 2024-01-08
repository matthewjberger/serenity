## Textures

- import textures from gltf and load into the scene
- import material info from gltf and load into the scene
- collect texture data from imported gltf, create texture views, then for bindless put them into a texture array.
- provide offsets into that array based on material indices as a material uniform that we set per primitive
- render each primitive with the correct material
- allow displaying textures in editor
- allow displaying materials in editor

## Editor

- gizmo controls
    - selection of translation, rotation, scaling
    - toggle between global and local transform
    - use global transform for camera nodes
- snapping
- visual grid shader
- save / load maps with a version
- add/remove nodes
- add/remove components
- allow adding mesh nodes where you can select the mesh to use from the list of meshes
    - this would just be changing the id of the mesh used by the node
    - should allow selecting primitives

## Pipelines

- need pipeline for drawing simple primitives with various line modes
  - use this to draw the lines for a bounding box visualization
- postprocessing shader (using clipped fullscreen triangle)

## Collision Shapes

- Add simple collider shape components (capsule, box, sphere)
- Setup mouse raycasting for picking
  - Allow selecting entities by an optional capsule component added for editor,
    add tags to collision shapes for knowing what they can interact with

## Physics


- Setup physics world with those shapes
- Simulate using a fixed timestep and then render meshes at their new transforms

## Animation

- Add animations
- Add skinning


## Textures/GLTF

- each texture in gltf.textures() has a texture.sampler().index()
 - This can be used to index into the flat array of samplers during the import
- Add links between the textures and the samplers

## Meshes

- Mesh draw commands are looked up using a string that is just the name of the node, this should be something more unique
