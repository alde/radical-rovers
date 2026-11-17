//! Which screen the game is on. Gameplay only runs while Playing; game time
//! is frozen everywhere else.

use bevy::prelude::*;

#[derive(States, Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Screen {
    #[default]
    Title,
    HowTo {
        from_pause: bool,
    },
    Playing,
    Paused,
}

impl Screen {
    /// Whether the planet is on screen (behind a menu or not).
    pub fn shows_world(self) -> bool {
        matches!(self, Screen::Playing | Screen::Paused | Screen::HowTo { from_pause: true })
    }

    /// `--screen title|howto|pause|playing`, for screenshots.
    pub fn from_arg(arg: &str) -> Option<Screen> {
        match arg {
            "title" => Some(Screen::Title),
            "howto" => Some(Screen::HowTo { from_pause: false }),
            "pause" => Some(Screen::Paused),
            "playing" => Some(Screen::Playing),
            _ => None,
        }
    }
}

pub struct ScreenPlugin;

impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::Playing), unpause_time).add_systems(OnExit(Screen::Playing), pause_time);
    }
}

fn pause_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

fn unpause_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_world_shows_behind_pause_but_not_title() {
        assert!(Screen::Paused.shows_world());
        assert!(Screen::HowTo { from_pause: true }.shows_world());
        assert!(!Screen::Title.shows_world());
        assert!(!Screen::HowTo { from_pause: false }.shows_world());
    }
}
