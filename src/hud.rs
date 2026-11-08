//! In-game overlay: player panels with lives, and the call banner with its countdown bar.

use bevy::prelude::*;

use crate::round::{Match, Phase};
use crate::theme::*;
use crate::{AppState, Roster};

#[derive(Component)]
struct LifePip {
    player: usize,
    index: u32,
}

#[derive(Component)]
struct PanelName(usize);

#[derive(Component)]
struct BannerTitle;

#[derive(Component)]
struct BannerSub;

#[derive(Component)]
struct BarTrack;

#[derive(Component)]
struct BarFill;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Playing), spawn_hud)
            .add_systems(
                Update,
                (update_panels, update_banner).run_if(in_state(AppState::Playing)),
            );
    }
}

fn text(value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

fn spawn_hud(mut commands: Commands, roster: Res<Roster>) {
    let panel_bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5));

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(px(10)),
                row_gap: px(8),
                ..default()
            },
            DespawnOnExit(AppState::Playing),
        ))
        .with_children(|root| {
            // Player panels.
            root.spawn(Node {
                width: percent(100),
                justify_content: JustifyContent::SpaceEvenly,
                ..default()
            })
            .with_children(|row| {
                for id in (0..4).filter(|&i| roster.joined[i]) {
                    let def = &PLAYERS[id];
                    row.spawn((
                        Node {
                            align_items: AlignItems::Center,
                            column_gap: px(10),
                            padding: UiRect::axes(px(14), px(6)),
                            border_radius: BorderRadius::all(px(8)),
                            ..default()
                        },
                        panel_bg,
                    ))
                    .with_children(|panel| {
                        panel.spawn((text(def.name, 22.0, def.color), PanelName(id)));
                        panel.spawn(text(def.keys_label, 13.0, TEXT_DIM));
                        for index in 0..STARTING_LIVES {
                            panel.spawn((
                                Node {
                                    width: px(14),
                                    height: px(14),
                                    border_radius: BorderRadius::MAX,
                                    ..default()
                                },
                                BackgroundColor(DANGER),
                                LifePip { player: id, index },
                            ));
                        }
                    });
                }
            });

            // Call banner.
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(4),
                    padding: UiRect::axes(px(28), px(8)),
                    min_width: px(440),
                    border_radius: BorderRadius::all(px(12)),
                    ..default()
                },
                panel_bg,
            ))
            .with_children(|banner| {
                banner.spawn((text("", 44.0, TEXT), BannerTitle));
                banner.spawn((text("", 20.0, TEXT), BannerSub));
                banner
                    .spawn((
                        Node {
                            width: px(380),
                            height: px(10),
                            margin: UiRect::top(px(4)),
                            border_radius: BorderRadius::all(px(5)),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.15)),
                        BarTrack,
                    ))
                    .with_child((
                        Node {
                            width: percent(100),
                            height: percent(100),
                            ..default()
                        },
                        BackgroundColor(TEXT),
                        BarFill,
                    ));
            });
        });
}

fn update_panels(
    roster: Res<Roster>,
    mut pips: Query<(&LifePip, &mut BackgroundColor)>,
    mut names: Query<(&PanelName, &mut TextColor)>,
) {
    for (pip, mut bg) in &mut pips {
        bg.0 = if pip.index < roster.lives[pip.player] {
            DANGER
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.12)
        };
    }
    for (name, mut color) in &mut names {
        color.0 = if roster.lives[name.0] > 0 {
            PLAYERS[name.0].color
        } else {
            TEXT_DIM
        };
    }
}

#[allow(clippy::type_complexity)]
fn update_banner(
    m: Res<Match>,
    roster: Res<Roster>,
    mut title: Query<(&mut Text, &mut TextColor), With<BannerTitle>>,
    mut sub: Query<(&mut Text, &mut TextColor), (With<BannerSub>, Without<BannerTitle>)>,
    mut track: Query<&mut Node, (With<BarTrack>, Without<BarFill>)>,
    mut fill: Query<(&mut Node, &mut BackgroundColor), With<BarFill>>,
) {
    let (Ok((mut title, mut title_color)), Ok((mut sub, mut sub_color))) =
        (title.single_mut(), sub.single_mut())
    else {
        return;
    };

    let (t, tc, s, sc, bar): (String, Color, String, Color, Option<Color>) = match m.phase {
        Phase::Ready => (
            format!("{}", m.timer.ceil().max(1.0)),
            TEXT,
            "Get ready! Steer with your two keys".into(),
            TEXT_DIM,
            Some(TEXT),
        ),
        Phase::Roam => (
            format!("ROUND {}", m.round),
            TEXT_DIM,
            "Keep moving... listen for the call!".into(),
            TEXT_DIM,
            None,
        ),
        Phase::Call => (
            m.element.call().into(),
            m.element.color(),
            m.element.instruction().into(),
            TEXT,
            Some(m.element.color()),
        ),
        Phase::Verdict => ("TIME!".into(), TEXT, m.message.clone(), TEXT, None),
        Phase::Finished => match m.winner {
            Some(id) => (
                format!("{} WINS!", PLAYERS[id].name),
                PLAYERS[id].color,
                "SPACE: rematch    ESC: back to lobby".into(),
                TEXT_DIM,
                None,
            ),
            None if roster.solo() => (
                "GAME OVER".into(),
                DANGER,
                format!(
                    "You reached round {}    SPACE: try again    ESC: back to lobby",
                    m.round
                ),
                TEXT_DIM,
                None,
            ),
            None => (
                "NOBODY SURVIVED".into(),
                DANGER,
                "SPACE: rematch    ESC: back to lobby".into(),
                TEXT_DIM,
                None,
            ),
        },
    };

    if title.0 != t {
        title.0 = t;
    }
    if sub.0 != s {
        sub.0 = s;
    }
    title_color.0 = tc;
    sub_color.0 = sc;

    if let (Ok(mut track), Ok((mut fill, mut fill_color))) = (track.single_mut(), fill.single_mut())
    {
        track.display = if bar.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if let Some(color) = bar {
            fill_color.0 = color;
            fill.width = percent((m.timer / m.duration).clamp(0.0, 1.0) * 100.0);
        }
    }
}
