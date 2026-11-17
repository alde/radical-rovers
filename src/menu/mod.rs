//! Synthwave title screen, how-to-play and pause menus.

mod backdrop;
mod overlays;
mod thumbnail;
mod title;
mod widgets;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::IsDefaultUiCamera;

use crate::audio::SoundSettings;
use crate::camera::GameCamera;
use crate::hud::HudRoot;
use crate::planet::Planet;
use crate::rover::TeleportTo;
use crate::screen::Screen;
use crate::tricks::Tricks;
use crate::worlds::{self, Worlds};
use backdrop::MenuCamera;
use widgets::{FocusIndex, NeonButton, OnActivate};

#[derive(Message, Clone, Copy, PartialEq, Debug)]
pub enum MenuAction {
    Start,
    HowTo,
    Back,
    Resume,
    ToggleSound,
    MainMenu,
    Quit,
    SelectWorld(usize),
    CycleSite { world: usize, delta: i32 },
}

/// Chosen world and landing site per world, as shown on the title cards.
#[derive(Resource)]
pub struct MenuSelection {
    pub world: usize,
    pub sites: Vec<usize>,
}

impl MenuSelection {
    fn cycle_site(&mut self, world: usize, delta: i32, site_count: usize) {
        let site = &mut self.sites[world];
        *site = (*site as i32 + delta).rem_euclid(site_count as i32) as usize;
    }
}

/// Keyboard focus within the current screen's buttons.
#[derive(Resource, Default)]
struct MenuFocus(usize);

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MenuAction>()
            .init_resource::<MenuFocus>()
            .add_plugins(backdrop::BackdropPlugin)
            .add_systems(OnEnter(Screen::Title), title::spawn)
            .add_systems(OnEnter(Screen::HowTo { from_pause: false }), overlays::spawn_how_to)
            .add_systems(OnEnter(Screen::HowTo { from_pause: true }), overlays::spawn_how_to)
            .add_systems(OnEnter(Screen::Paused), overlays::spawn_pause)
            .add_systems(StateTransition, reset_focus.after(bevy::state::state::StateTransitionSystems::EnterSchedules))
            .add_systems(Update, (switch_cameras, keyboard, follow_hover, paint_buttons, title::refresh_cards, overlays::refresh_sound_label, apply_actions).chain());
    }
}

fn reset_focus(screen: Res<State<Screen>>, mut focus: ResMut<MenuFocus>) {
    if screen.is_changed() {
        focus.0 = 0;
    }
}

/// Title shows the synthwave backdrop; everything else shows the planet.
/// Retries until both cameras exist: the first screen is entered before
/// Startup spawns them.
fn switch_cameras(
    mut commands: Commands,
    mut applied: Local<bool>,
    screen: Res<State<Screen>>,
    mut menu_camera: Query<(Entity, &mut Camera), (With<MenuCamera>, Without<GameCamera>)>,
    mut game_camera: Query<(Entity, &mut Camera), (With<GameCamera>, Without<MenuCamera>)>,
    mut hud: Query<&mut Visibility, With<HudRoot>>,
) {
    if *applied && !screen.is_changed() {
        return;
    }
    let world = screen.get().shows_world();
    let (Ok((menu, mut menu_cam)), Ok((game, mut game_cam))) = (menu_camera.single_mut(), game_camera.single_mut()) else { return };
    *applied = true;
    menu_cam.is_active = !world;
    game_cam.is_active = world;
    let (ui_on, ui_off) = if world { (game, menu) } else { (menu, game) };
    commands.entity(ui_on).insert(IsDefaultUiCamera);
    commands.entity(ui_off).remove::<IsDefaultUiCamera>();
    for mut visibility in &mut hud {
        *visibility = if world { Visibility::Inherited } else { Visibility::Hidden };
    }
}

fn keyboard(keys: Res<ButtonInput<KeyCode>>, screen: Res<State<Screen>>, selection: Res<MenuSelection>, mut focus: ResMut<MenuFocus>, buttons: Query<(&FocusIndex, &OnActivate)>, mut actions: MessageWriter<MenuAction>) {
    let back = [KeyCode::Escape, KeyCode::Enter, KeyCode::Backspace];
    match screen.get() {
        Screen::Playing if keys.just_pressed(KeyCode::Escape) => {
            actions.write(MenuAction::Back);
        }
        Screen::Title => title_keys(&keys, &selection, &mut actions),
        Screen::HowTo { .. } if keys.any_just_pressed(back) => {
            actions.write(MenuAction::Back);
        }
        Screen::Paused => pause_keys(&keys, &mut focus, &buttons, &mut actions),
        _ => {}
    }
}

/// ← → choose the world, ↑ ↓ its landing site, Enter starts, H explains.
fn title_keys(keys: &ButtonInput<KeyCode>, selection: &MenuSelection, actions: &mut MessageWriter<MenuAction>) {
    let (world, count) = (selection.world, selection.sites.len());
    if keys.just_pressed(KeyCode::ArrowLeft) {
        actions.write(MenuAction::SelectWorld((world + count - 1) % count));
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        actions.write(MenuAction::SelectWorld((world + 1) % count));
    }
    for (key, delta) in [(KeyCode::ArrowUp, -1), (KeyCode::ArrowDown, 1)] {
        if keys.just_pressed(key) {
            actions.write(MenuAction::CycleSite { world, delta });
        }
    }
    if keys.just_pressed(KeyCode::Enter) {
        actions.write(MenuAction::Start);
    }
    if keys.just_pressed(KeyCode::KeyH) {
        actions.write(MenuAction::HowTo);
    }
}

/// ↑ ↓ move focus, Enter activates it, Esc resumes.
fn pause_keys(keys: &ButtonInput<KeyCode>, focus: &mut MenuFocus, buttons: &Query<(&FocusIndex, &OnActivate)>, actions: &mut MessageWriter<MenuAction>) {
    if keys.just_pressed(KeyCode::Escape) {
        actions.write(MenuAction::Resume);
    }
    let count = buttons.iter().count().max(1);
    if keys.just_pressed(KeyCode::ArrowDown) {
        focus.0 = (focus.0 + 1) % count;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        focus.0 = (focus.0 + count - 1) % count;
    }
    if keys.just_pressed(KeyCode::Enter)
        && let Some((_, OnActivate(action))) = buttons.iter().find(|(i, _)| i.0 == focus.0)
    {
        actions.write(*action);
    }
}

/// The mouse moves keyboard focus, so there's only ever one lit button.
fn follow_hover(buttons: Query<(&Hovered, &FocusIndex), Changed<Hovered>>, mut focus: ResMut<MenuFocus>) {
    for (hovered, index) in &buttons {
        if hovered.0 {
            focus.0 = index.0;
        }
    }
}

fn paint_buttons(
    focus: Res<MenuFocus>,
    mut buttons: Query<(&NeonButton, &FocusIndex, &Hovered, &mut BackgroundColor, &mut BorderColor, &mut BoxShadow, &Children)>,
    mut texts: Query<&mut TextColor>,
) {
    for (button, index, hovered, mut background, mut border, mut shadow, children) in &mut buttons {
        let lit = hovered.0 || index.0 == focus.0;
        let Some(label) = children.iter().find(|child| texts.contains(*child)) else { continue };
        if let Ok(mut text) = texts.get_mut(label) {
            widgets::paint_button(button, lit, &mut background, &mut border, &mut shadow, &mut text);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_actions(
    mut commands: Commands,
    mut actions: MessageReader<MenuAction>,
    screen: Res<State<Screen>>,
    mut next: ResMut<NextState<Screen>>,
    mut selection: ResMut<MenuSelection>,
    worlds: Res<Worlds>,
    planet: Res<Planet>,
    mut tricks: ResMut<Tricks>,
    mut sound: ResMut<SoundSettings>,
    mut teleports: MessageWriter<TeleportTo>,
    mut exit: MessageWriter<AppExit>,
) {
    for action in actions.read() {
        match *action {
            MenuAction::Start => {
                start_run(&mut commands, &worlds, &selection, &planet, &mut teleports);
                *tricks = Tricks::default();
                next.set(Screen::Playing);
            }
            MenuAction::HowTo => next.set(Screen::HowTo { from_pause: *screen.get() == Screen::Paused }),
            MenuAction::Back => next.set(back_from(*screen.get())),
            MenuAction::Resume => next.set(Screen::Playing),
            MenuAction::ToggleSound => sound.enabled = !sound.enabled,
            MenuAction::MainMenu => next.set(Screen::Title),
            MenuAction::Quit => {
                exit.write(AppExit::Success);
            }
            MenuAction::SelectWorld(world) => selection.world = world,
            MenuAction::CycleSite { world, delta } => {
                selection.world = world;
                selection.cycle_site(world, delta, worlds.0[world].planet.landmarks.len());
            }
        }
    }
}

/// Where Back (or Esc) goes from each screen. From play, Esc pauses.
fn back_from(screen: Screen) -> Screen {
    match screen {
        Screen::HowTo { from_pause: true } | Screen::Playing => Screen::Paused,
        Screen::HowTo { from_pause: false } | Screen::Title => Screen::Title,
        Screen::Paused => Screen::Playing,
    }
}

fn start_run(commands: &mut Commands, worlds: &Worlds, selection: &MenuSelection, planet: &Planet, teleports: &mut MessageWriter<TeleportTo>) {
    let world = &worlds.0[selection.world];
    if world.planet.id != planet.id {
        worlds::activate(commands, worlds, selection.world);
    }
    let site = &world.planet.landmarks[selection.sites[selection.world]];
    teleports.write(TeleportTo { lat: site.lat, lon: site.lon, name: site.name });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn landing_sites_wrap_both_ways() {
        let mut selection = MenuSelection { world: 0, sites: vec![0, 0] };
        selection.cycle_site(1, -1, 7);
        assert_eq!(selection.sites[1], 6);
        selection.cycle_site(1, 1, 7);
        assert_eq!(selection.sites[1], 0);
    }

    /// Headless app with just the menu's action handling and the resources it touches.
    fn menu_app() -> App {
        let worlds = Worlds::load(std::path::Path::new("no-such-dir")); // procedural: fast and hermetic
        let mars = worlds.index_of("mars").unwrap();
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin))
            .insert_state(Screen::Title)
            .add_message::<MenuAction>()
            .add_message::<TeleportTo>()
            .add_message::<AppExit>()
            .insert_resource(worlds.0[mars].planet.clone())
            .insert_resource(worlds.0[mars].terrain.clone())
            .insert_resource(MenuSelection { world: mars, sites: vec![0; worlds.0.len()] })
            .insert_resource(worlds)
            .insert_resource(SoundSettings { enabled: true })
            .init_resource::<Tricks>()
            .add_systems(Update, apply_actions);
        app
    }

    fn send(app: &mut App, action: MenuAction) {
        app.world_mut().write_message(action);
        app.update();
        app.update(); // let the state transition apply
    }

    #[test]
    fn starting_on_another_world_switches_planet_and_drops_you_at_the_site() {
        let mut app = menu_app();
        let moon = app.world().resource::<Worlds>().index_of("moon").unwrap();
        app.world_mut().resource_mut::<Tricks>().0.score = 1234;
        send(&mut app, MenuAction::CycleSite { world: moon, delta: 3 });
        send(&mut app, MenuAction::Start);
        assert_eq!(*app.world().resource::<State<Screen>>().get(), Screen::Playing);
        assert_eq!(app.world().resource::<Planet>().id, "moon");
        assert_eq!(app.world().resource::<Tricks>().0.score, 0, "a new run starts from zero");
        let teleports = app.world().resource::<Messages<TeleportTo>>();
        let mut cursor = teleports.get_cursor();
        let arrival = cursor.read(teleports).last().expect("expected a teleport");
        assert_eq!(arrival.name, crate::planet::moon().landmarks[3].name);
    }

    #[test]
    fn pause_resume_and_main_menu_move_between_screens() {
        let mut app = menu_app();
        send(&mut app, MenuAction::Start);
        send(&mut app, MenuAction::Back); // Esc in play
        assert_eq!(*app.world().resource::<State<Screen>>().get(), Screen::Paused);
        send(&mut app, MenuAction::HowTo);
        assert_eq!(*app.world().resource::<State<Screen>>().get(), Screen::HowTo { from_pause: true });
        send(&mut app, MenuAction::Back);
        send(&mut app, MenuAction::MainMenu);
        assert_eq!(*app.world().resource::<State<Screen>>().get(), Screen::Title);
    }

    #[test]
    fn sound_toggles() {
        let mut app = menu_app();
        send(&mut app, MenuAction::ToggleSound);
        assert!(!app.world().resource::<SoundSettings>().enabled);
    }

    #[test]
    fn back_and_escape_route_between_screens() {
        assert_eq!(back_from(Screen::Playing), Screen::Paused);
        assert_eq!(back_from(Screen::Paused), Screen::Playing);
        assert_eq!(back_from(Screen::HowTo { from_pause: true }), Screen::Paused);
        assert_eq!(back_from(Screen::HowTo { from_pause: false }), Screen::Title);
    }
}
