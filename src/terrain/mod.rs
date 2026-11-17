pub mod chunk_mesh;
pub mod chunks;
pub mod colorizer;
pub mod dem;
pub mod heightfield;

use std::path::Path;
use std::sync::Arc;

use bevy::prelude::*;

use crate::planet::{Palette, Planet};
use chunks::{ChunkCache, stream_chunks};
use dem::{Dem, GridDem, ProceduralDem};
use heightfield::Heightfield;

#[derive(Resource)]
pub struct Terrain {
    pub heightfield: Arc<Heightfield>,
    pub palette: Palette,
    pub source_label: String,
}

#[derive(Resource)]
pub struct TerrainMaterial(pub Handle<StandardMaterial>);

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkCache>().add_systems(Startup, setup_material).add_systems(Update, stream_chunks);
    }
}

impl Terrain {
    /// Real NASA data from `data/` when converted, procedural otherwise.
    pub fn load(planet: &Planet, data_dir: &Path) -> Self {
        let (dem, source_label): (Arc<dyn Dem>, String) = match GridDem::load(data_dir, planet.id) {
            Ok(grid) => {
                let label = format!("{} {} px/deg", planet.dem_source, grid.pixels_per_degree());
                (Arc::new(grid), label)
            }
            Err(err) => {
                warn!("No DEM for {} ({err}); using procedural terrain. Run tools/fetch-dem.sh", planet.id);
                (Arc::new(ProceduralDem::new(&planet.procedural)), "procedural placeholder".into())
            }
        };
        Self { heightfield: Arc::new(Heightfield::new(planet, dem)), palette: planet.palette, source_label }
    }
}

fn setup_material(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let material = StandardMaterial { base_color: Color::WHITE, perceptual_roughness: 0.95, reflectance: 0.1, ..default() };
    commands.insert_resource(TerrainMaterial(materials.add(material)));
}
