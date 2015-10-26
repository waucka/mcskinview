
extern crate nalgebra;
#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub texcoord: [f32; 2],
    pub normal: [f32; 3],
}

implement_vertex!(Vertex, position, texcoord, normal);
