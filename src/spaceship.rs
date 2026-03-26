use crate::physics::{Fixed, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShipState {
    Flying,
    Landed,
}

pub struct Spaceship {
    pub position: Vec2,
    pub velocity: Vec2,
    pub rotation: Fixed, // in radians, stored as fixed point
    pub fuel: i32, // liters
    pub max_fuel: i32,
    pub state: ShipState,
    pub angular_velocity: Fixed,
}

impl Spaceship {
    pub fn new(pos: Vec2) -> Self {
        Spaceship {
            position: pos,
            velocity: Vec2::zero(),
            rotation: Fixed::from_i32(0),
            fuel: 1000,
            max_fuel: 1000,
            state: ShipState::Landed,
            angular_velocity: Fixed::from_i32(0),
        }
    }

    pub fn rotate_left(&mut self, dt: Fixed) {
        self.angular_velocity = self.angular_velocity - Fixed::from_f64(3.0) * dt;
    }

    pub fn rotate_right(&mut self, dt: Fixed) {
        self.angular_velocity = self.angular_velocity + Fixed::from_f64(3.0) * dt;
    }

    pub fn apply_thrust(&mut self, dt: Fixed) -> bool {
        if self.fuel <= 0 {
            return false;
        }

        let thrust_power = Fixed::from_f64(100.0);
        let cos_rot = self.rotation.to_f64().cos();
        let sin_rot = self.rotation.to_f64().sin();

        let accel = Vec2 {
            x: Fixed::from_f64(cos_rot) * thrust_power,
            y: Fixed::from_f64(sin_rot) * thrust_power,
        };

        self.velocity = self.velocity + accel * dt;

        // Consume fuel: ~1 liter per second at full thrust
        self.fuel = (self.fuel - 1).max(0);

        true
    }

    pub fn update(&mut self, gravity: Vec2, dt: Fixed) {
        // Apply gravity
        self.velocity = self.velocity + gravity * dt;

        // Update position
        self.position = self.position + self.velocity * dt;

        // Update rotation
        self.rotation = self.rotation + self.angular_velocity * dt;

        // Dampen angular velocity slightly
        self.angular_velocity = self.angular_velocity * Fixed::from_f64(0.95);
    }

    pub fn get_direction_vector(self) -> Vec2 {
        let rot = self.rotation.to_f64();
        Vec2::from_f64(rot.cos(), rot.sin())
    }

    pub fn refuel(&mut self, amount: i32) {
        self.fuel = (self.fuel + amount).min(self.max_fuel);
    }
}
