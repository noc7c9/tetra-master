use bevy::{app::AppExit, prelude::*};
use bevy_prototype_lyon::prelude::*;

const Z: f32 = 999.9;
const DEFAULT_FILL: Color = Color::CYAN;
const DEFAULT_OPACITY: f32 = 0.1;

pub struct Plugin;

#[cfg(debug_assertions)]
pub use debug_only::*;

#[allow(dead_code)]
#[cfg(debug_assertions)]
mod debug_only {
    use super::*;

    impl bevy::app::Plugin for Plugin {
        fn build(&self, app: &mut App) {
            app.insert_resource(Msaa { samples: 4 })
                .add_plugin(ShapePlugin)
                .add_system(rect_on_add)
                .add_system(quit_on_ctrl_escape);
        }
    }

    fn quit_on_ctrl_escape(mut exit: EventWriter<AppExit>, keys: Res<Input<KeyCode>>) {
        let ctrl_down = keys.pressed(KeyCode::LControl) || keys.pressed(KeyCode::RControl);
        if keys.just_pressed(KeyCode::Escape) && ctrl_down {
            exit.send(AppExit)
        }
    }

    fn rect_on_add(mut commands: Commands, query: Query<(Entity, &RectInit)>) {
        for (entity, rect) in &query {
            let shape = shapes::Rectangle {
                extents: rect.size,
                origin: RectangleOrigin::BottomLeft,
            };

            let mut fill = DEFAULT_FILL;
            fill.set_a(DEFAULT_OPACITY);

            commands
                .entity(entity)
                .remove::<RectInit>()
                .insert_bundle(GeometryBuilder::build_as(
                    &shape,
                    DrawMode::Fill(FillMode::color(fill)),
                    Transform::from_translation(rect.position.extend(Z)),
                ));
        }
    }

    #[derive(Component)]
    pub struct RectInit {
        position: Vec2,
        size: Vec2,
    }

    pub fn rect(position: impl Into<Vec2>, size: impl Into<Vec2>) -> RectInit {
        RectInit {
            position: position.into(),
            size: size.into(),
        }
    }
}
