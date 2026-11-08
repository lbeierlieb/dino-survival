//! Title screen: rules, and 1-4 players join (1 = solo practice) by pressing one of their steering keys.

use bevy::prelude::*;

use crate::rng::Rng;
use crate::theme::*;
use crate::{AppState, Roster};

#[derive(Component)]
struct SlotCard(usize);

#[derive(Component)]
struct SlotStatus(usize);

#[derive(Component)]
struct StartPrompt;

pub struct LobbyPlugin;

impl Plugin for LobbyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Lobby), spawn_lobby).add_systems(
            Update,
            (lobby_input, update_lobby).chain().run_if(in_state(AppState::Lobby)),
        );
    }
}

fn text(value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont { font_size: FontSize::Px(size), ..default() },
        TextColor(color),
    )
}

fn span(value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        TextSpan::new(value),
        TextFont { font_size: FontSize::Px(size), ..default() },
        TextColor(color),
    )
}

fn spawn_lobby(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(18),
                ..default()
            },
            DespawnOnExit(AppState::Lobby),
        ))
        .with_children(|root| {
            root.spawn(text("", 54.0, TEXT)).with_children(|title| {
                for (i, e) in Element::ALL.iter().enumerate() {
                    let word = e.call().trim_end_matches('!');
                    let sep = if i < 3 { "  " } else { "" };
                    title.spawn(span(format!("{word}{sep}"), 54.0, e.color()));
                }
            });
            root.spawn(text("a dinosaur party game for 2-4 players on one keyboard (or practice solo)", 22.0, TEXT_DIM));

            // Rules.
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(8),
                    padding: UiRect::all(px(18)),
                    border_radius: BorderRadius::all(px(12)),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.05)),
            ))
            .with_children(|rules| {
                rules.spawn(text("Your dino never stops running. You can only steer it left and right.", 18.0, TEXT));
                rules.spawn(text("When an element is called, race to the matching safe spot:", 18.0, TEXT));
                rules
                    .spawn(Node { column_gap: px(28), margin: UiRect::vertical(px(4)), ..default() })
                    .with_children(|legend| {
                        for e in Element::ALL {
                            legend.spawn(text("", 20.0, TEXT)).with_children(|t| {
                                t.spawn(span(e.call().trim_end_matches('!'), 20.0, e.color()));
                                t.spawn(span(" -> ", 20.0, TEXT_DIM));
                                t.spawn(span(e.spot_name(), 20.0, TEXT));
                            });
                        }
                    });
                rules.spawn(text("When time runs out, the dino furthest from safety loses a life.", 18.0, TEXT));
                rules.spawn(text("Spots reshuffle every round and some wander. Shove rivals out of your way!", 18.0, TEXT));
                rules.spawn(text("Last dino standing wins.", 18.0, TEXT));
            });

            // Player slots.
            root.spawn(Node { column_gap: px(16), ..default() }).with_children(|row| {
                for (id, def) in PLAYERS.iter().enumerate() {
                    row.spawn((
                        Node {
                            width: px(230),
                            height: px(130),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            row_gap: px(8),
                            border_radius: BorderRadius::all(px(12)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.06)),
                        SlotCard(id),
                    ))
                    .with_children(|card| {
                        card.spawn(text(def.name, 30.0, def.color));
                        card.spawn(text(def.keys_label, 16.0, TEXT));
                        card.spawn((text("", 16.0, TEXT_DIM), SlotStatus(id)));
                    });
                }
            });

            root.spawn((text("", 24.0, TEXT), StartPrompt));
            root.spawn(text(
                "BACKSPACE: clear players    ESC in game: back here",
                14.0,
                TEXT_DIM,
            ));
        });
}

fn lobby_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time<Real>>,
    mut roster: ResMut<Roster>,
    mut rng: ResMut<Rng>,
    mut next: ResMut<NextState<AppState>>,
) {
    for (id, def) in PLAYERS.iter().enumerate() {
        if def.any_just_pressed(&keys) {
            roster.joined[id] = true;
        }
    }
    if keys.just_pressed(KeyCode::Backspace) {
        roster.joined = [false; 4];
    }
    let count = roster.joined.iter().filter(|&&j| j).count();
    if count >= 1 && keys.any_just_pressed([KeyCode::Space, KeyCode::Enter]) {
        // When exactly players hit start is as good a seed as any.
        rng.reseed(time.elapsed().as_nanos() as u64);
        next.set(AppState::Playing);
    }
}

fn update_lobby(
    roster: Res<Roster>,
    mut cards: Query<(&SlotCard, &mut BackgroundColor)>,
    mut statuses: Query<(&SlotStatus, &mut Text, &mut TextColor), Without<StartPrompt>>,
    mut prompt: Query<&mut Text, With<StartPrompt>>,
) {
    for (card, mut bg) in &mut cards {
        bg.0 = if roster.joined[card.0] {
            shade(PLAYERS[card.0].color, 0.35)
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.06)
        };
    }
    for (status, mut text, mut color) in &mut statuses {
        let (t, c) = if roster.joined[status.0] {
            ("READY!", SAFE)
        } else {
            ("press a key to join", TEXT_DIM)
        };
        if text.0 != t {
            text.0 = t.into();
        }
        color.0 = c;
    }
    let count = roster.joined.iter().filter(|&&j| j).count();
    if let Ok(mut prompt) = prompt.single_mut() {
        let t = match count {
            0 => "Waiting for players...".to_string(),
            1 => "1 player ready  -  press SPACE or ENTER to practice solo".to_string(),
            _ => format!("{count} players ready  -  press SPACE or ENTER to start"),
        };
        if prompt.0 != t {
            prompt.0 = t;
        }
    }
}
