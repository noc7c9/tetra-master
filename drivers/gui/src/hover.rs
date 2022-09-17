use bevy::prelude::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_event::<StartEvent>()
            .add_event::<EndEvent>()
            .add_system(system);
    }
}

#[derive(Debug)]
pub struct StartEvent {
    pub entity: Entity,
}

#[derive(Debug)]
pub struct EndEvent {
    pub entity: Entity,
}

#[derive(Debug, Component)]
pub struct Area {
    pub is_hovered: bool,
    bounding_box: (Vec2, Vec2),
}

impl Area {
    pub fn new(position: Vec2, size: Vec2) -> Self {
        Self {
            is_hovered: false,
            bounding_box: (position, position + size),
        }
    }

    fn contains(&self, point: Vec2) -> bool {
        let a = self.bounding_box.0;
        let b = self.bounding_box.1;
        (a.x..b.x).contains(&point.x) && (a.y..b.y).contains(&point.y)
    }
}

fn system(
    mut start_event: EventWriter<StartEvent>,
    mut end_event: EventWriter<EndEvent>,
    mut cursor_moved: EventReader<CursorMoved>,
    windows: Res<Windows>,
    mut hoverables: Query<(Entity, &mut Area)>,
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = match camera.get_single().ok() {
        Some(res) => res,
        None => return,
    };

    let window = windows.get_primary().unwrap();

    let evt = match cursor_moved.iter().last() {
        None => return,
        Some(evt) => evt,
    };

    let screen_pos = evt.position;
    let screen_size = Vec2::new(window.width() as f32, window.height() as f32);

    // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
    let ndc = (screen_pos / screen_size) * 2.0 - Vec2::ONE;

    // matrix for undoing the projection and camera transform
    let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();

    // use it to convert ndc to world-space coordinates
    let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

    // reduce it to a 2D value
    let world_pos: Vec2 = world_pos.truncate();

    // iterate over all hoverables to check which are hovered over
    for (entity, mut hoverable) in &mut hoverables {
        let is_hovered = hoverable.contains(world_pos);
        // avoid triggering change detection if it's the same
        if hoverable.is_hovered != is_hovered {
            hoverable.is_hovered = is_hovered;

            if is_hovered {
                start_event.send(StartEvent { entity });
            } else {
                end_event.send(EndEvent { entity });
            }
        }
    }
}
