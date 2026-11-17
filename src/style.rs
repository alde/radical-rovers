//! Shared look: fonts and the synthwave palette. System fonts, because Bevy's
//! built-in face lacks glyphs like ≈ ° ² · and has no display or script cut.

use bevy::prelude::*;

pub const MONO: &str = "Menlo, SF Mono, DejaVu Sans Mono, Consolas, monospace";
pub const DISPLAY: &str = "Futura, Avenir Next, Arial Black, Helvetica Neue, sans-serif";
pub const SCRIPT: &str = "Snell Roundhand, Brush Script MT, Segoe Script, cursive";

pub const NEON_PINK: Color = Color::srgb(1.0, 0.169, 0.839);
pub const HOT_PINK: Color = Color::srgb(1.0, 0.43, 0.78);
pub const NEON_CYAN: Color = Color::srgb(0.0, 0.9, 1.0);
pub const NIGHT: Color = Color::srgb(0.043, 0.008, 0.129);
pub const DUSK: Color = Color::srgb(0.169, 0.039, 0.31);
pub const CHROME_TOP: Color = Color::srgb(0.91, 0.97, 1.0);
pub const CHROME_BOTTOM: Color = Color::srgb(1.0, 0.62, 0.9);
pub const SHEET: Color = Color::srgba(0.063, 0.012, 0.141, 0.88);

pub fn font(families: &str, size: f32, italic: bool) -> TextFont {
    TextFont {
        font: FontSource::Families(families.into()),
        font_size: FontSize::Px(size),
        style: if italic { FontStyle::Italic } else { FontStyle::Normal },
        ..default()
    }
}

pub fn mono_text(value: &str, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    (Text::new(value), font(MONO, size, false), TextColor(color))
}

pub fn display_text(value: &str, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    (Text::new(value), font(DISPLAY, size, true), TextColor(color))
}

/// A soft coloured halo around a node.
pub fn glow(color: Color, blur: f32) -> BoxShadow {
    BoxShadow::new(color, Val::ZERO, Val::ZERO, Val::ZERO, Val::Px(blur))
}
