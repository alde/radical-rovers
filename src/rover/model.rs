//! Visual rover: chassis, mast, solar deck and four steerable, sprung wheels.

use bevy::prelude::*;

use super::spec::RoverSpec;

const AIR_SPIN_DECAY: f32 = 0.4; // 1/s; free-spinning wheels slow down in the air

#[derive(Component)]
pub struct WheelPivot(pub usize);

#[derive(Component, Default)]
pub struct WheelTyre {
    pub spin_rate: f32,
}

pub fn spawn_model(commands: &mut Commands, root: Entity, spec: &RoverSpec, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) {
    let e = spec.half_extents.as_vec3();
    let mut part = |shape: Mesh, color: Color, metallic: f32, at: Vec3| {
        let material = materials.add(StandardMaterial { base_color: color, metallic, perceptual_roughness: 0.5, ..default() });
        commands.spawn((Mesh3d(meshes.add(shape)), MeshMaterial3d(material), Transform::from_translation(at), ChildOf(root)));
    };
    part(Cuboid::new(e.x * 2.0, e.y * 1.2, e.z * 2.0).into(), Color::srgb(0.85, 0.83, 0.78), 0.1, Vec3::ZERO);
    part(Cuboid::new(e.x * 1.4, e.y * 0.6, e.z * 1.1).into(), Color::srgb(0.79, 0.64, 0.23), 0.8, Vec3::new(0.0, e.y * 0.85, 0.2));
    part(Cuboid::new(e.x * 2.4, 0.05, e.z).into(), Color::srgb(0.1, 0.16, 0.33), 0.4, Vec3::new(0.0, e.y * 0.9, e.z * 0.75));
    part(Cylinder::new(0.06, 1.4).into(), Color::srgb(0.73, 0.73, 0.73), 0.5, Vec3::new(-e.x * 0.6, e.y + 0.7, -e.z * 0.7));
    part(Cuboid::new(0.5, 0.25, 0.3).into(), Color::srgb(0.93, 0.93, 0.93), 0.1, Vec3::new(-e.x * 0.6, e.y + 1.45, -e.z * 0.7));
    part(Cylinder::new(0.35, 0.05).into(), Color::srgb(0.87, 0.87, 0.87), 0.3, Vec3::new(e.x * 0.6, e.y + 0.35, e.z * 0.1));
    spawn_wheels(commands, root, spec, meshes, materials);
}

fn spawn_wheels(commands: &mut Commands, root: Entity, spec: &RoverSpec, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) {
    let tyre_mesh = meshes.add(Cylinder::new(spec.wheel_radius as f32, spec.wheel_width as f32).mesh().resolution(10));
    let tyre_material = materials.add(StandardMaterial { base_color: Color::srgb(0.33, 0.35, 0.36), metallic: 0.6, perceptual_roughness: 0.5, ..default() });
    for (i, mount) in spec.wheel_mounts.iter().enumerate() {
        let pivot = commands.spawn((WheelPivot(i), Transform::from_translation(mount.as_vec3()), Visibility::default(), ChildOf(root))).id();
        // Cylinders stand along y; lay the tyre on its side so it rolls about x.
        let lay_flat = Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        let spinner = commands.spawn((WheelTyre::default(), Transform::default(), Visibility::default(), ChildOf(pivot))).id();
        commands.spawn((Mesh3d(tyre_mesh.clone()), MeshMaterial3d(tyre_material.clone()), lay_flat, ChildOf(spinner)));
    }
}

/// Wheel spin direction: rolling forward (-z), the top moves forward, which is
/// a negative rotation about x.
pub fn advance_spin(tyre: &mut WheelTyre, grounded: bool, ground_speed: f32, radius: f32, dt: f32) -> f32 {
    tyre.spin_rate = if grounded { ground_speed / radius } else { tyre.spin_rate * (1.0 - AIR_SPIN_DECAY * dt) };
    -tyre.spin_rate * dt
}
