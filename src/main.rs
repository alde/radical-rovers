//! Radical Rovers: a rover stunt sandbox on the real Moon and Mars.

mod audio;
mod camera;
mod capture;
mod hud;
mod math;
mod menu;
mod planet;
mod rover;
mod screen;
mod sky;
mod style;
mod terrain;
mod tricks;
mod world;
mod worlds;

use std::path::PathBuf;

use bevy::prelude::*;

use menu::MenuSelection;
use screen::Screen;
use worlds::Worlds;

/// `--planet moon --site 3` skips the title and drops you there.
/// `--screen title|howto|pause|playing` opens a given screen.
/// `--screenshot out.png` captures a frame and quits.
#[derive(Resource, Debug, Default, PartialEq)]
pub struct LaunchOptions {
    pub planet: Option<String>,
    pub site: usize,
    pub screen: Option<Screen>,
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
                "--screen" => options.screen = args.next().as_deref().and_then(Screen::from_arg),
                "--screenshot" => options.screenshot = args.next().map(PathBuf::from),
                _ => {}
            }
        }
        options
    }

    /// Straight into play if a planet or site was named, else the title.
    fn first_screen(&self) -> Screen {
        let named_place = self.planet.is_some() || self.site > 0;
        self.screen.unwrap_or(if named_place { Screen::Playing } else { Screen::Title })
    }
}

/// `data/` next to the working directory when run with cargo, otherwise next
/// to the executable (or in the macOS bundle's Resources).
fn data_dir() -> PathBuf {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from));
    let candidates = [Some(PathBuf::from("data")), exe_dir.as_ref().map(|d| d.join("data")), exe_dir.map(|d| d.join("../Resources/data"))];
    candidates.into_iter().flatten().find(|p| p.is_dir()).unwrap_or_else(|| PathBuf::from("data"))
}

const DEFAULT_PLANET: &str = "mars";

fn main() {
    let launch = LaunchOptions::parse(std::env::args().skip(1));
    let worlds = Worlds::load(&data_dir());
    let world = launch.planet.as_deref().and_then(|id| worlds.index_of(id)).or_else(|| worlds.index_of(DEFAULT_PLANET)).unwrap_or(0);
    let mut sites = vec![0; worlds.0.len()];
    sites[world] = launch.site.min(worlds.0[world].planet.landmarks.len() - 1);
    let window = Window { title: "Radical Rovers".into(), ..default() };
    let screenshot = launch.screenshot.clone();
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(window), ..default() }))
        .insert_resource(worlds.0[world].planet.clone())
        .insert_resource(worlds.0[world].terrain.clone())
        .insert_state(launch.first_screen())
        .insert_resource(MenuSelection { world, sites })
        .insert_resource(worlds)
        .insert_resource(launch)
        .add_plugins((screen::ScreenPlugin, world::WorldPlugin, terrain::TerrainPlugin, rover::RoverPlugin, camera::CameraPlugin, sky::SkyPlugin))
        .add_plugins((tricks::TricksPlugin, hud::HudPlugin, menu::MenuPlugin, audio::SoundPlugin));
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
        let options = LaunchOptions::parse(args);
        assert_eq!(options, LaunchOptions { planet: Some("moon".into()), site: 3, ..default() });
        assert_eq!(options.first_screen(), Screen::Playing);
    }

    #[test]
    fn starts_on_the_title_unless_told_otherwise() {
        assert_eq!(LaunchOptions::default().first_screen(), Screen::Title);
        let options = LaunchOptions::parse(["--screen", "pause"].map(String::from));
        assert_eq!(options.first_screen(), Screen::Paused);
    }

    #[test]
    fn defaults_without_arguments() {
        assert_eq!(LaunchOptions::parse(Vec::new()), LaunchOptions::default());
    }
}
