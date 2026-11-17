//! Radical Rovers: a rover stunt sandbox on the real Moon and Mars.

mod camera;
mod capture;
mod hud;
mod math;
mod planet;
mod rover;
mod sky;
mod terrain;
mod tricks;
mod world;

use std::path::PathBuf;

use bevy::prelude::*;

use planet::Planet;
use terrain::Terrain;

/// Which world and landing site to start at: `--planet moon --site 3`.
/// `--screenshot out.png` captures a frame and quits.
#[derive(Resource, Debug, Default, PartialEq)]
pub struct LaunchOptions {
    pub planet: Option<String>,
    pub site: usize,
    pub screenshot: Option<PathBuf>,
}

impl LaunchOptions {
    fn parse(args: impl IntoIterator<Item = String>) -> Self {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--planet" => options.planet = args.next(),
                "--site" => options.site = args.next().and_then(|s| s.parse().ok()).unwrap_or(0),
                "--screenshot" => options.screenshot = args.next().map(PathBuf::from),
                _ => {}
            }
        }
        options
    }
}

/// `data/` next to the working directory when run with cargo, otherwise next
/// to the executable (or in the macOS bundle's Resources).
fn data_dir() -> PathBuf {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from));
    let candidates = [Some(PathBuf::from("data")), exe_dir.as_ref().map(|d| d.join("data")), exe_dir.map(|d| d.join("../Resources/data"))];
    candidates.into_iter().flatten().find(|p| p.is_dir()).unwrap_or_else(|| PathBuf::from("data"))
}

fn main() {
    let launch = LaunchOptions::parse(std::env::args().skip(1));
    let planet = launch.planet.as_deref().and_then(Planet::by_id).unwrap_or_else(planet::mars);
    let terrain = Terrain::load(&planet, &data_dir());
    let window = Window { title: "Radical Rovers".into(), ..default() };
    let screenshot = launch.screenshot.clone();
    let mut app = App::new();
    app
        .add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(window), ..default() }))
        .insert_resource(planet)
        .insert_resource(terrain)
        .insert_resource(launch)
        .add_plugins((world::WorldPlugin, terrain::TerrainPlugin, rover::RoverPlugin, camera::CameraPlugin, sky::SkyPlugin, tricks::TricksPlugin, hud::HudPlugin));
    if let Some(path) = screenshot {
        app.add_plugins(capture::CapturePlugin(path));
    }
    app.run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_planet_and_site() {
        let args = ["--planet", "moon", "--site", "3"].map(String::from);
        assert_eq!(LaunchOptions::parse(args), LaunchOptions { planet: Some("moon".into()), site: 3, screenshot: None });
    }

    #[test]
    fn defaults_without_arguments() {
        assert_eq!(LaunchOptions::parse(Vec::new()), LaunchOptions::default());
    }
}
