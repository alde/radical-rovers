//! Sun, ambient light, sky colour and distance fog for the active planet.

use bevy::prelude::*;

use crate::camera::GameCamera;
use crate::planet::Planet;

#[derive(Component)]
struct Sun;

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_sun).add_systems(Update, apply_sky.run_if(resource_changed::<Planet>));
    }
}

fn spawn_sun(mut commands: Commands) {
    commands.spawn((Sun, DirectionalLight { shadow_maps_enabled: false, ..default() }, Transform::default()));
}

fn apply_sky(
    mut commands: Commands,
    planet: Res<Planet>,
    mut sun: Query<(&mut DirectionalLight, &mut Transform), With<Sun>>,
    mut fog: Query<&mut DistanceFog, With<GameCamera>>,
) {
    commands.insert_resource(ClearColor(planet.sky));
    let (ambient_color, ambient_brightness) = planet.ambient;
    commands.insert_resource(GlobalAmbientLight { color: ambient_color, brightness: ambient_brightness, ..default() });
    if let Ok((mut light, mut transform)) = sun.single_mut() {
        light.illuminance = planet.sun_illuminance;
        *transform = Transform::IDENTITY.looking_to(-planet.sun_direction.normalize(), Vec3::Y);
    }
    if let Ok(mut fog) = fog.single_mut() {
        let (color, start, end) = planet.fog;
        *fog = DistanceFog { color, falloff: FogFalloff::Linear { start, end }, ..default() };
    }
}
