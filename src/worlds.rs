//! Both planets, loaded at startup so switching from the menu is instant.
//! The active one is mirrored into the `Planet` and `Terrain` resources.

use std::path::Path;

use bevy::prelude::*;

use crate::planet::{self, Planet};
use crate::terrain::Terrain;

pub struct World {
    pub planet: Planet,
    pub terrain: Terrain,
}

#[derive(Resource)]
pub struct Worlds(pub Vec<World>);

impl Worlds {
    pub fn load(data_dir: &Path) -> Self {
        let worlds = [planet::moon(), planet::mars()].into_iter().map(|planet| World { terrain: Terrain::load(&planet, data_dir), planet }).collect();
        Self(worlds)
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.0.iter().position(|w| w.planet.id == id)
    }
}

/// Make world `index` the active one. Systems watching `Planet`/`Terrain`
/// (sky, terrain streaming) react to the change.
pub fn activate(commands: &mut Commands, worlds: &Worlds, index: usize) {
    let world = &worlds.0[index];
    commands.insert_resource(world.planet.clone());
    commands.insert_resource(world.terrain.clone());
}
