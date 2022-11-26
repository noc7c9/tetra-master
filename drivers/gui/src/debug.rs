use bevy::prelude::*;

pub struct Plugin;

#[cfg(debug_assertions)]
pub use debug_only::*;

#[cfg(not(debug_assertions))]
impl bevy::app::Plugin for Plugin {
    fn build(&self, _: &mut App) {}
}

#[allow(dead_code)]
#[cfg(debug_assertions)]
mod debug_only {
    use super::*;
    use crate::layout::Z;
    use bevy::app::AppExit;
    use bevy_prototype_lyon::prelude::*;

    const DEFAULT_FILL: Color = Color::CYAN;
    const OPACITY: f32 = 0.1;

    impl bevy::app::Plugin for Plugin {
        fn build(&self, app: &mut App) {
            app.insert_resource(Msaa { samples: 4 })
                .add_plugin(ShapePlugin)
                .add_system(rect_initialization)
                .add_system(quit_on_ctrl_escape);
        }
    }

    fn quit_on_ctrl_escape(mut exit: EventWriter<AppExit>, keys: Res<Input<KeyCode>>) {
        let ctrl_down = keys.pressed(KeyCode::LControl) || keys.pressed(KeyCode::RControl);
        if keys.just_pressed(KeyCode::Escape) && ctrl_down {
            exit.send(AppExit)
        }
    }

    #[allow(clippy::type_complexity)]
    fn rect_initialization(
        mut commands: Commands,
        query: Query<(Entity, &RectInit)>,
        required_components: Query<(
            Option<&Transform>,
            Option<&GlobalTransform>,
            Option<&Visibility>,
            Option<&ComputedVisibility>,
        )>,
    ) {
        for (entity, rect) in &query {
            // ensure required sibling component exist
            if let Ok((transform, global_transform, visibility, computed_visibility)) =
                required_components.get(entity)
            {
                if transform.is_none() {
                    panic!("debug::rect requires the Transform component, use debug::transform");
                }
                if global_transform.is_none() {
                    panic!(
                        "debug::rect requires the GlobalTransform component, use debug::transform"
                    );
                }

                let mut commands = commands.entity(entity);
                if visibility.is_none() {
                    commands.insert(Visibility::default());
                }
                if computed_visibility.is_none() {
                    commands.insert(ComputedVisibility::default());
                }
            }

            let shape = shapes::Rectangle {
                extents: rect.size,
                origin: RectangleOrigin::BottomLeft,
            };

            let mut fill = rect.fill;
            fill.set_a(OPACITY);

            commands
                .entity(entity)
                .remove::<RectInit>()
                .with_children(|p| {
                    p.spawn(GeometryBuilder::build_as(
                        &shape,
                        DrawMode::Fill(FillMode::color(fill)),
                        Transform::from_xyz(0., 0., Z::DEBUG),
                    ));
                });
        }
    }

    #[derive(Component)]
    pub struct RectInit {
        size: Vec2,
        fill: Color,
    }

    impl RectInit {
        pub fn fill(mut self, color: Color) -> Self {
            self.fill = color;
            self
        }
    }

    pub fn rect(size: impl Into<Vec2>) -> RectInit {
        RectInit {
            size: size.into(),
            fill: DEFAULT_FILL,
        }
    }
}
