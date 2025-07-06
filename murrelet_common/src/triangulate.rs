use glam::{vec2, Vec2, Vec3};

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct VertexSimple {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub face_pos: [f32; 2],
}

impl VertexSimple {
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

#[derive(Debug, Clone)]
pub struct Triangulate {
    pub vertices: Vec<VertexSimple>,
    pub order: Vec<u32>,
}

impl Triangulate {
    pub fn new() -> Self {
        Triangulate {
            vertices: vec![],
            order: vec![],
        }
    }

    pub fn add_many_vertices_and_offset(&mut self, vertices: Vec<VertexSimple>, indices: Vec<u32>) {
        let vertex_offset = self.vertices.len() as u32;
        self.vertices.extend(vertices);
        self.order
            .extend(indices.iter().map(|i| *i + vertex_offset));
    }

    pub fn vertices(&self) -> &[VertexSimple] {
        &self.vertices
    }

    pub fn add_vertex(&mut self, v: [f32; 3], n: [f32; 3], face_pos: [f32; 2]) -> u32 {
        let vv = VertexSimple::new(v, n, face_pos);
        self.add_vertex_simple(vv)
    }

    pub fn add_vertex_simple(&mut self, vv: VertexSimple) -> u32 {
        self.vertices.push(vv);
        (self.vertices.len() - 1) as u32
    }

    pub fn add_tri(&mut self, tri: [u32; 3]) {
        self.order.extend(tri)
    }

    // alternatively can add vertices and then add teh vec
    pub fn add_rect(&mut self, v: &[Vec3; 4], flip: bool) {
        let edge1 = v[0] - v[1];
        let edge2 = v[3] - v[1];
        let normal = edge1.cross(edge2).normalize().to_array();

        let v0 = self.add_vertex(v[0].to_array(), normal, [1.0, 0.0]);
        let v1 = self.add_vertex(v[1].to_array(), normal, [0.0, 0.0]);
        let v2 = self.add_vertex(v[2].to_array(), normal, [1.0, 1.0]);
        let v3 = self.add_vertex(v[3].to_array(), normal, [0.0, 1.0]);

        if !flip {
            self.order.extend([v0, v2, v1, v1, v2, v3])
        } else {
            self.order.extend([v0, v1, v2, v1, v3, v2])
        }
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
