//! Shared constants: arena geometry, elements, and the four player slots.

use bevy::prelude::*;

/// The world area the camera always keeps visible.
pub const VIEW_SIZE: Vec2 = Vec2::new(1280.0, 800.0);
/// Center and half-size of the playing field (room above it for the HUD).
pub const ARENA_CENTER: Vec2 = Vec2::new(0.0, -50.0);
pub const ARENA_HALF: Vec2 = Vec2::new(610.0, 300.0);

pub const STARTING_LIVES: u32 = 3;
pub const DINO_RADIUS: f32 = 22.0;
pub const DINO_SPEED: f32 = 220.0;
pub const DINO_TURN_RATE: f32 = 3.2;
pub const SPOT_RADIUS: f32 = 56.0;
pub const SPOTS_PER_ELEMENT: usize = 2;

pub const BACKGROUND: Color = Color::srgb(0.09, 0.08, 0.10);
pub const GROUND: Color = Color::srgb(0.55, 0.50, 0.32);
pub const WALL: Color = Color::srgb(0.25, 0.20, 0.14);
pub const TEXT: Color = Color::srgb(0.95, 0.93, 0.88);
pub const TEXT_DIM: Color = Color::srgb(0.60, 0.58, 0.55);
pub const SAFE: Color = Color::srgb(0.40, 0.95, 0.45);
pub const DANGER: Color = Color::srgb(1.0, 0.30, 0.25);

/// The four calls. Each one has its own kind of safe spot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Element {
    Fire,
    Water,
    Storm,
    Lightning,
}

impl Element {
    pub const ALL: [Element; 4] = [
        Element::Fire,
        Element::Water,
        Element::Storm,
        Element::Lightning,
    ];

    pub fn call(self) -> &'static str {
        match self {
            Element::Fire => "FIRE!",
            Element::Water => "WATER!",
            Element::Storm => "STORM!",
            Element::Lightning => "LIGHTNING!",
        }
    }

    pub fn spot_name(self) -> &'static str {
        match self {
            Element::Fire => "LAKE",
            Element::Water => "MOUNTAIN",
            Element::Storm => "CAVE",
            Element::Lightning => "FERNS",
        }
    }

    pub fn instruction(self) -> &'static str {
        match self {
            Element::Fire => "Cool off in a LAKE",
            Element::Water => "Flood! Climb a MOUNTAIN",
            Element::Storm => "Take shelter in a CAVE",
            Element::Lightning => "Duck low into the FERNS",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Element::Fire => Color::srgb(1.0, 0.38, 0.12),
            Element::Water => Color::srgb(0.25, 0.60, 1.0),
            Element::Storm => Color::srgb(0.72, 0.62, 0.95),
            Element::Lightning => Color::srgb(1.0, 0.90, 0.20),
        }
    }
}

pub struct PlayerDef {
    pub name: &'static str,
    pub color: Color,
    pub left: &'static [KeyCode],
    pub right: &'static [KeyCode],
    pub keys_label: &'static str,
}

pub const PLAYERS: [PlayerDef; 4] = [
    PlayerDef {
        name: "REX",
        color: Color::srgb(0.38, 0.82, 0.30),
        left: &[KeyCode::KeyA],
        right: &[KeyCode::KeyD],
        keys_label: "A / D",
    },
    PlayerDef {
        name: "TRIKE",
        color: Color::srgb(0.68, 0.42, 0.98),
        left: &[KeyCode::KeyJ],
        right: &[KeyCode::KeyL],
        keys_label: "J / L",
    },
    PlayerDef {
        name: "STEGO",
        color: Color::srgb(1.0, 0.45, 0.70),
        left: &[KeyCode::ArrowLeft],
        right: &[KeyCode::ArrowRight],
        keys_label: "LEFT / RIGHT",
    },
    PlayerDef {
        name: "RAPTOR",
        color: Color::srgb(0.30, 0.88, 0.85),
        left: &[KeyCode::Numpad4, KeyCode::Minus],
        right: &[KeyCode::Numpad6, KeyCode::Equal],
        keys_label: "NUM 4 / 6  or  - / =",
    },
];

impl PlayerDef {
    pub fn turn_input(&self, keys: &ButtonInput<KeyCode>) -> f32 {
        let left = keys.any_pressed(self.left.iter().copied());
        let right = keys.any_pressed(self.right.iter().copied());
        (left as i32 - right as i32) as f32
    }

    pub fn any_just_pressed(&self, keys: &ButtonInput<KeyCode>) -> bool {
        keys.any_just_pressed(self.left.iter().chain(self.right).copied())
    }
}

/// Darker shade of a color, for outlines, legs and spikes.
pub fn shade(color: Color, factor: f32) -> Color {
    let c = color.to_srgba();
    Color::srgb(c.red * factor, c.green * factor, c.blue * factor)
}
