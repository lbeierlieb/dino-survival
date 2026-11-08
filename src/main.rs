//! Fire, Water, Storm, Lightning: a local multiplayer dinosaur party game,
//! inspired by the German gym game "Feuer, Wasser, Sturm, Blitz".

mod assets;
mod dino;
mod hud;
mod lobby;
mod rng;
mod round;
mod spots;
mod theme;

use bevy::{camera::ScalingMode, prelude::*};

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Lobby,
    Playing,
    /// Passed through for one frame on a rematch, so `Playing` is torn down and rebuilt.
    Restart,
}

/// Which player slots joined in the lobby, and how many lives each has left.
#[derive(Resource, Default)]
pub struct Roster {
    pub joined: [bool; 4],
    pub lives: [u32; 4],
}

impl Roster {
    pub fn alive(&self) -> impl Iterator<Item = usize> + '_ {
        (0..4).filter(|&i| self.joined[i] && self.lives[i] > 0)
    }

    /// Practice mode: one player, who loses a life whenever they aren't safe.
    pub fn solo(&self) -> bool {
        self.joined.iter().filter(|&&j| j).count() == 1
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Fire, Water, Storm, Lightning".into(),
                resolution: (1280, 800).into(),
                canvas: Some("#bevy".into()),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(theme::BACKGROUND))
        .init_resource::<rng::Rng>()
        .init_resource::<Roster>()
        .init_state::<AppState>()
        .add_systems(Startup, (spawn_camera, assets::load_assets))
        .add_plugins((lobby::LobbyPlugin, round::RoundPlugin, hud::HudPlugin))
        .add_systems(OnEnter(AppState::Restart), |mut next: ResMut<NextState<AppState>>| {
            next.set(AppState::Playing)
        })
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: theme::VIEW_SIZE.x,
                min_height: theme::VIEW_SIZE.y,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}
