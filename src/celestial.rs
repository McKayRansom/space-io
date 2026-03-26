use crate::physics::{Fixed, Vec2};

#[derive(Debug, Clone, Copy)]
pub struct CelestialBody {
    pub position: Vec2,
    pub velocity: Vec2,
    pub radius: Fixed,
    pub mass: Fixed, // in relative units
    pub name: &'static str,
}

impl CelestialBody {
    pub fn new(pos: Vec2, vel: Vec2, radius: Fixed, mass: Fixed, name: &'static str) -> Self {
        CelestialBody {
            position: pos,
            velocity: vel,
            radius,
            mass,
            name,
        }
    }

    pub fn update(&mut self, dt: Fixed, central_mass: CelestialBody) {
        let to_center = central_mass.position - self.position;
        let distance = to_center.magnitude();

        if distance.0 > 0 {
            let g = Fixed::from_f64(0.0001); // Gravitational parameter
            let accel_magnitude = g * central_mass.mass / (distance * distance);
            let accel_direction = to_center.normalize();
            let acceleration = accel_direction * accel_magnitude;

            self.velocity = self.velocity + acceleration * dt;
            self.position = self.position + self.velocity * dt;
        }
    }

    pub fn gravity_at(&self, point: Vec2) -> Vec2 {
        let to_body = self.position - point;
        let distance = to_body.magnitude();

        if distance.0 <= self.radius.0 {
            // Inside body, no gravity
            Vec2::zero()
        } else {
            let g = Fixed::from_f64(0.00001); // Gravitational parameter (scaled)
            let accel_magnitude = g * self.mass / (distance * distance);
            let direction = to_body.normalize();
            direction * accel_magnitude
        }
    }

    pub fn is_surface_altitude(point: Vec2, altitude: Fixed) -> bool {
        true // Simplified: checking will be done in game logic
    }
}

pub struct Orbit {
    pub semi_major_axis: Fixed,
    pub eccentricity: Fixed,
    pub periapsis: Fixed,
    pub apoapsis: Fixed,
}

impl Orbit {
    /// Calculate orbital elements from position and velocity around a central body
    pub fn from_state_vectors(
        pos: Vec2,
        vel: Vec2,
        central_position: Vec2,
        central_mass: Fixed,
    ) -> Option<Self> {
        let r = pos - central_position;
        let r_mag = r.magnitude();
        let v_mag = vel.magnitude();

        if r_mag.0 == 0 || v_mag.0 == 0 {
            return None;
        }

        let g = Fixed::from_f64(0.00001);
        let mu = g * central_mass;

        // Specific orbital energy
        let energy = (v_mag * v_mag) / Fixed::from_f64(2.0) - mu / r_mag;

        // Semi-major axis
        let sma = if energy.0 != 0 {
            energy / Fixed::from_f64(-2.0) * Fixed::from_f64(-1.0)
        } else {
            r_mag // Parabolic orbit
        };

        if sma.0 <= 0 {
            return None; // Hyperbolic or invalid orbit
        }

        // Eccentricity
        let h = r.magnitude() * v_mag; // Specific angular momentum (approximation)
        let p = h * h / mu; // Semi-latus rectum
        let e_squared = Fixed::from_f64(1.0) + Fixed::from_f64(2.0) * energy * h * h / (mu * mu);
        
        if e_squared.0 < 0 {
            return None;
        }

        let ecc = e_squared.sqrt();

        if ecc.0 < 0 {
            return None;
        }

        let periapsis = sma * (Fixed::from_f64(1.0) - ecc);
        let apoapsis = sma * (Fixed::from_f64(1.0) + ecc);

        Some(Orbit {
            semi_major_axis: sma,
            eccentricity: ecc,
            periapsis,
            apoapsis,
        })
    }

    /// Get predicted position along orbit at time t (simplified)
    pub fn get_position_at_time(
        &self,
        current_pos: Vec2,
        central_pos: Vec2,
        t: f64,
    ) -> Vec2 {
        // Simplified: linear approximation for visualization
        // In a full implementation, would use Kepler's equation
        let direction = (current_pos - central_pos).normalize();
        let distance_traveled = self.semi_major_axis * Fixed::from_f64(0.1 * t);
        central_pos + direction * distance_traveled
    }
}
