
use bevy::prelude::*;
use bevy::mesh::{Indices, PrimitiveTopology};

/// Creates a wedge/arc mesh with smooth curved edges.
/// `segments` controls how smooth the arc is (more segments = smoother curve).
pub fn wedge(inner: f32, outer: f32, a0: f32, a1: f32) -> Mesh {
    let segments = 32; // Number of segments for the arc
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity((segments + 1) * 2);
    let mut indices: Vec<u32> = Vec::with_capacity(segments * 6);
    
    // Generate vertices along the arc
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let angle = a0 + (a1 - a0) * t;
        let (sin, cos) = angle.sin_cos();
        
        // Inner vertex
        positions.push([cos * inner, sin * inner, 0.0]);
        // Outer vertex
        positions.push([cos * outer, sin * outer, 0.0]);
    }
    
    // Generate triangles
    for i in 0..segments {
        let base = (i * 2) as u32;
        // First triangle
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 3);
        // Second triangle
        indices.push(base);
        indices.push(base + 3);
        indices.push(base + 2);
    }
    
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
