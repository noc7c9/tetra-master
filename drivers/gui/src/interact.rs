use bevy::prelude::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ClickEvent>()
            .add_event::<DoubleClickEvent>()
            .add_event::<HoverStartEvent>()
            .add_event::<HoverEndEvent>()
            // .add_system(system)
            ;

        //         #[cfg(debug_assertions)]
        //         {
        //             app.add_system(require_transform_with_area);
        //         }
    }
}

#[derive(Debug)]
pub struct HoverStartEvent {
    pub entity: Entity,
}

#[derive(Debug)]
pub struct HoverEndEvent {
    pub entity: Entity,
}

#[derive(Debug)]
pub struct ClickEvent {
    pub entity: Entity,
}

#[derive(Debug)]
pub struct DoubleClickEvent {
    pub entity: Entity,
}

// #[derive(Debug, Component)]
// pub struct Area {
//     pub is_hovered: bool,
//     size: Vec2,
// }

// impl Area {
//     pub fn new(size: Vec2) -> Self {
//         Self {
//             is_hovered: false,
//             size,
//         }
//     }

//     fn contains(&self, transform: &GlobalTransform, point: Vec2) -> bool {
//         let scale = transform.compute_transform().scale.truncate();
//         let size = self.size * scale;
//         let a = transform.translation().truncate() - size / 2.;
//         let b = a + size;
//         (a.x..b.x).contains(&point.x) && (a.y..b.y).contains(&point.y)
//     }
// }

// fn system(
//     mut start_event: EventWriter<StartEvent>,
//     mut end_event: EventWriter<EndEvent>,
//     mut cursor_moved: EventReader<CursorMoved>,
//     windows: Res<Windows>,
//     mut hoverables: Query<(Entity, &GlobalTransform, &mut Area)>,
//     camera: Query<(&Camera, &GlobalTransform)>,
// ) {
//     let (camera, camera_transform) = match camera.get_single().ok() {
//         Some(res) => res,
//         None => return,
//     };

//     let window = windows.get_primary().unwrap();

//     let evt = match cursor_moved.iter().last() {
//         None => return,
//         Some(evt) => evt,
//     };

//     let screen_pos = evt.position;
//     let screen_size = Vec2::new(window.width(), window.height());

//     // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
//     let ndc = (screen_pos / screen_size) * 2.0 - Vec2::ONE;

//     // matrix for undoing the projection and camera transform
//     let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();

//     // use it to convert ndc to world-space coordinates
//     let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

//     // reduce it to a 2D value
//     let world_pos: Vec2 = world_pos.truncate();

//     // end hover on all areas that are no longer hovered over
//     let mut hovered_areas = Vec::new();
//     for (entity, transform, mut hoverable) in &mut hoverables {
//         let is_hovered = hoverable.contains(transform, world_pos);

//         // capture hovered over areas for further processing
//         if is_hovered {
//             hovered_areas.push((entity, transform, hoverable));
//         }
//         // end hover on non-hovered over areas if they were previously hovered
//         else if hoverable.is_hovered {
//             // hover ended
//             hoverable.is_hovered = false;
//             end_event.send(EndEvent { entity });
//         }
//     }

//     // handle overlapping hovered over areas
//     hovered_areas.sort_by(|(_, a, _), (_, b, _)| b.translation().z.total_cmp(&a.translation().z));
//     let mut hovered_areas = hovered_areas.into_iter();
//     // top most area is considered hovered
//     if let Some((entity, _, mut hoverable)) = hovered_areas.next() {
//         if !hoverable.is_hovered {
//             hoverable.is_hovered = true;
//             start_event.send(StartEvent { entity });
//         }
//     }
//     // remaining areas are considered not hovered
//     for (entity, _, mut hoverable) in hovered_areas {
//         if hoverable.is_hovered {
//             hoverable.is_hovered = false;
//             end_event.send(EndEvent { entity });
//         }
//     }
// }

// #[allow(dead_code)]
// #[cfg(debug_assertions)]
// pub fn debug_log_events(mut start: EventReader<StartEvent>, mut end: EventReader<EndEvent>) {
//     for evt in end.iter() {
//         dbg!(evt);
//     }
//     for evt in start.iter() {
//         dbg!(evt);
//     }
// }

// #[cfg(debug_assertions)]
// fn require_transform_with_area(query: Query<&Area, Without<GlobalTransform>>) {
//     assert!(
//         query.is_empty(),
//         "hover::Area should not be added without a GlobalTransform"
//     );
// }
