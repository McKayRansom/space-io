use std::ops::{Add, Sub};

use macroquad::prelude::rand;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pos {
    pub x: i16,
    pub y: i16,
}

pub mod dirs {
    use macroquad::prelude::rand;

    use super::Pos;

    pub const UP_LEFT: Pos = Pos::new(-1, -1);
    pub const UP: Pos = Pos::new(0, -1);
    pub const UP_RIGHT: Pos = Pos::new(1, -1);
    pub const RIGHT: Pos = Pos::new(1, 0);
    pub const DOWN_RIGHT: Pos = Pos::new(1, 1);
    pub const DOWN: Pos = Pos::new(0, 1);
    pub const DOWN_LEFT: Pos = Pos::new(-1, 1);
    pub const LEFT: Pos = Pos::new(-1, 0);
    pub const NONE: Pos = Pos::new(0, 0);

    pub const ALL: &[Pos] = &[
        UP_LEFT, UP, UP_RIGHT, RIGHT, DOWN_RIGHT, DOWN, DOWN_LEFT, LEFT,
    ];
    pub const _ALL_REV: &[Pos] = &[RIGHT, LEFT, DOWN, UP];

    pub fn rand() -> Pos {
        ALL[rand::rand() as usize % ALL.len()]
    }

    pub fn rotate_right(pos: Pos) -> Pos {
        match pos {
            UP_LEFT => UP,
            UP => UP_RIGHT,
            UP_RIGHT => RIGHT,
            RIGHT => DOWN_RIGHT,
            DOWN_RIGHT => DOWN,
            DOWN => DOWN_LEFT,
            DOWN_LEFT => LEFT,
            LEFT => UP_LEFT,
            _ => panic!("Invalid dir: {:?}", pos),
        }
    }

    pub fn rotate_left(pos: Pos) -> Pos {
        match pos {
            UP_LEFT => LEFT,
            UP => UP_LEFT,
            UP_RIGHT => UP,
            RIGHT => UP_RIGHT,
            DOWN_RIGHT => RIGHT,
            DOWN => DOWN_RIGHT,
            DOWN_LEFT => DOWN,
            LEFT => DOWN_LEFT,
            _ => panic!("Invalid dir: {:?}", pos),
        }
    }

    pub fn invert(pos: Pos) -> Pos {
        match pos {
            UP_LEFT => DOWN_RIGHT,
            UP => DOWN,
            UP_RIGHT => DOWN_LEFT,
            RIGHT => LEFT,
            DOWN_RIGHT => UP_LEFT,
            DOWN => UP,
            DOWN_LEFT => UP_RIGHT,
            LEFT => RIGHT,
            _ => UP,
            // _ => panic!("Invalid dir: {:?}", pos),
        }
    }
}

impl Pos {
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }

    pub fn rand(max: Pos) -> Self {
        Self {
            x: rand::gen_range(0, max.x),
            y: rand::gen_range(0, max.y),
        }
    }
}

impl From<(i16, i16)> for Pos {
    fn from(value: (i16, i16)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}

impl Add<Pos> for Pos {
    type Output = Pos;

    fn add(self, rhs: Pos) -> Self::Output {
        Pos {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub<Pos> for Pos {
    type Output = Pos;

    fn sub(self, rhs: Pos) -> Self::Output {
        Pos {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
