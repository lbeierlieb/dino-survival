//! The always-running dinosaurs: steering, wall bounces and shoving each other.

use bevy::prelude::*;
use std::f32::consts::PI;

use crate::assets::{GameAssets, part};
use crate::round::Match;
use crate::theme::*;

#[derive(Component)]
pub struct Dino {
    pub id: usize,
    pub heading: f32,
    /// Extra velocity from collisions; decays quickly.
    pub knock: Vec2,
}

/// Floating name tag that follows dino `0`.
#[derive(Component)]
pub struct DinoLabel(pub usize);

pub fn spawn_dino(
    commands: &mut Commands,
    assets: &GameAssets,
    materials: &mut Assets<ColorMaterial>,
    id: usize,
    pos: Vec2,
    heading: f32,
) {
    let def = &PLAYERS[id];
    let body = def.color;
    let dark = shade(body, 0.6);
    let c = &assets.circle;

    commands
        .spawn((
            Dino { id, heading, knock: Vec2::ZERO },
            Transform::from_translation(pos.extend(5.0 + id as f32 * 0.1))
                .with_rotation(Quat::from_rotation_z(heading)),
            Visibility::default(),
            DespawnOnExit(crate::AppState::Playing),
        ))
        .with_children(|p| {
            // The dino faces +X in local space.
            for (x, y) in [(12.0, 15.0), (12.0, -15.0), (-10.0, 15.0), (-10.0, -15.0)] {
                p.spawn(part(c, materials, dark, Vec2::new(x, y), 0.1, Vec2::splat(5.5), 0.0));
            }
            p.spawn(part(&assets.triangle, materials, body, Vec2::new(-32.0, 0.0), 0.2, Vec2::new(28.0, 14.0), PI));
            p.spawn(part(c, materials, body, Vec2::ZERO, 0.3, Vec2::new(24.0, 16.0), 0.0));
            for x in [-13.0, -3.0, 7.0] {
                p.spawn(part(&assets.rhombus, materials, dark, Vec2::new(x, 0.0), 0.4, Vec2::new(9.0, 7.0), 0.0));
            }
            p.spawn(part(c, materials, body, Vec2::new(24.0, 0.0), 0.5, Vec2::splat(11.0), 0.0));
            p.spawn(part(c, materials, body, Vec2::new(33.0, 0.0), 0.5, Vec2::new(8.0, 7.0), 0.0));
            for y in [6.0, -6.0] {
                p.spawn(part(c, materials, Color::WHITE, Vec2::new(27.0, y), 0.6, Vec2::splat(3.2), 0.0));
                p.spawn(part(c, materials, Color::BLACK, Vec2::new(28.3, y), 0.7, Vec2::splat(1.7), 0.0));
            }
        });

    commands.spawn((
        DinoLabel(id),
        Text2d::new(def.name),
        TextFont { font_size: FontSize::Px(15.0), ..default() },
        TextColor(def.color),
        Transform::from_translation((pos + Vec2::Y * 38.0).extend(20.0)),
        DespawnOnExit(crate::AppState::Playing),
    ));
}

pub fn steer_and_move(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    m: Res<Match>,
    mut dinos: Query<(&mut Dino, &mut Transform)>,
) {
    if !m.phase.dinos_move() {
        return;
    }
    let dt = time.delta_secs();
    let min = ARENA_CENTER - ARENA_HALF + Vec2::splat(DINO_RADIUS);
    let max = ARENA_CENTER + ARENA_HALF - Vec2::splat(DINO_RADIUS);

    for (mut dino, mut transform) in &mut dinos {
        dino.heading += PLAYERS[dino.id].turn_input(&keys) * DINO_TURN_RATE * dt;

        let velocity = Vec2::from_angle(dino.heading) * DINO_SPEED + dino.knock;
        let mut pos = transform.translation.truncate() + velocity * dt;
        dino.knock *= (-5.0 * dt).exp();

        // Bounce off the walls by mirroring the heading.
        let dir = Vec2::from_angle(dino.heading);
        if (pos.x < min.x && dir.x < 0.0) || (pos.x > max.x && dir.x > 0.0) {
            dino.heading = PI - dino.heading;
        }
        if (pos.y < min.y && dir.y < 0.0) || (pos.y > max.y && dir.y > 0.0) {
            dino.heading = -dino.heading;
        }
        if pos.x < min.x || pos.x > max.x {
            dino.knock.x *= -0.6;
        }
        if pos.y < min.y || pos.y > max.y {
            dino.knock.y *= -0.6;
        }
        pos = pos.clamp(min, max);
        dino.heading = dino.heading.rem_euclid(std::f32::consts::TAU);

        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
        transform.rotation = Quat::from_rotation_z(dino.heading);
    }
}

/// Dinos are solid: overlapping dinos get pushed apart and shove each other.
pub fn collide_dinos(m: Res<Match>, mut dinos: Query<(&mut Dino, &mut Transform)>) {
    if !m.phase.dinos_move() {
        return;
    }
    let mut pairs = dinos.iter_combinations_mut();
    while let Some([(mut a, mut ta), (mut b, mut tb)]) = pairs.fetch_next() {
        let delta = tb.translation.truncate() - ta.translation.truncate();
        let dist = delta.length();
        let overlap = 2.0 * DINO_RADIUS - dist;
        if overlap <= 0.0 {
            continue;
        }
        let normal = if dist > 0.001 { delta / dist } else { Vec2::X };
        let push = normal * overlap * 0.5;
        ta.translation -= push.extend(0.0);
        tb.translation += push.extend(0.0);

        let va = Vec2::from_angle(a.heading) * DINO_SPEED + a.knock;
        let vb = Vec2::from_angle(b.heading) * DINO_SPEED + b.knock;
        let closing = (va - vb).dot(normal);
        if closing > 0.0 {
            let impulse = normal * (closing * 0.9 + 60.0);
            a.knock -= impulse;
            b.knock += impulse;
        }
    }
}

pub fn sync_labels(
    dinos: Query<(&Dino, &Transform), Without<DinoLabel>>,
    mut labels: Query<(&DinoLabel, &mut Transform)>,
) {
    for (label, mut transform) in &mut labels {
        if let Some((_, t)) = dinos.iter().find(|(d, _)| d.id == label.0) {
            transform.translation.x = t.translation.x;
            transform.translation.y = t.translation.y + 38.0;
        }
    }
}
