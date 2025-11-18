// use glam::{vec2, Vec2, Vec3};
// use lerpable::Lerpable;

use bytemuck::{Pod, Zeroable};
use glam::{vec2, Vec2, Vec3};
use lerpable::Lerpable;

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct DefaultVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub face_pos: [f32; 2],
}

impl DefaultVertex {
    pub fn new(position: [f32; 3], normal: [f32; 3], face_pos: [f32; 2]) -> Self {
        Self {
            position,
            normal,
            face_pos,
        }
    }
    pub fn pos(&self) -> [f32; 3] {
        self.position
    }

    pub fn pos_vec3(&self) -> Vec3 {
        glam::vec3(self.position[0], self.position[1], self.position[2])
    }

    // pub fn from_simple(vs: &VertexSimple) -> Self {
    //     Self {
    //         position: vs.position,
    //         normal: vs.normal,
    //         face_pos: vs.face_pos,
    //     }
    // }

    //     pub fn new(position: [f32; 3], normal: [f32; 3], face_pos: [f32; 2]) -> Self {
    //     Self {
    //         position,
    //         normal,
    //         face_pos,
    //     }
    // }
    // pub fn pos(&self) -> [f32; 3] {
    //     self.position
    // }

    // pub fn pos_vec3(&self) -> Vec3 {
    //     glam::vec3(self.position[0], self.position[1], self.position[2])
    // }

    pub fn pos2d(&self) -> Vec2 {
        vec2(self.position[0], self.position[1])
    }

    pub fn attrs(&self) -> Vec<f32> {
        vec![
            self.normal[0],
            self.normal[1],
            self.normal[2],
            self.face_pos[0],
            self.face_pos[1],
        ]
    }
}

impl Lerpable for DefaultVertex {
    fn lerpify<T: lerpable::IsLerpingMethod>(&self, other: &Self, pct: &T) -> Self {
        DefaultVertex {
            position: [
                self.position[0].lerpify(&other.position[0], pct),
                self.position[1].lerpify(&other.position[1], pct),
                self.position[2].lerpify(&other.position[2], pct),
            ],
            normal: [
                self.normal[0].lerpify(&other.normal[0], pct),
                self.normal[1].lerpify(&other.normal[1], pct),
                self.normal[2].lerpify(&other.normal[2], pct),
            ],
            face_pos: [
                self.face_pos[0].lerpify(&other.face_pos[0], pct),
                self.face_pos[1].lerpify(&other.face_pos[1], pct),
            ],
        }
    }
}

// unsafe impl Zeroable for DefaultVertex {}
// unsafe impl Pod for DefaultVertex {}

// impl Lerpable for VertexSimple {
//     fn lerpify<T: lerpable::IsLerpingMethod>(&self, other: &Self, pct: &T) -> Self {
//         VertexSimple {
//             position: [
//                 self.position[0].lerpify(&other.position[0], pct),
//                 self.position[1].lerpify(&other.position[1], pct),
//                 self.position[2].lerpify(&other.position[2], pct),
//             ],
//             normal: [
//                 self.normal[0].lerpify(&other.normal[0], pct),
//                 self.normal[1].lerpify(&other.normal[1], pct),
//                 self.normal[2].lerpify(&other.normal[2], pct),
//             ],
//             face_pos: [
//                 self.face_pos[0].lerpify(&other.face_pos[0], pct),
//                 self.face_pos[1].lerpify(&other.face_pos[1], pct),
//             ],
//         }
//     }
// }

// #[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
// #[repr(C)]
// pub struct VertexSimple {
//     pub position: [f32; 3],
//     pub normal: [f32; 3],
//     pub face_pos: [f32; 2],
// }

// impl VertexSimple {
// pub fn new(position: [f32; 3], normal: [f32; 3], face_pos: [f32; 2]) -> Self {
//     Self {
//         position,
//         normal,
//         face_pos,
//     }
// }
// pub fn pos(&self) -> [f32; 3] {
//     self.position
// }

// pub fn pos_vec3(&self) -> Vec3 {
//     glam::vec3(self.position[0], self.position[1], self.position[2])
// }

// pub fn pos2d(&self) -> Vec2 {
//     vec2(self.position[0], self.position[1])
// }

// pub fn attrs(&self) -> Vec<f32> {
//     vec![
//         self.normal[0],
//         self.normal[1],
//         self.normal[2],
//         self.face_pos[0],
//         self.face_pos[1],
//     ]
// }
// }

#[derive(Debug, Clone)]
pub struct Triangulate<Vertex> {
    pub vertices: Vec<Vertex>,
    pub order: Vec<u32>,
}

impl<Vertex> Triangulate<Vertex> {
    pub fn new() -> Self {
        Triangulate {
            vertices: vec![],
            order: vec![],
        }
    }

    pub fn new_from_vertices_indices(vertices: Vec<Vertex>, order: Vec<u32>) -> Self {
        Triangulate { vertices, order }
    }

    pub fn add_many_vertices_and_offset(&mut self, vertices: Vec<Vertex>, indices: Vec<u32>) {
        let vertex_offset = self.vertices.len() as u32;
        self.vertices.extend(vertices);
        self.order
            .extend(indices.iter().map(|i| *i + vertex_offset));
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    // pub fn add_vertex(&mut self, v: [f32; 3], n: [f32; 3], face_pos: [f32; 2]) -> u32 {
    //     let vv = VertexSimple::new(v, n, face_pos);
    //     self.add_vertex_simple(vv)
    // }

    pub fn add_vertex_simple(&mut self, vv: Vertex) -> u32 {
        self.vertices.push(vv);
        (self.vertices.len() - 1) as u32
    }

    pub fn add_tri(&mut self, tri: [u32; 3]) {
        self.order.extend(tri)
    }

    // alternatively can add vertices and then add teh vec
    // pub fn add_rect(&mut self, v: &[Vec3; 4], flip: bool) {
    //     let edge1 = v[0] - v[1];
    //     let edge2 = v[3] - v[1];
    //     let normal = edge1.cross(edge2).normalize().to_array();

    //     let v0 = self.add_vertex(v[0].to_array(), normal, [1.0, 0.0]);
    //     let v1 = self.add_vertex(v[1].to_array(), normal, [0.0, 0.0]);
    //     let v2 = self.add_vertex(v[2].to_array(), normal, [1.0, 1.0]);
    //     let v3 = self.add_vertex(v[3].to_array(), normal, [0.0, 1.0]);

    //     if !flip {
    //         self.order.extend([v0, v2, v1, v1, v2, v3])
    //     } else {
    //         self.order.extend([v0, v1, v2, v1, v3, v2])
    //     }
    // }

    pub fn set_order(&mut self, u: Vec<u32>) {
        self.order = u;
    }

    pub fn order(&self) -> &[u32] {
        &self.order
    }

    pub fn indices(&self) -> &[u32] {
        &self.order
    }

    pub fn add_order(&mut self, collect: &[u32]) {
        self.order.extend_from_slice(collect);
    }

    pub fn add_rect_simple(&mut self, ids: &[u32; 4], flip: bool) {
        let [v0, v1, v3, v2] = ids;
        if !flip {
            self.order.extend([v0, v2, v1, v1, v2, v3])
        } else {
            self.order.extend([v0, v1, v2, v1, v3, v2])
        }
    }
}
