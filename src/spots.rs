//! Safe spots: lakes, mountains, caves and ferns. Re-rolled every round; some of them wander.

use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::assets::{GameAssets, part};
use crate::rng::Rng;
use crate::round::{Match, Phase};
use crate::theme::*;

#[derive(Component)]
pub struct SafeSpot {
    pub element: Element,
    pub radius: f32,
    pub velocity: Vec2,
    /// Stacking order among the spots, 0 at the bottom. Called spots are drawn above the rest.
    pub layer: usize,
}

/// Each spot gets its own z band (its parts span local z 0.0..=0.5), so overlapping spots
/// draw one on top of the other instead of interleaving. All stay below the dinos.
fn layer_z(rank: usize) -> f32 {
    0.5 + rank as f32 * 0.55
}

/// Seconds since the spot appeared, for the pop-in animation.
#[derive(Component, Default)]
pub struct SpotAge(f32);

pub fn spawn_spots(
    commands: &mut Commands,
    assets: &GameAssets,
    materials: &mut Assets<ColorMaterial>,
    rng: &mut Rng,
    round: u32,
) {
    let r = SPOT_RADIUS;
    let min = ARENA_CENTER - ARENA_HALF + Vec2::splat(r + 8.0);
    let max = ARENA_CENTER + ARENA_HALF - Vec2::splat(r + 8.0);

    let mut kinds: Vec<Element> = Element::ALL
        .iter()
        .flat_map(|&e| std::iter::repeat_n(e, SPOTS_PER_ELEMENT))
        .collect();
    // Shuffle so the moving spots are a random mix of kinds.
    for i in (1..kinds.len()).rev() {
        kinds.swap(i, rng.index(i + 1));
    }
    // More spots start wandering as the game goes on.
    let movers = (1 + round as usize / 2).min(kinds.len() / 2);

    let mut placed: Vec<Vec2> = Vec::new();
    for (i, element) in kinds.into_iter().enumerate() {
        // Rejection sampling for a spot that doesn't overlap the others; relax if crowded.
        let mut pos = Vec2::ZERO;
        for attempt in 0..300 {
            pos = Vec2::new(rng.range(min.x, max.x), rng.range(min.y, max.y));
            let gap = if attempt < 200 {
                2.0 * r + 40.0
            } else {
                2.0 * r
            };
            if placed.iter().all(|p| p.distance(pos) > gap) {
                break;
            }
        }
        placed.push(pos);

        let velocity = if i < movers {
            Vec2::from_angle(rng.angle()) * rng.range(45.0, 80.0 + round as f32 * 4.0)
        } else {
            Vec2::ZERO
        };
        spawn_spot(commands, assets, materials, element, pos, velocity, i);
    }
}

fn spawn_spot(
    commands: &mut Commands,
    assets: &GameAssets,
    materials: &mut Assets<ColorMaterial>,
    element: Element,
    pos: Vec2,
    velocity: Vec2,
    layer: usize,
) {
    let r = SPOT_RADIUS;
    let c = &assets.circle;
    let tri = &assets.triangle;
    commands
        .spawn((
            SafeSpot {
                element,
                radius: r,
                velocity,
                layer,
            },
            SpotAge::default(),
            Transform::from_translation(pos.extend(layer_z(layer)))
                .with_scale(Vec3::new(0.01, 0.01, 1.0)),
            Visibility::default(),
            DespawnOnExit(crate::AppState::Playing),
        ))
        .with_children(|p| {
            p.spawn(part(
                &assets.ring,
                materials,
                element.color(),
                Vec2::ZERO,
                0.0,
                Vec2::splat(r + 6.0),
                0.0,
            ));
            match element {
                Element::Fire => {
                    // Lake
                    p.spawn(part(
                        c,
                        materials,
                        Color::srgb(0.13, 0.38, 0.75),
                        Vec2::ZERO,
                        0.1,
                        Vec2::splat(r),
                        0.0,
                    ));
                    p.spawn(part(
                        c,
                        materials,
                        Color::srgb(0.22, 0.55, 0.92),
                        Vec2::new(0.0, -0.05 * r),
                        0.2,
                        Vec2::splat(0.72 * r),
                        0.0,
                    ));
                    for (x, y, w) in [(-0.2, 0.3, 0.32), (0.25, -0.15, 0.22), (-0.1, -0.4, 0.18)] {
                        p.spawn(part(
                            c,
                            materials,
                            Color::srgb(0.65, 0.85, 1.0),
                            Vec2::new(x, y) * r,
                            0.3,
                            Vec2::new(w, 0.05) * r,
                            0.0,
                        ));
                    }
                }
                Element::Water => {
                    // Mountain
                    p.spawn(part(
                        c,
                        materials,
                        Color::srgb(0.38, 0.34, 0.30),
                        Vec2::ZERO,
                        0.1,
                        Vec2::splat(r),
                        0.0,
                    ));
                    p.spawn(part(
                        tri,
                        materials,
                        Color::srgb(0.50, 0.47, 0.45),
                        Vec2::new(0.32, -0.25) * r,
                        0.2,
                        Vec2::new(0.8, 0.9) * r,
                        FRAC_PI_2,
                    ));
                    p.spawn(part(
                        tri,
                        materials,
                        Color::srgb(0.60, 0.57, 0.54),
                        Vec2::new(-0.18, -0.08) * r,
                        0.3,
                        Vec2::new(1.1, 1.1) * r,
                        FRAC_PI_2,
                    ));
                    p.spawn(part(
                        tri,
                        materials,
                        Color::srgb(0.96, 0.96, 0.98),
                        Vec2::new(-0.18, 0.31) * r,
                        0.4,
                        Vec2::new(0.32, 0.32) * r,
                        FRAC_PI_2,
                    ));
                }
                Element::Storm => {
                    // Cave
                    p.spawn(part(
                        c,
                        materials,
                        Color::srgb(0.36, 0.27, 0.20),
                        Vec2::ZERO,
                        0.1,
                        Vec2::splat(r),
                        0.0,
                    ));
                    for (x, y, s) in [
                        (-0.6, 0.45, 0.3),
                        (0.55, 0.5, 0.26),
                        (0.0, 0.62, 0.24),
                        (-0.7, -0.3, 0.2),
                        (0.68, -0.3, 0.22),
                    ] {
                        p.spawn(part(
                            c,
                            materials,
                            Color::srgb(0.28, 0.21, 0.16),
                            Vec2::new(x, y) * r,
                            0.2,
                            Vec2::splat(s * r),
                            0.0,
                        ));
                    }
                    p.spawn(part(
                        c,
                        materials,
                        Color::srgb(0.07, 0.05, 0.05),
                        Vec2::new(0.0, -0.12) * r,
                        0.3,
                        Vec2::new(0.55, 0.45) * r,
                        0.0,
                    ));
                    p.spawn(part(
                        &assets.square,
                        materials,
                        Color::srgb(0.07, 0.05, 0.05),
                        Vec2::new(0.0, -0.45) * r,
                        0.3,
                        Vec2::new(1.1, 0.5) * r,
                        0.0,
                    ));
                }
                Element::Lightning => {
                    // Ferns
                    p.spawn(part(
                        c,
                        materials,
                        Color::srgb(0.12, 0.33, 0.14),
                        Vec2::ZERO,
                        0.1,
                        Vec2::splat(r),
                        0.0,
                    ));
                    for k in 0..7 {
                        let a = k as f32 * PI * 2.0 / 7.0 + 0.3;
                        let shade = if k % 2 == 0 {
                            Color::srgb(0.25, 0.62, 0.25)
                        } else {
                            Color::srgb(0.32, 0.72, 0.30)
                        };
                        p.spawn(part(
                            c,
                            materials,
                            shade,
                            Vec2::from_angle(a) * 0.45 * r,
                            0.2,
                            Vec2::new(0.48, 0.14) * r,
                            a,
                        ));
                    }
                    p.spawn(part(
                        c,
                        materials,
                        Color::srgb(0.18, 0.48, 0.20),
                        Vec2::ZERO,
                        0.3,
                        Vec2::splat(0.18 * r),
                        0.0,
                    ));
                }
            }
            p.spawn((
                Text2d::new(element.spot_name()),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(element.color()),
                Transform::from_xyz(0.0, -r - 18.0, 0.5),
            ));
        });
}

/// Distance from `pos` to the edge of the nearest spot of `element` (0 when inside),
/// together with the closest point of that spot.
pub fn distance_to_safety<'a>(
    pos: Vec2,
    element: Element,
    spots: impl Iterator<Item = (Vec2, &'a SafeSpot)>,
) -> (f32, Vec2) {
    spots
        .filter(|(_, s)| s.element == element)
        .map(|(center, s)| {
            let offset = pos - center;
            let gap = offset.length() - s.radius;
            if gap <= 0.0 {
                (0.0, pos)
            } else {
                (gap, center + offset.normalize() * s.radius)
            }
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap_or((f32::INFINITY, pos))
}

pub fn move_spots(
    time: Res<Time>,
    m: Res<Match>,
    mut spots: Query<(&mut SafeSpot, &mut Transform)>,
) {
    if !matches!(m.phase, Phase::Roam | Phase::Call) {
        return;
    }
    let dt = time.delta_secs();
    for (mut spot, mut transform) in &mut spots {
        if spot.velocity == Vec2::ZERO {
            continue;
        }
        let min = ARENA_CENTER - ARENA_HALF + Vec2::splat(spot.radius);
        let max = ARENA_CENTER + ARENA_HALF - Vec2::splat(spot.radius);
        let mut pos = transform.translation.truncate() + spot.velocity * dt;
        if pos.x < min.x || pos.x > max.x {
            spot.velocity.x = -spot.velocity.x;
        }
        if pos.y < min.y || pos.y > max.y {
            spot.velocity.y = -spot.velocity.y;
        }
        pos = pos.clamp(min, max);
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
    }
}

/// Pop in when spawned; the spots matching the current call pulse and move to the top.
pub fn animate_spots(
    time: Res<Time>,
    m: Res<Match>,
    mut spots: Query<(&SafeSpot, &mut SpotAge, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    let called = |spot: &SafeSpot| m.phase == Phase::Call && spot.element == m.element;

    // Called spots take the top z bands; otherwise spawn order decides.
    let mut spots: Vec<_> = spots.iter_mut().collect();
    spots.sort_by_key(|&(spot, ..)| (called(spot), spot.layer));

    for (rank, (spot, mut age, mut transform)) in spots.into_iter().enumerate() {
        age.0 += time.delta_secs();
        let k = (age.0 / 0.35).min(1.0);
        // Ease-out with a little overshoot.
        let pop = 1.0 + 2.2 * (k - 1.0).powi(3) + 1.2 * (k - 1.0).powi(2);
        let pulse = if called(spot) {
            1.0 + 0.07 * (t * 10.0).sin()
        } else {
            1.0
        };
        // Leave z unscaled so a spot's parts stay inside its z band.
        let s = (pop * pulse).max(0.01);
        transform.scale = Vec3::new(s, s, 1.0);
        transform.translation.z = layer_z(rank);
    }
}
