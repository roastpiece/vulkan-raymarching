pub mod fs_raymarching {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/raymarching.frag"
    }
}

pub mod vs_raymarching {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/raymarching.vert"
    }
}