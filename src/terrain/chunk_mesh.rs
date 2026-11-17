//! Geometry for one square terrain tile: a (res+1)^2 grid in tile-local x/z
//! with heights relative to the tile origin, plus a skirt hanging from the
//! border to hide cracks against neighbours at a different level of detail.

use bevy::math::DVec3;

use super::colorizer::terrain_color;
use super::heightfield::Heightfield;
use crate::planet::Palette;

const SKIRT_STEPS: f64 = 2.0; // skirt depth in vertex spacings

pub struct ChunkMeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

/// `origin` is the tile's world corner; vertex positions are relative to it
/// so they stay small enough for f32.
pub fn build_chunk_mesh(heightfield: &Heightfield, palette: &Palette, origin: DVec3, size: f64, res: usize) -> ChunkMeshData {
    let n = res + 1;
    let step = size / res as f64;
    let heights = sample_heights(heightfield, origin, step, n + 2);
    let perimeter = perimeter_indices(res);
    let mut mesh = ChunkMeshData {
        positions: Vec::with_capacity(n * n + perimeter.len()),
        normals: Vec::with_capacity(n * n + perimeter.len()),
        colors: Vec::with_capacity(n * n + perimeter.len()),
        indices: build_indices(res, &perimeter),
    };
    fill_surface(&mut mesh, &heights, heightfield, palette, origin, step, n);
    fill_skirt(&mut mesh, &perimeter, step * SKIRT_STEPS + 1.0);
    mesh
}

// Heights on a grid with a one-sample border so normals are seamless across tiles.
fn sample_heights(heightfield: &Heightfield, origin: DVec3, step: f64, b: usize) -> Vec<f64> {
    let mut heights = Vec::with_capacity(b * b);
    for j in 0..b {
        for i in 0..b {
            let (x, z) = (origin.x + (i as f64 - 1.0) * step, origin.z + (j as f64 - 1.0) * step);
            heights.push(heightfield.height_at(x, z));
        }
    }
    heights
}

fn fill_surface(mesh: &mut ChunkMeshData, heights: &[f64], hf: &Heightfield, palette: &Palette, origin: DVec3, step: f64, n: usize) {
    let b = n + 2;
    for j in 0..n {
        for i in 0..n {
            let h = (j + 1) * b + (i + 1);
            let normal = DVec3::new(heights[h - 1] - heights[h + 1], 2.0 * step, heights[h - b] - heights[h + b]).normalize();
            let (x, z) = (i as f64 * step, j as f64 * step);
            mesh.positions.push([x as f32, (heights[h] - origin.y) as f32, z as f32]);
            mesh.normals.push(normal.as_vec3().to_array());
            mesh.colors.push(terrain_color(palette, hf, origin.x + x, origin.z + z, normal.y));
        }
    }
}

fn fill_skirt(mesh: &mut ChunkMeshData, perimeter: &[u32], drop: f64) {
    for &src in perimeter {
        let [x, y, z] = mesh.positions[src as usize];
        mesh.positions.push([x, y - drop as f32, z]);
        mesh.normals.push(mesh.normals[src as usize]);
        mesh.colors.push(mesh.colors[src as usize]);
    }
}

/// Border vertices as a closed loop: top, right, bottom, left.
fn perimeter_indices(res: usize) -> Vec<u32> {
    let n = res + 1;
    let top = (0..res).map(|i| i);
    let right = (0..res).map(|j| j * n + res);
    let bottom = (1..=res).rev().map(|i| res * n + i);
    let left = (1..=res).rev().map(|j| j * n);
    top.chain(right).chain(bottom).chain(left).map(|i| i as u32).collect()
}

fn build_indices(res: usize, perimeter: &[u32]) -> Vec<u32> {
    let n = res + 1;
    let first_skirt = (n * n) as u32;
    let mut out = Vec::with_capacity(6 * res * res + 6 * perimeter.len());
    for j in 0..res {
        for i in 0..res {
            let a = (j * n + i) as u32;
            let n = n as u32;
            // Counter-clockwise seen from above (+y), so faces point up.
            out.extend([a, a + n, a + 1, a + 1, a + n, a + n + 1]);
        }
    }
    for (k, &top) in perimeter.iter().enumerate() {
        let next = (k + 1) % perimeter.len();
        let (bottom, next_bottom) = (first_skirt + k as u32, first_skirt + next as u32);
        out.extend([top, bottom, perimeter[next], perimeter[next], bottom, next_bottom]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perimeter_is_a_closed_loop_of_border_vertices() {
        let loop_ = perimeter_indices(4);
        assert_eq!(loop_.len(), 16);
        let unique: std::collections::HashSet<_> = loop_.iter().collect();
        assert_eq!(unique.len(), 16);
        let n = 5;
        assert!(loop_.iter().all(|&v| {
            let (i, j) = (v as usize % n, v as usize / n);
            i == 0 || j == 0 || i == 4 || j == 4
        }));
    }

    #[test]
    fn surface_triangles_face_up() {
        let indices = build_indices(1, &perimeter_indices(1));
        // Grid of 2x2 vertices at unit spacing: index -> (x, z).
        let pos = |v: u32| DVec3::new(f64::from(v % 2), 0.0, f64::from(v / 2));
        for tri in indices[..6].chunks(3) {
            let (a, b, c) = (pos(tri[0]), pos(tri[1]), pos(tri[2]));
            assert!((b - a).cross(c - a).y > 0.0);
        }
    }
}
