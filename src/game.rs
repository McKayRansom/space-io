use macroquad::{
    color::colors,
    input::{is_key_down, KeyCode},
    math::Vec2 as MqVec2,
    shapes::draw_circle_lines,
    text::draw_text,
    window::{clear_background, screen_height, screen_width},
};

use crate::celestial::{CelestialBody, Orbit};
use crate::physics::{Fixed, Vec2};
use crate::spaceship::{Spaceship, ShipState};

const SCALE: f32 = 0.001; // Pixels per unit (fixed point)
const TIME_STEP: f32 = 0.016; // ~60 FPS

pub struct Game {
    spaceship: Spaceship,
    planet: CelestialBody,
    moon: CelestialBody,
    time: Fixed,
}

impl Game {
    pub fn new() -> Self {
        let earth_pos = Vec2::from_f64(400.0, 300.0);
        let moon_pos = Vec2::from_f64(600.0, 300.0);
        let moon_vel = Vec2::from_f64(0.0, 100.0); // Orbital velocity

        let mut ship = Spaceship::new(earth_pos + Vec2::from_f64(0.0, 100.0));
        ship.state = ShipState::Landed;

        Game {
            spaceship: ship,
            planet: CelestialBody::new(
                earth_pos,
                Vec2::zero(),
                Fixed::from_f64(50.0),
                Fixed::from_f64(10000.0),
                "Earth",
            ),
            moon: CelestialBody::new(
                moon_pos,
                moon_vel,
                Fixed::from_f64(20.0),
                Fixed::from_f64(1000.0),
                "Moon",
            ),
            time: Fixed::from_i32(0),
        }
    }

    pub fn update(&mut self) {
        let dt = Fixed::from_f64(TIME_STEP as f64);

        // Handle input
        self.handle_input(dt);

        // Update space bodies
        self.moon.update(dt, self.planet);

        // Calculate total gravity at ship position
        let gravity = self.planet.gravity_at(self.spaceship.position)
            + self.moon.gravity_at(self.spaceship.position);

        // Update ship
        self.spaceship.update(gravity, dt);

        // Check landing/collision
        self.check_collision_and_landing();

        // Update time
        self.time = self.time + dt;
    }

    fn handle_input(&mut self, dt: Fixed) {
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            self.spaceship.rotate_left(dt);
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            self.spaceship.rotate_right(dt);
        }
        if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
            if self.spaceship.state == ShipState::Flying {
                self.spaceship.apply_thrust(dt);
            }
        }
        if is_key_down(KeyCode::Space) {
            if self.spaceship.state == ShipState::Landed {
                // Takeoff
                self.spaceship.state = ShipState::Flying;
                self.spaceship.velocity = Vec2::from_f64(0.0, -50.0); // Initial liftoff velocity
                self.spaceship.fuel = self.spaceship.max_fuel;
            }
        }
    }

    fn check_collision_and_landing(&mut self) {
        let to_planet = self.spaceship.position - self.planet.position;
        let dist_to_planet = to_planet.magnitude();
        let planet_surface = self.planet.radius + Fixed::from_f64(20.0); // Landing zone

        if dist_to_planet <= planet_surface && self.spaceship.state == ShipState::Flying {
            // Check if moving slowly enough to land
            let speed = self.spaceship.velocity.magnitude();
            if speed < Fixed::from_f64(50.0) {
                self.spaceship.state = ShipState::Landed;
                self.spaceship.position = self.planet.position
                    + (to_planet.normalize() * self.planet.radius);
                self.spaceship.velocity = Vec2::zero();
                self.spaceship.angular_velocity = Fixed::from_i32(0);
                self.spaceship.refuel(10); // Slowly refuel while landed
            }
        }

        let to_moon = self.spaceship.position - self.moon.position;
        let dist_to_moon = to_moon.magnitude();
        let moon_surface = self.moon.radius + Fixed::from_f64(20.0);

        if dist_to_moon <= moon_surface && self.spaceship.state == ShipState::Flying {
            let speed = self.spaceship.velocity.magnitude();
            if speed < Fixed::from_f64(50.0) {
                self.spaceship.state = ShipState::Landed;
                self.spaceship.position =
                    self.moon.position + (to_moon.normalize() * self.moon.radius);
                self.spaceship.velocity = Vec2::zero();
                self.spaceship.angular_velocity = Fixed::from_i32(0);
            }
        }
    }

    pub fn draw(&self) {
        clear_background(colors::BLACK);

        // Convert world coordinates to screen coordinates
        let screen_center_x = screen_width() / 2.0;
        let screen_center_y = screen_height() / 2.0;

        // Draw planet
        let planet_pos = self.planet.position.to_f64_tuple();
        let planet_screen_x = screen_center_x + planet_pos.0 as f32 * SCALE;
        let planet_screen_y = screen_center_y + planet_pos.1 as f32 * SCALE;
        let planet_radius = self.planet.radius.to_f64() as f32 * SCALE;

        macroquad::shapes::draw_circle(planet_screen_x, planet_screen_y, planet_radius, colors::BLUE);
        draw_circle_lines(
            planet_screen_x,
            planet_screen_y,
            planet_radius,
            2.0,
            colors::DARKBLUE,
        );

        // Draw moon
        let moon_pos = self.moon.position.to_f64_tuple();
        let moon_screen_x = screen_center_x + moon_pos.0 as f32 * SCALE;
        let moon_screen_y = screen_center_y + moon_pos.1 as f32 * SCALE;
        let moon_radius = self.moon.radius.to_f64() as f32 * SCALE;

        macroquad::shapes::draw_circle(moon_screen_x, moon_screen_y, moon_radius, colors::GRAY);
        draw_circle_lines(
            moon_screen_x,
            moon_screen_y,
            moon_radius,
            2.0,
            colors::DARKGRAY,
        );

        // Draw ship orbit prediction
        if self.spaceship.state == ShipState::Flying {
            self.draw_orbit_prediction(
                screen_center_x,
                screen_center_y,
                self.planet.position,
            );
        }

        // Draw spaceship
        let ship_pos = self.spaceship.position.to_f64_tuple();
        let ship_screen_x = screen_center_x + ship_pos.0 as f32 * SCALE;
        let ship_screen_y = screen_center_y + ship_pos.1 as f32 * SCALE;
        let ship_size = 8.0;

        // Draw ship as rotated square
        self.draw_ship(
            ship_screen_x,
            ship_screen_y,
            self.spaceship.rotation.to_f64() as f32,
            ship_size,
        );

        // Draw HUD
        self.draw_hud(screen_center_x, screen_center_y);
    }

    fn draw_ship(&self, x: f32, y: f32, rotation: f32, size: f32) {
        let cos_r = rotation.cos();
        let sin_r = rotation.sin();

        let corners = [
            (-size, -size),
            (size, -size),
            (size, size),
            (-size, size),
        ];

        let rotated: Vec<_> = corners
            .iter()
            .map(|(cx, cy)| {
                let rx = cx * cos_r - cy * sin_r;
                let ry = cx * sin_r + cy * cos_r;
                (x + rx, y + ry)
            })
            .collect();

        // Draw filled square by drawing 4 triangles
        let p0 = rotated[0];
        let p1 = rotated[1];
        let p2 = rotated[2];
        let p3 = rotated[3];

        // Draw two triangles to form a square
        macroquad::shapes::draw_triangle(
            MqVec2::new(p0.0, p0.1),
            MqVec2::new(p1.0, p1.1),
            MqVec2::new(p2.0, p2.1),
            colors::WHITE,
        );
        macroquad::shapes::draw_triangle(
            MqVec2::new(p0.0, p0.1),
            MqVec2::new(p2.0, p2.1),
            MqVec2::new(p3.0, p3.1),
            colors::WHITE,
        );

        // Draw direction indicator (nose of the ship)
        let nose_x = x + cos_r * size * 1.5;
        let nose_y = y + sin_r * size * 1.5;
        macroquad::shapes::draw_line(x, y, nose_x, nose_y, 2.0, colors::RED);
    }

    fn draw_orbit_prediction(&self, screen_cx: f32, screen_cy: f32, central_pos: Vec2) {
        let orbit = match Orbit::from_state_vectors(
            self.spaceship.position,
            self.spaceship.velocity,
            central_pos,
            self.planet.mass,
        ) {
            Some(o) => o,
            None => return,
        };

        // Draw periapsis and apoapsis markers
        let periapsis_dist = orbit.periapsis.to_f64() as f32 * SCALE;
        macroquad::shapes::draw_circle(
            screen_cx,
            screen_cy + periapsis_dist,
            3.0,
            colors::GREEN,
        );

        let apoapsis_dist = orbit.apoapsis.to_f64() as f32 * SCALE;
        macroquad::shapes::draw_circle(
            screen_cx,
            screen_cy - apoapsis_dist,
            3.0,
            colors::RED,
        );

        // Draw predicted orbit path (simplified ellipse approximation)
        let segments = 32;
        for i in 0..segments {
            let t1 = i as f64 / segments as f64 * 2.0 * std::f64::consts::PI;
            let t2 = (i + 1) as f64 / segments as f64 * 2.0 * std::f64::consts::PI;

            let pos1 = self.orbit_point(central_pos, &orbit, t1);
            let pos2 = self.orbit_point(central_pos, &orbit, t2);

            let p1 = (
                screen_cx + pos1.0 as f32 * SCALE,
                screen_cy + pos1.1 as f32 * SCALE,
            );
            let p2 = (
                screen_cx + pos2.0 as f32 * SCALE,
                screen_cy + pos2.1 as f32 * SCALE,
            );

            macroquad::shapes::draw_line(p1.0, p1.1, p2.0, p2.1, 1.0, colors::YELLOW);
        }
    }

    fn orbit_point(&self, center: Vec2, orbit: &Orbit, nu: f64) -> (f64, f64) {
        let r = orbit.semi_major_axis.to_f64()
            * (1.0 - orbit.eccentricity.to_f64().powi(2))
            / (1.0 + orbit.eccentricity.to_f64() * nu.cos());
        let x = center.x.to_f64() + r * nu.cos();
        let y = center.y.to_f64() + r * nu.sin();
        (x, y)
    }

    fn draw_hud(&self, _screen_cx: f32, _screen_cy: f32) {
        let fuel_percent = (self.spaceship.fuel as f32 / self.spaceship.max_fuel as f32) * 100.0;
        draw_text(
            &format!(
                "FUEL: {:.0}% ({}/{}L)",
                fuel_percent, self.spaceship.fuel, self.spaceship.max_fuel
            ),
            10.0,
            20.0,
            20.0,
            colors::WHITE,
        );

        let vel = self.spaceship.velocity.magnitude().to_f64();
        draw_text(
            &format!("VEL: {:.1} m/s", vel),
            10.0,
            45.0,
            20.0,
            colors::WHITE,
        );

        let state_str = match self.spaceship.state {
            ShipState::Flying => "FLYING",
            ShipState::Landed => "LANDED",
        };
        draw_text(
            &format!("STATE: {}", state_str),
            10.0,
            70.0,
            20.0,
            colors::WHITE,
        );

        draw_text(
            &format!(
                "POS: ({:.0}, {:.0})",
                self.spaceship.position.x.to_f64(),
                self.spaceship.position.y.to_f64()
            ),
            10.0,
            95.0,
            20.0,
            colors::WHITE,
        );

        draw_text("CONTROLS: Arrow Keys or WASD to rotate, UP/W for thrust, SPACE to launch", 10.0, screen_height() - 10.0, 16.0, colors::WHITE);
    }
}
