//! The title screen's 3D scene: a striped sunset sun over an endless neon
//! grid, rendered by its own HDR camera with bloom on a separate render layer.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

const LAYER: usize = 1;
const GRID_SPACING: f32 = 4.0;
const GRID_HALF_WIDTH: f32 = 120.0;
const GRID_DEPTH: f32 = 240.0;
const GRID_LINE: f32 = 0.06;
const GRID_SPEED: f32 = 6.0; // m/s towards the viewer
const SUN_RADIUS: f32 = 70.0;
const SUN_DISTANCE: f32 = 330.0;
const SUN_SLICES: usize = 48;
/// Bloom only kicks in above 1.0 in HDR; neon needs to be "brighter than white".
const NEON_INTENSITY: f32 = 4.0;
const SUN_INTENSITY: f32 = 1.3;

#[derive(Component)]
pub struct MenuCamera;

#[derive(Component)]
struct Grid;

pub struct BackdropPlugin;

impl Plugin for BackdropPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_backdrop).add_systems(Update, scroll_grid);
    }
}

fn spawn_backdrop(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let layer = RenderLayers::layer(LAYER);
    commands.spawn((
        MenuCamera,
        Camera3d::default(),
        Camera { is_active: false, order: 1, clear_color: ClearColorConfig::Custom(crate::style::NIGHT), ..default() },
        bevy::camera::Hdr,
        Bloom { intensity: 0.35, ..Bloom::OLD_SCHOOL },
        DistanceFog { color: Color::srgb(0.16, 0.02, 0.22), falloff: FogFalloff::Linear { start: 60.0, end: GRID_DEPTH }, ..default() },
        Transform::from_xyz(0.0, 3.0, 8.0).looking_at(Vec3::new(0.0, 6.0, -SUN_DISTANCE), Vec3::Y),
        layer.clone(),
    ));
    let unlit_vertex_colours = materials.add(StandardMaterial { unlit: true, fog_enabled: false, ..default() });
    commands.spawn((Mesh3d(meshes.add(sky_mesh())), MeshMaterial3d(unlit_vertex_colours.clone()), layer.clone()));
    commands.spawn((Mesh3d(meshes.add(sun_mesh())), MeshMaterial3d(unlit_vertex_colours), Transform::from_xyz(0.0, 4.0, -SUN_DISTANCE), layer.clone()));
    let neon = materials.add(StandardMaterial { base_color: Color::BLACK, emissive: LinearRgba::rgb(1.0, 0.17, 0.84) * NEON_INTENSITY, ..default() });
    commands.spawn((Grid, Mesh3d(meshes.add(grid_mesh())), MeshMaterial3d(neon), Transform::default(), Visibility::default(), layer));
}

/// The floor scrolls one cell, then snaps back: an endless grid from a finite one.
fn scroll_grid(time: Res<Time<Real>>, mut grid: Query<&mut Transform, With<Grid>>) {
    for mut transform in &mut grid {
        transform.translation.z = (transform.translation.z + GRID_SPEED * time.delta_secs()) % GRID_SPACING;
    }
}

/// Horizontal bands of the sun's disc, top to bottom: (y_top, y_bottom) in
/// units of radius. The top half is solid; below the middle, ever-wider gaps
/// cut it into stripes.
pub fn sun_bands(slices: usize) -> Vec<(f32, f32)> {
    let step = 2.0 / slices as f32;
    let mut bands = Vec::new();
    for k in 0..slices {
        let top = 1.0 - k as f32 * step;
        let bottom = top - step;
        // Integer test for the midline: float rounding would misfile the last band.
        if (k + 1) * 2 <= slices {
            bands.push((top, bottom.max(0.0)));
            continue;
        }
        let depth = -bottom; // 0 at the middle, 1 at the bottom
        let gap = step * (0.15 + 0.7 * depth);
        if k % 2 == 0 && top - gap > bottom {
            bands.push((top, bottom + gap));
        }
    }
    bands
}

fn sun_mesh() -> Mesh {
    let (mut positions, mut colors, mut indices) = (Vec::new(), Vec::new(), Vec::new());
    let colour_at = |y: f32| {
        let t = (1.0 - y) / 2.0; // 0 top, 1 bottom
        let (top, bottom) = (LinearRgba::rgb(1.0, 0.62, 0.08), LinearRgba::rgb(1.0, 0.04, 0.42));
        (top * (1.0 - t) + bottom * t) * SUN_INTENSITY
    };
    for (top, bottom) in sun_bands(SUN_SLICES) {
        let base = positions.len() as u32;
        for y in [top, bottom] {
            let half = (1.0 - y * y).max(0.0).sqrt();
            positions.extend([[-half * SUN_RADIUS, y * SUN_RADIUS, 0.0], [half * SUN_RADIUS, y * SUN_RADIUS, 0.0]]);
            colors.extend([colour_at(y).to_f32_array(); 2]);
        }
        indices.extend([base, base + 2, base + 1, base + 1, base + 2, base + 3]);
    }
    coloured_mesh(positions, colors, indices)
}

fn sky_mesh() -> Mesh {
    let (w, z) = (900.0, -SUN_DISTANCE - 20.0);
    let rows = [(260.0, crate::style::NIGHT), (60.0, crate::style::DUSK), (0.0, Color::srgb(0.75, 0.12, 0.45)), (-40.0, crate::style::NIGHT)];
    let (mut positions, mut colors, mut indices) = (Vec::new(), Vec::new(), Vec::new());
    for (i, (y, colour)) in rows.iter().enumerate() {
        positions.extend([[-w, *y, z], [w, *y, z]]);
        colors.extend([colour.to_linear().to_f32_array(); 2]);
        if i > 0 {
            let b = (i as u32 - 1) * 2;
            indices.extend([b, b + 2, b + 1, b + 1, b + 2, b + 3]);
        }
    }
    coloured_mesh(positions, colors, indices)
}

/// Thin quads on the ground: lines across (x) and lines receding (z).
fn grid_mesh() -> Mesh {
    let (mut positions, mut indices) = (Vec::new(), Vec::new());
    let mut quad = |x0: f32, z0: f32, x1: f32, z1: f32| {
        let b = positions.len() as u32;
        positions.extend([[x0, 0.0, z0], [x1, 0.0, z0], [x0, 0.0, z1], [x1, 0.0, z1]]);
        indices.extend([b, b + 2, b + 1, b + 1, b + 2, b + 3]);
    };
    let mut z = GRID_SPACING;
    while z > -GRID_DEPTH {
        quad(-GRID_HALF_WIDTH, z - GRID_LINE, GRID_HALF_WIDTH, z + GRID_LINE);
        z -= GRID_SPACING;
    }
    let mut x = -GRID_HALF_WIDTH;
    while x <= GRID_HALF_WIDTH {
        quad(x - GRID_LINE, -GRID_DEPTH, x + GRID_LINE, GRID_SPACING);
        x += GRID_SPACING;
    }
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_indices(Indices::U32(indices))
}

fn coloured_mesh(positions: Vec<[f32; 3]>, colors: Vec<[f32; 4]>, indices: Vec<u32>) -> Mesh {
    let normals = vec![[0.0, 0.0, 1.0]; positions.len()];
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_indices(Indices::U32(indices))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sun_top_half_is_solid() {
        let bands = sun_bands(48);
        let top_half: f32 = bands.iter().filter(|(_, b)| *b >= 0.0).map(|(t, b)| t - b).sum();
        assert!((top_half - 1.0).abs() < 1e-4);
    }

    #[test]
    fn sun_stripes_thin_out_towards_the_bottom() {
        let stripes: Vec<f32> = sun_bands(48).iter().filter(|(_, b)| *b < 0.0).map(|(t, b)| t - b).collect();
        assert!(stripes.len() > 4);
        assert!(stripes.windows(2).all(|w| w[1] <= w[0]), "{stripes:?}");
    }
}
