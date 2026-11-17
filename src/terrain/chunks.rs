//! Quadtree level-of-detail terrain streamed around the player. Tiles build
//! on background threads, coarse first; while a fine tile is missing, its
//! nearest built ancestor stands in, so there are no holes.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::math::DVec3;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};

use super::chunk_mesh::{ChunkMeshData, build_chunk_mesh};
use super::{Terrain, TerrainMaterial};
use crate::world::WorldPosition;

const BASE_CHUNK_SIZE: f64 = 64.0; // metres; finest tile edge (2 m vertex spacing at CHUNK_RES)
const CHUNK_RES: usize = 32; // quads per tile edge
const MAX_LEVEL: u32 = 9; // root tiles are 64 m * 2^9 = ~33 km
const ROOT_RING: i64 = 1; // 3x3 root tiles around the player
const SPLIT_DISTANCE: f64 = 1.0; // split a tile when the player is within one tile-width of it
const MAX_IN_FLIGHT: usize = 24; // background builds at once
const MAX_CACHED: usize = 900;

fn size_of(level: u32) -> f64 {
    BASE_CHUNK_SIZE * f64::from(1u32 << level)
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ChunkKey {
    level: u32,
    ix: i64,
    iz: i64,
}

impl ChunkKey {
    fn origin(self) -> DVec3 {
        let size = size_of(self.level);
        DVec3::new(self.ix as f64 * size, 0.0, self.iz as f64 * size)
    }

    fn children(self) -> [ChunkKey; 4] {
        let (level, x, z) = (self.level - 1, self.ix * 2, self.iz * 2);
        [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(dx, dz)| ChunkKey { level, ix: x + dx, iz: z + dz })
    }

    /// Chebyshev distance from a point to the tile's square footprint.
    fn distance_to(self, px: f64, pz: f64) -> f64 {
        let size = size_of(self.level);
        let half = size / 2.0;
        let dx = (px - (self.ix as f64 + 0.5) * size).abs() - half;
        let dz = (pz - (self.iz as f64 + 0.5) * size).abs() - half;
        dx.max(dz).max(0.0)
    }
}

/// The tiles wanted this frame: leaves are drawn, inner nodes are fallbacks.
struct Node {
    key: ChunkKey,
    children: Option<Box<[Node; 4]>>,
}

fn build_tree(key: ChunkKey, px: f64, pz: f64) -> Node {
    let split = key.level > 0 && key.distance_to(px, pz) < SPLIT_DISTANCE * size_of(key.level);
    let children = split.then(|| Box::new(key.children().map(|child| build_tree(child, px, pz))));
    Node { key, children }
}

fn build_roots(px: f64, pz: f64) -> Vec<Node> {
    let size = size_of(MAX_LEVEL);
    let (cx, cz) = ((px / size).floor() as i64, (pz / size).floor() as i64);
    let mut roots = Vec::new();
    for dz in -ROOT_RING..=ROOT_RING {
        for dx in -ROOT_RING..=ROOT_RING {
            roots.push(build_tree(ChunkKey { level: MAX_LEVEL, ix: cx + dx, iz: cz + dz }, px, pz));
        }
    }
    roots
}

enum ChunkState {
    Building(Task<ChunkMeshData>),
    Ready { entity: Entity, last_used: u64 },
}

#[derive(Resource, Default)]
pub struct ChunkCache {
    chunks: HashMap<ChunkKey, ChunkState>,
    frame: u64,
    pub pending: usize,
}

impl ChunkCache {
    fn is_ready(&self, key: ChunkKey) -> bool {
        matches!(self.chunks.get(&key), Some(ChunkState::Ready { .. }))
    }

    fn in_flight(&self) -> usize {
        self.chunks.values().filter(|c| matches!(c, ChunkState::Building(_))).count()
    }
}

#[derive(Component)]
pub struct TerrainChunk;

/// Marks what the streaming is centred on (the rover).
#[derive(Component)]
pub struct TerrainFocus;

pub fn stream_chunks(
    mut commands: Commands,
    mut cache: ResMut<ChunkCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    terrain: Res<Terrain>,
    material: Res<TerrainMaterial>,
    focus: Query<&WorldPosition, With<TerrainFocus>>,
    mut visibility: Query<&mut Visibility, With<TerrainChunk>>,
) {
    let Ok(focus) = focus.single() else { return };
    let (px, pz) = (focus.0.x, focus.0.z);
    cache.frame += 1;
    collect_finished(&mut commands, &mut cache, &mut meshes, &material);
    let roots = build_roots(px, pz);
    queue_missing(&mut cache, &terrain, &roots, px, pz);
    let shown = displayed_tiles(&mut cache, &roots);
    for (key, state) in &cache.chunks {
        if let ChunkState::Ready { entity, .. } = state
            && let Ok(mut vis) = visibility.get_mut(*entity)
        {
            vis.set_if_neq(if shown.contains(key) { Visibility::Inherited } else { Visibility::Hidden });
        }
    }
    evict(&mut commands, &mut cache);
}

fn collect_finished(commands: &mut Commands, cache: &mut ChunkCache, meshes: &mut Assets<Mesh>, material: &TerrainMaterial) {
    let frame = cache.frame;
    for (key, state) in cache.chunks.iter_mut() {
        let ChunkState::Building(task) = state else { continue };
        let Some(data) = block_on(poll_once(task)) else { continue };
        let entity = commands
            .spawn((
                TerrainChunk,
                Mesh3d(meshes.add(to_mesh(data))),
                MeshMaterial3d(material.0.clone()),
                WorldPosition(key.origin()),
                Transform::default(),
                Visibility::Hidden,
            ))
            .id();
        *state = ChunkState::Ready { entity, last_used: frame };
    }
}

fn queue_missing(cache: &mut ChunkCache, terrain: &Terrain, roots: &[Node], px: f64, pz: f64) {
    let mut missing = Vec::new();
    fn walk(node: &Node, cache: &ChunkCache, out: &mut Vec<ChunkKey>) {
        match &node.children {
            Some(children) => children.iter().for_each(|c| walk(c, cache, out)),
            None if !cache.chunks.contains_key(&node.key) => out.push(node.key),
            None => {}
        }
    }
    roots.iter().for_each(|r| walk(r, cache, &mut missing));
    cache.pending = missing.len();
    missing.sort_by(|a, b| b.level.cmp(&a.level).then(a.distance_to(px, pz).total_cmp(&b.distance_to(px, pz))));
    let pool = AsyncComputeTaskPool::get();
    let free = MAX_IN_FLIGHT.saturating_sub(cache.in_flight());
    for key in missing.into_iter().take(free) {
        let (heightfield, palette) = (Arc::clone(&terrain.heightfield), terrain.palette);
        let task = pool.spawn(async move { build_chunk_mesh(&heightfield, &palette, key.origin(), size_of(key.level), CHUNK_RES) });
        cache.chunks.insert(key, ChunkState::Building(task));
    }
}

/// Leaves where built; otherwise the nearest built ancestor.
fn displayed_tiles(cache: &mut ChunkCache, roots: &[Node]) -> std::collections::HashSet<ChunkKey> {
    fn can_display(node: &Node, cache: &ChunkCache) -> bool {
        cache.is_ready(node.key) || node.children.as_ref().is_some_and(|c| c.iter().all(|n| can_display(n, cache)))
    }
    fn show(node: &Node, cache: &ChunkCache, out: &mut std::collections::HashSet<ChunkKey>) {
        if let Some(children) = &node.children
            && children.iter().all(|c| can_display(c, cache))
        {
            return children.iter().for_each(|c| show(c, cache, out));
        }
        if cache.is_ready(node.key) {
            out.insert(node.key);
        } else if let Some(children) = &node.children {
            children.iter().for_each(|c| show(c, cache, out));
        }
    }
    let mut shown = std::collections::HashSet::new();
    roots.iter().for_each(|r| show(r, cache, &mut shown));
    let frame = cache.frame;
    for key in &shown {
        if let Some(ChunkState::Ready { last_used, .. }) = cache.chunks.get_mut(key) {
            *last_used = frame;
        }
    }
    shown
}

fn evict(commands: &mut Commands, cache: &mut ChunkCache) {
    if cache.chunks.len() <= MAX_CACHED {
        return;
    }
    let frame = cache.frame;
    let mut stale: Vec<_> = cache
        .chunks
        .iter()
        .filter_map(|(k, s)| match s {
            ChunkState::Ready { last_used, entity } if *last_used < frame => Some((*k, *last_used, *entity)),
            _ => None,
        })
        .collect();
    stale.sort_by_key(|(_, used, _)| *used);
    let excess = cache.chunks.len() - MAX_CACHED * 4 / 5;
    for (key, _, entity) in stale.into_iter().take(excess) {
        commands.entity(entity).despawn();
        cache.chunks.remove(&key);
    }
}

fn to_mesh(data: ChunkMeshData) -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, data.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, data.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, data.colors)
        .with_inserted_indices(Indices::U32(data.indices))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaves(node: &Node, out: &mut Vec<ChunkKey>) {
        match &node.children {
            Some(c) => c.iter().for_each(|n| leaves(n, out)),
            None => out.push(node.key),
        }
    }

    #[test]
    fn finest_tiles_surround_the_player() {
        let mut all = Vec::new();
        build_roots(10.0, 10.0).iter().for_each(|r| leaves(r, &mut all));
        let under = all.iter().find(|k| k.distance_to(10.0, 10.0) == 0.0).unwrap();
        assert_eq!(under.level, 0);
    }

    #[test]
    fn leaves_tile_the_area_without_overlap() {
        let mut all = Vec::new();
        build_roots(5_000.0, -3_000.0).iter().for_each(|r| leaves(r, &mut all));
        let area: f64 = all.iter().map(|k| size_of(k.level).powi(2)).sum();
        let roots = (2 * ROOT_RING + 1) as f64;
        assert_eq!(area, (roots * size_of(MAX_LEVEL)).powi(2));
    }
}
