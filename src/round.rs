//! The match flow: get ready -> roam -> call -> verdict -> (next round | winner).

use bevy::prelude::*;

use crate::assets::GameAssets;
use crate::dino::{self, Dino, DinoLabel};
use crate::rng::Rng;
use crate::spots::{self, SafeSpot};
use crate::theme::*;
use crate::{AppState, Roster};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// Countdown before the first round; everyone frozen.
    Ready,
    /// Running around, waiting for the next call.
    Roam,
    /// An element was called; race to a matching safe spot.
    Call,
    /// Time's up: show who was furthest away.
    Verdict,
    /// One dino (or none) left.
    Finished,
}

impl Phase {
    pub fn dinos_move(self) -> bool {
        matches!(self, Phase::Roam | Phase::Call | Phase::Finished)
    }
}

#[derive(Resource)]
pub struct Match {
    pub round: u32,
    pub phase: Phase,
    /// Seconds left in the current phase.
    pub timer: f32,
    pub duration: f32,
    pub element: Element,
    /// Verdict text, e.g. "REX loses a life".
    pub message: String,
    /// Verdict lines from each dino to its nearest safe spot (from, to, lost a life).
    pub lines: Vec<(Vec2, Vec2, bool)>,
    pub winner: Option<usize>,
}

impl Default for Match {
    fn default() -> Self {
        Match {
            round: 1,
            phase: Phase::Ready,
            timer: 3.0,
            duration: 3.0,
            element: Element::Fire,
            message: String::new(),
            lines: Vec::new(),
            winner: None,
        }
    }
}

impl Match {
    fn enter(&mut self, phase: Phase, duration: f32) {
        self.phase = phase;
        self.timer = duration;
        self.duration = duration;
    }

    /// The countdown gets shorter every round.
    fn call_duration(&self) -> f32 {
        (3.6 - 0.12 * (self.round - 1) as f32).max(1.8)
    }
}

/// Text that drifts up and fades out over the verdict ("SAFE", "-1 LIFE").
#[derive(Component)]
struct Floater {
    age: f32,
}

pub struct RoundPlugin;

impl Plugin for RoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Match>()
            .add_systems(OnEnter(AppState::Playing), setup_match)
            .add_systems(
                Update,
                (
                    tick_phase,
                    dino::steer_and_move,
                    dino::collide_dinos,
                    dino::animate_dinos,
                    spots::move_spots,
                    spots::animate_spots,
                    dino::sync_labels,
                    animate_floaters,
                    draw_verdict,
                    menu_keys,
                )
                    .chain()
                    .run_if(in_state(AppState::Playing)),
            );
    }
}

fn setup_match(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut rng: ResMut<Rng>,
    mut roster: ResMut<Roster>,
    mut m: ResMut<Match>,
) {
    *m = Match::default();
    spawn_arena(&mut commands, &assets, &mut materials, &mut rng);

    let corners = [
        Vec2::new(-1.0, 1.0),
        Vec2::new(1.0, -1.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(-1.0, -1.0),
    ];
    let mut slot = 0;
    for id in 0..4 {
        roster.lives[id] = if roster.joined[id] { STARTING_LIVES } else { 0 };
        if !roster.joined[id] {
            continue;
        }
        let pos = ARENA_CENTER + corners[slot] * (ARENA_HALF - Vec2::splat(110.0));
        let heading = (ARENA_CENTER - pos).to_angle();
        dino::spawn_dino(&mut commands, &assets, &mut materials, id, pos, heading);
        slot += 1;
    }
    spots::spawn_spots(&mut commands, &assets, &mut materials, &mut rng, m.round);
}

fn spawn_arena(
    commands: &mut Commands,
    assets: &GameAssets,
    materials: &mut Assets<ColorMaterial>,
    rng: &mut Rng,
) {
    let scoped = DespawnOnExit(AppState::Playing);
    commands.spawn((
        crate::assets::part(
            &assets.square,
            materials,
            WALL,
            ARENA_CENTER,
            -2.0,
            ARENA_HALF * 2.0 + Vec2::splat(28.0),
            0.0,
        ),
        scoped.clone(),
    ));
    commands.spawn((
        crate::assets::part(
            &assets.square,
            materials,
            GROUND,
            ARENA_CENTER,
            -1.9,
            ARENA_HALF * 2.0,
            0.0,
        ),
        scoped.clone(),
    ));
    // Some patchy texture on the ground.
    for _ in 0..40 {
        let pos = ARENA_CENTER
            + Vec2::new(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0))
                * (ARENA_HALF - Vec2::splat(30.0));
        let size = Vec2::new(rng.range(18.0, 50.0), rng.range(10.0, 28.0));
        let color = shade(GROUND, rng.range(0.90, 0.97));
        commands.spawn((
            crate::assets::part(
                &assets.circle,
                materials,
                color,
                pos,
                -1.8,
                size,
                rng.angle(),
            ),
            scoped.clone(),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn tick_phase(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<GameAssets>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut rng: ResMut<Rng>,
    mut roster: ResMut<Roster>,
    mut m: ResMut<Match>,
    dinos: Query<(Entity, &Dino, &Transform)>,
    labels: Query<(Entity, &DinoLabel)>,
    spot_query: Query<(Entity, &Transform, &SafeSpot)>,
) {
    m.timer -= time.delta_secs();
    if m.timer > 0.0 {
        return;
    }

    match m.phase {
        Phase::Ready => {
            let roam = rng.range(1.0, 2.5);
            m.enter(Phase::Roam, roam);
        }
        Phase::Roam => {
            // Re-roll once to make immediate repeats less likely.
            let mut element = Element::ALL[rng.index(4)];
            if element == m.element {
                element = Element::ALL[rng.index(4)];
            }
            m.element = element;
            let duration = m.call_duration();
            m.enter(Phase::Call, duration);
        }
        Phase::Call => {
            resolve_call(&mut commands, &mut roster, &mut m, &dinos, &spot_query);
            m.enter(Phase::Verdict, 2.4);
        }
        Phase::Verdict => {
            for (entity, d, _) in &dinos {
                if roster.lives[d.id] == 0 {
                    commands.entity(entity).despawn();
                }
            }
            for (entity, label) in &labels {
                if roster.lives[label.0] == 0 {
                    commands.entity(entity).despawn();
                }
            }
            m.lines.clear();

            let alive: Vec<usize> = roster.alive().collect();
            let over = if roster.solo() {
                alive.is_empty()
            } else {
                alive.len() <= 1
            };
            if over {
                m.winner = alive.first().copied();
                m.enter(Phase::Finished, f32::INFINITY);
                return;
            }

            m.round += 1;
            for (entity, _, _) in &spot_query {
                commands.entity(entity).despawn();
            }
            spots::spawn_spots(&mut commands, &assets, &mut materials, &mut rng, m.round);
            let roam = rng.range(1.5, 4.0);
            m.enter(Phase::Roam, roam);
        }
        Phase::Finished => {}
    }
}

/// The dino(s) furthest from a matching safe spot lose a life. Anyone inside one is safe.
fn resolve_call(
    commands: &mut Commands,
    roster: &mut Roster,
    m: &mut Match,
    dinos: &Query<(Entity, &Dino, &Transform)>,
    spot_query: &Query<(Entity, &Transform, &SafeSpot)>,
) {
    let results: Vec<(usize, Vec2, f32, Vec2)> = dinos
        .iter()
        .map(|(_, d, t)| {
            let pos = t.translation.truncate();
            let spots = spot_query
                .iter()
                .map(|(_, st, s)| (st.translation.truncate(), s));
            let (dist, nearest) = spots::distance_to_safety(pos, m.element, spots);
            (d.id, pos, dist, nearest)
        })
        .collect();

    let worst = results.iter().map(|r| r.2).fold(0.0, f32::max);
    let solo = roster.solo();
    let mut losers = Vec::new();
    let mut knocked_out = Vec::new();
    m.lines.clear();
    for &(id, pos, dist, nearest) in &results {
        // Solo practice has nobody to be furthest from, so being outside a spot costs a life.
        let lost = if solo {
            dist > 0.0
        } else {
            worst > 0.5 && dist >= worst - 0.5
        };
        m.lines.push((pos, nearest, lost));
        let (text, color) = if lost {
            roster.lives[id] = roster.lives[id].saturating_sub(1);
            losers.push(PLAYERS[id].name);
            if roster.lives[id] == 0 {
                knocked_out.push(PLAYERS[id].name);
                ("OUT!", DANGER)
            } else {
                ("-1 LIFE", DANGER)
            }
        } else if dist <= 0.0 {
            ("SAFE", SAFE)
        } else {
            continue;
        };
        commands.spawn((
            Floater { age: 0.0 },
            Text2d::new(text),
            TextFont {
                font_size: FontSize::Px(22.0),
                ..default()
            },
            TextColor(color),
            Transform::from_translation((pos + Vec2::Y * floater_offset(pos)).extend(30.0)),
            DespawnOnExit(AppState::Playing),
        ));
    }

    m.message = match losers.len() {
        0 if solo => "Made it!".to_string(),
        1 if solo => "Not safe!".to_string(),
        0 => "Everyone made it!".to_string(),
        1 => format!("{} was furthest away", losers[0]),
        _ => format!("{} tied for last", join_names(&losers)),
    };
    if !knocked_out.is_empty() {
        let verb = if knocked_out.len() == 1 { "is" } else { "are" };
        m.message += &format!("  -  {} {verb} OUT!", join_names(&knocked_out));
    }
}

/// Above the dino, unless that would hide it under the banner at the top of the arena.
fn floater_offset(pos: Vec2) -> f32 {
    if pos.y > ARENA_CENTER.y + ARENA_HALF.y - 120.0 {
        -55.0
    } else {
        58.0
    }
}

fn join_names(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn animate_floaters(
    mut commands: Commands,
    time: Res<Time>,
    mut floaters: Query<(Entity, &mut Floater, &mut Transform, &mut TextColor)>,
) {
    let dt = time.delta_secs();
    for (entity, mut floater, mut transform, mut color) in &mut floaters {
        floater.age += dt;
        transform.translation.y += 30.0 * dt;
        color
            .0
            .set_alpha((1.0 - (floater.age - 1.4).max(0.0) / 0.8).max(0.0));
        if floater.age > 2.2 {
            commands.entity(entity).despawn();
        }
    }
}

fn draw_verdict(m: Res<Match>, mut gizmos: Gizmos) {
    if m.phase != Phase::Verdict {
        return;
    }
    for &(from, to, lost) in &m.lines {
        let color = if lost { DANGER } else { SAFE };
        gizmos.line_2d(from, to, color);
        gizmos.circle_2d(Isometry2d::from_translation(to), 4.0, color);
    }
}

fn menu_keys(
    keys: Res<ButtonInput<KeyCode>>,
    m: Res<Match>,
    mut next: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next.set(AppState::Lobby);
    } else if m.phase == Phase::Finished && keys.any_just_pressed([KeyCode::Space, KeyCode::Enter])
    {
        next.set(AppState::Restart);
    }
}
