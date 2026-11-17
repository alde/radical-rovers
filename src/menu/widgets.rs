//! Building blocks shared by the menu screens: neon buttons and chrome text.

use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button};

use super::MenuAction;
use crate::style::{self, CHROME_BOTTOM, CHROME_TOP, DISPLAY, NEON_CYAN, NEON_PINK, NIGHT};

/// What a click (or Enter, when focused) does.
#[derive(Component, Clone, Copy)]
pub struct OnActivate(pub MenuAction);

/// Keyboard focus order within a screen's button list.
#[derive(Component)]
pub struct FocusIndex(pub usize);

/// Neon-outlined menu button; `primary` gets the hot pink fill.
#[derive(Component)]
pub struct NeonButton {
    pub primary: bool,
}

pub fn send_activation(event: On<Activate>, buttons: Query<&OnActivate>, mut actions: MessageWriter<MenuAction>) {
    if let Ok(OnActivate(action)) = buttons.get(event.entity) {
        actions.write(*action);
    }
}

pub fn neon_button(parent: &mut ChildSpawnerCommands, label: &str, action: MenuAction, focus: usize, primary: bool) {
    let node = Node {
        padding: UiRect::axes(Val::Px(28.0), Val::Px(12.0)),
        border: UiRect::all(Val::Px(2.0)),
        justify_content: JustifyContent::Center,
        min_width: Val::Px(220.0),
        ..default()
    };
    parent
        .spawn((Button, bevy::picking::hover::Hovered::default(), OnActivate(action), FocusIndex(focus), NeonButton { primary }, node, BorderColor::all(NEON_CYAN), BackgroundColor(NIGHT.with_alpha(0.6)), style::glow(NEON_CYAN, 12.0)))
        .observe(send_activation)
        .with_child(style::display_text(&label.to_uppercase(), 18.0, NEON_CYAN));
}

/// Paint a button lit (hovered/focused) or idle.
pub fn paint_button(button: &NeonButton, lit: bool, background: &mut BackgroundColor, border: &mut BorderColor, shadow: &mut BoxShadow, text: &mut TextColor) {
    let accent = if button.primary { NEON_PINK } else { NEON_CYAN };
    *border = BorderColor::all(accent);
    *shadow = style::glow(accent.with_alpha(if lit { 0.9 } else { 0.45 }), if lit { 26.0 } else { 12.0 });
    background.0 = match (button.primary, lit) {
        (true, _) => NEON_PINK.mix(&Color::srgb(0.48, 0.12, 0.64), if lit { 0.0 } else { 0.45 }),
        (false, true) => NEON_CYAN,
        (false, false) => NIGHT.with_alpha(0.6),
    };
    text.0 = if lit && !button.primary { NIGHT } else if button.primary { Color::WHITE } else { NEON_CYAN };
}

/// Two-tone chrome lettering: pale steel above the midline, pink below,
/// with a deep purple drop shadow.
pub fn chrome_text(parent: &mut ChildSpawnerCommands, value: &str, size: f32) {
    let font = || style::font(DISPLAY, size, true);
    parent.spawn(Node { position_type: PositionType::Relative, ..default() }).with_children(|chrome| {
        chrome.spawn((Text::new(value), font(), TextColor(CHROME_BOTTOM), TextShadow { offset: Vec2::new(0.0, size * 0.06), color: Color::srgb(0.1, 0.02, 0.22) }));
        let top_half = Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(50.0), overflow: Overflow::clip(), ..default() };
        chrome.spawn(top_half).with_child((Text::new(value), font(), TextColor(CHROME_TOP)));
    });
}
