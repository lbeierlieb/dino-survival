//! Unit-sized meshes shared by everything; parts are sized with `Transform::scale`.

use bevy::prelude::*;

#[derive(Resource)]
pub struct GameAssets {
    /// Radius 1.
    pub circle: Handle<Mesh>,
    /// 1 x 1.
    pub square: Handle<Mesh>,
    /// Length 1 along +X, width 1, tip pointing to +X.
    pub triangle: Handle<Mesh>,
    /// 1 x 1 diamond.
    pub rhombus: Handle<Mesh>,
    /// Outer radius 1, inner radius 0.9.
    pub ring: Handle<Mesh>,
}

pub fn load_assets(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(GameAssets {
        circle: meshes.add(Circle::new(1.0).mesh().resolution(48)),
        square: meshes.add(Rectangle::new(1.0, 1.0)),
        triangle: meshes.add(Triangle2d::new(
            Vec2::new(0.5, 0.0),
            Vec2::new(-0.5, 0.5),
            Vec2::new(-0.5, -0.5),
        )),
        rhombus: meshes.add(Rhombus::new(1.0, 1.0)),
        ring: meshes.add(Annulus::new(0.9, 1.0).mesh().resolution(64)),
    });
}

/// A colored mesh child placed at `pos` (local), with a non-uniform `scale` and rotation `rot`.
pub fn part(
    mesh: &Handle<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    color: Color,
    pos: Vec2,
    z: f32,
    scale: Vec2,
    rot: f32,
) -> impl Bundle {
    (
        Mesh2d(mesh.clone()),
        MeshMaterial2d(materials.add(color)),
        Transform {
            translation: pos.extend(z),
            rotation: Quat::from_rotation_z(rot),
            scale: scale.extend(1.0),
        },
    )
}
