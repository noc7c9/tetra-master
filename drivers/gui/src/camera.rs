// Wrapper around bevy's 2D camera to provide a camera that render's at a specific resolution and
// then resizes to fill the screen while keeping the aspect ratio

use bevy::prelude::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system(maintain_aspect_ratio);
    }
}

#[derive(Component)]
struct CameraRenderSize(Vec2);

#[derive(Bundle)]
pub struct Camera {
    camera_render_size: CameraRenderSize,
    camera_2d: Camera2dBundle,
}

impl Camera {
    pub fn new(render_size: Vec2) -> Self {
        Self {
            camera_render_size: CameraRenderSize(render_size),
            camera_2d: default(),
        }
    }
}

fn maintain_aspect_ratio(
    mut window_resized: EventReader<bevy::window::WindowResized>,
    mut camera: Query<(&mut OrthographicProjection, &CameraRenderSize)>,
) {
    if let Some(window_size) = window_resized.iter().last() {
        if let Ok((mut projection, CameraRenderSize(render_size))) = camera.get_single_mut() {
            projection.scale =
                (render_size.x / window_size.width).max(render_size.y / window_size.height);
        }
    }
}
