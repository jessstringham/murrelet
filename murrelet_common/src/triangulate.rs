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

    pub fn add_vertex_simple(&mut self, vv: Vertex) -> u32 {
        self.vertices.push(vv);
        (self.vertices.len() - 1) as u32
    }

    pub fn add_tri(&mut self, tri: [u32; 3]) {
        self.order.extend(tri)
    }

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
