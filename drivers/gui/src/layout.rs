// Utilities for positioning (and scaling) entities on screen

use crate::{ASSET_SCALE, RENDER_SIZE};
use bevy::prelude::*;

const RENDER_HALF_SIZE: Vec2 = vec2!(RENDER_SIZE / 2.);
const SCALE: Vec3 = Vec3::new(ASSET_SCALE, ASSET_SCALE, 1.0);

pub struct Z;

impl Z {
    pub const BG: f32 = 0.;

    pub const CARD_COUNTER: f32 = 1.;
    pub const TURN_INDICATOR_COIN: f32 = 1.;

    pub const CANDIDATE_HAND_CARD: f32 = 1.;

    pub const HAND_CARD: f32 = 1.;
    pub const HAND_CARD_ACTIVE: f32 = 5.;
    pub const HAND_CARD_HOVERED: f32 = 10.;

    pub const BOARD_CARD: f32 = 1.;
    pub const BOARD_BLOCKED_CELL: f32 = 1.;
    pub const BOARD_CARD_STATS: f32 = 2.;
    pub const BOARD_CARD_SELECT_INDICATOR: f32 = 2.;

    // hover areas
    // pub const CANDIDATE_HAND_HOVER_AREA: f32 = 100.;
    pub const BOARD_CELL_HOVER_AREA: f32 = 100.;

    #[cfg(debug_assertions)]
    pub const DEBUG: f32 = 666.;

    pub const UI_TEXT: f32 = 10.;
}

pub trait TransformExt: Sized {
    #[must_use]
    fn offset(self, offset: impl Into<Vec2>) -> Self;

    #[must_use]
    #[inline]
    fn offset_x(self, x: f32) -> Self {
        self.offset((x, 0.0))
    }

    #[must_use]
    #[inline]
    fn offset_y(self, y: f32) -> Self {
        self.offset((0.0, y))
    }

    #[must_use]
    fn offset_z(self, y: f32) -> Self;

    #[must_use]
    fn z(self, z: f32) -> Self;

    #[must_use]
    fn scale(self, scale: f32) -> Self;
}

impl TransformExt for Transform {
    #[inline]
    fn offset(mut self, offset: impl Into<Vec2>) -> Self {
        let offset = offset.into();
        self.translation.x += offset.x;
        self.translation.y += offset.y;
        self
    }

    #[inline]
    fn offset_z(mut self, offset_z: f32) -> Self {
        self.translation.z += offset_z;
        self
    }

    #[inline]
    fn z(mut self, z: f32) -> Self {
        self.translation.z = z;
        self
    }

    #[inline]
    fn scale(mut self, amount: f32) -> Self {
        self.scale = Vec3::ONE * amount;
        self
    }
}

pub fn absolute(translation: impl Into<Vec2>) -> Transform {
    Transform {
        translation: translation.into().extend(0.0),
        scale: SCALE,
        rotation: default(),
    }
}

#[allow(dead_code)]
pub fn center() -> Transform {
    absolute((0., 0.))
}

#[allow(dead_code)]
pub fn left() -> Transform {
    absolute((-RENDER_HALF_SIZE.x, 0.))
}

#[allow(dead_code)]
pub fn right() -> Transform {
    absolute((RENDER_HALF_SIZE.x, -0.))
}

#[allow(dead_code)]
pub fn top() -> Transform {
    absolute((0., RENDER_HALF_SIZE.y))
}

#[allow(dead_code)]
pub fn bottom() -> Transform {
    absolute((0., -RENDER_HALF_SIZE.y))
}

#[allow(dead_code)]
pub fn top_left() -> Transform {
    absolute((-RENDER_HALF_SIZE.x, RENDER_HALF_SIZE.y))
}

#[allow(dead_code)]
pub fn top_right() -> Transform {
    absolute((RENDER_HALF_SIZE.x, RENDER_HALF_SIZE.y))
}

#[allow(dead_code)]
pub fn bottom_left() -> Transform {
    absolute((-RENDER_HALF_SIZE.x, -RENDER_HALF_SIZE.y))
}

#[allow(dead_code)]
pub fn bottom_right() -> Transform {
    absolute((RENDER_HALF_SIZE.x, -RENDER_HALF_SIZE.y))
}

pub fn line_horizontal(transform: Transform) -> LineLayout {
    LineLayout::new(Direction::Horizontal, transform.translation)
}

pub fn line_vertical(transform: Transform) -> LineLayout {
    LineLayout::new(Direction::Vertical, transform.translation)
}

pub enum Direction {
    Horizontal,
    Vertical,
}

pub struct LineLayout {
    direction: Direction,
    translation: Vec3,
    padding: f32,

    entity_size: Option<Vec2>,
    num_entities: Option<usize>,
}

impl LineLayout {
    fn new(direction: Direction, translation: Vec3) -> Self {
        Self {
            direction,
            translation,
            padding: 0.0,

            entity_size: None,
            num_entities: None,
        }
    }

    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    pub fn entity_size(mut self, entity_size: Vec2) -> Self {
        self.entity_size = Some(entity_size);
        self
    }

    pub fn num_entities(mut self, num_entities: usize) -> Self {
        self.num_entities = Some(num_entities);
        self
    }

    pub fn index(&self, index: usize) -> Transform {
        // panic if all necessary fields have been not initialized
        let entity_size = self.entity_size.unwrap();
        let num_entities = self.num_entities.unwrap() as f32;

        // start with the center of the layout
        let mut x = self.translation.x;
        let mut y = self.translation.y;

        match self.direction {
            Direction::Horizontal => {
                // move to the bottom-left corner of the layout (ie. position of the first entity)
                x -= (num_entities / 2. - 0.5) * entity_size.x;
                x -= (num_entities / 2. - 0.5) * self.padding;

                // offset based on the index of the entity
                x += index as f32 * (entity_size.x + self.padding);
            }
            Direction::Vertical => {
                // move to the bottom-left corner of the layout (ie. position of the first entity)
                y -= (num_entities / 2. - 0.5) * entity_size.y;
                y -= (num_entities / 2. - 0.5) * self.padding;

                // offset based on the index of the entity
                y += index as f32 * (entity_size.y + self.padding);
            }
        }

        absolute((x, y)).z(self.translation.z)
    }
}
