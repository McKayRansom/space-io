use macroquad::{math::{vec2, Rect, Vec2}, window::{screen_height, screen_width}};

const DEFAULT_ZOOM: f32 = 0.1;
const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 4.;

pub struct Camera {
    pub zoom: f32,
    pub camera: Vec2,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            zoom: DEFAULT_ZOOM,
            camera: vec2(0., 0.),
        }
    }

    pub fn to_world(&self, screen_pos: Vec2) -> Vec2 {
        self.camera + (screen_pos / self.zoom)
    }

    pub fn to_screen(&self, world: Vec2) -> Vec2 {
        (world - self.camera) * self.zoom
    }

    pub fn to_screen_rect(&self, rect: Rect) -> Rect {
        Rect {
            x: (rect.x - self.camera.x) * self.zoom,
            y: (rect.y - self.camera.y) * self.zoom,
            w: rect.w * self.zoom,
            h: rect.h * self.zoom,
        }
    }

    pub fn keep_centered(&mut self, screen_pos: Vec2) {
        let screen_size: Vec2 = (screen_width(), screen_height()).into();

        // let's try 1/4 for now
        if screen_pos.x < screen_size.x / 4. {
            // adjust by the amount that it's off
            self.camera.x += screen_pos.x - screen_size.x / 4.;
        } else if screen_pos.x > screen_size.x * 3. / 4. {
            self.camera.x += screen_pos.x - screen_size.x * 3. / 4.;
        }

        if screen_pos.y < screen_size.y / 4. {
            // adjust by the amount that it's off
            self.camera.y += screen_pos.y - screen_size.y / 4.;
        } else if screen_pos.y > screen_size.y * 3. / 4. {
            self.camera.y += screen_pos.y - screen_size.y * 3. / 4.;
        }
    }

    #[allow(unused)]
    pub fn reset_camera(&mut self, size: (f32, f32)) {
        self.camera = vec2(
            -(screen_width() - size.0) / 2.,
            -(screen_height() - size.1) / 2.,
        );
        self.zoom = 1.;
        let zoom = (screen_height() / size.1).min(screen_width() / size.0);
        self.change_zoom(zoom - self.zoom);
    }

    pub fn change_zoom(&mut self, amount: f32) {
        let new_zoom = self.zoom + amount;

        if new_zoom <= MIN_ZOOM || new_zoom >= MAX_ZOOM {
            return;
        }

        let old_screen_zoom = 1. / self.zoom;
        let new_screen_zoom = 1. / new_zoom;
        self.camera.x += screen_width() * (old_screen_zoom - new_screen_zoom) / 2.;
        self.camera.y += screen_height() * (old_screen_zoom - new_screen_zoom) / 2.;

        self.zoom += amount;
        // println!("Zoom + {} = {}", amount, self.zoom);
        // let self.zoom = self.zoom.round();
    }
}
