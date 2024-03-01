# Phantom

`phantom` is a 3D graphics renderer written in rust using [wgpu](https://wgpu.rs/), supporting both [wasm](https://webassembly.org/) and native.

```
# native
cargo run -r -p editor

# web
trunk serve --open --config apps/editor/Trunk.toml
```

## Prerequisites (web)

* [trunk](https://trunkrs.dev/)