// Fixed-point physics using i64 for deterministic calculations
// Using 32-bit fixed point: upper 32 bits = integer, lower 32 bits = fractional

use std::ops::{Add, Sub, Mul, Div};

const FIXED_POINT_SHIFT: u32 = 32;
const FIXED_POINT_SCALE: i64 = 1i64 << FIXED_POINT_SHIFT;
const FIXED_POINT_MASK: i64 = FIXED_POINT_SCALE - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fixed(pub i64);

impl Fixed {
    pub fn from_i32(n: i32) -> Self {
        Fixed((n as i64) << FIXED_POINT_SHIFT)
    }

    pub fn from_f64(f: f64) -> Self {
        Fixed((f * FIXED_POINT_SCALE as f64) as i64)
    }

    pub fn to_f64(self) -> f64 {
        self.0 as f64 / FIXED_POINT_SCALE as f64
    }

    pub fn to_i32(self) -> i32 {
        (self.0 >> FIXED_POINT_SHIFT) as i32
    }

    pub fn abs(self) -> Self {
        Fixed(self.0.abs())
    }

    pub fn sqrt(self) -> Self {
        Fixed(((self.0 as f64).sqrt() * FIXED_POINT_SCALE as f64) as i64)
    }

    pub fn mul_i32(self, n: i32) -> Self {
        Fixed(self.0 * n as i64)
    }
}

impl Add for Fixed {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Fixed(self.0 + other.0)
    }
}

impl Sub for Fixed {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Fixed(self.0 - other.0)
    }
}

impl Mul for Fixed {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Fixed((self.0 * other.0) >> FIXED_POINT_SHIFT)
    }
}

impl Div for Fixed {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        Fixed((self.0 << FIXED_POINT_SHIFT) / other.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Vec2 {
    pub x: Fixed,
    pub y: Fixed,
}

impl Vec2 {
    pub fn new(x: Fixed, y: Fixed) -> Self {
        Vec2 { x, y }
    }

    pub fn from_f64(x: f64, y: f64) -> Self {
        Vec2 {
            x: Fixed::from_f64(x),
            y: Fixed::from_f64(y),
        }
    }

    pub fn zero() -> Self {
        Vec2 {
            x: Fixed::from_i32(0),
            y: Fixed::from_i32(0),
        }
    }

    pub fn magnitude_squared(self) -> Fixed {
        self.x * self.x + self.y * self.y
    }

    pub fn magnitude(self) -> Fixed {
        self.magnitude_squared().sqrt()
    }

    pub fn normalize(self) -> Self {
        let mag = self.magnitude();
        if mag.0 == 0 {
            Vec2::zero()
        } else {
            Vec2 {
                x: self.x / mag,
                y: self.y / mag,
            }
        }
    }

    pub fn dot(self, other: Vec2) -> Fixed {
        self.x * other.x + self.y * other.y
    }

    pub fn distance_to(self, other: Vec2) -> Fixed {
        (other - self).magnitude()
    }

    pub fn to_f64_tuple(self) -> (f64, f64) {
        (self.x.to_f64(), self.y.to_f64())
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Vec2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Vec2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Mul<Fixed> for Vec2 {
    type Output = Self;
    fn mul(self, scale: Fixed) -> Self {
        Vec2 {
            x: self.x * scale,
            y: self.y * scale,
        }
    }
}

impl Mul<Vec2> for Fixed {
    type Output = Vec2;
    fn mul(self, v: Vec2) -> Vec2 {
        Vec2 {
            x: self * v.x,
            y: self * v.y,
        }
    }
}
