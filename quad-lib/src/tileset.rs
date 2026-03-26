use std::collections::HashMap;

use macroquad::{
    color::Color,
    math::{Rect, Vec2, vec2},
    texture::{DrawTextureParams, FilterMode, Texture2D, draw_texture_ex, load_texture},
};
use nanoserde::{DeRon, DeRonState};
// use serde::{Deserialize, Serialize};

pub const TILE_SIZE: Vec2 = Vec2::new(16., 16.);
pub const ROW_LENGTH: usize = 8;

#[derive(Clone, Copy)]
// #[derive(Serialize, Deserialize)]
pub struct Sprite {
    pub row: u8,
    pub col: u8,
}

impl Sprite {
    pub const fn new(row: u8, col: u8) -> Self {
        Sprite { row, col }
    }
}

#[derive(nanoserde::DeRon)]
pub struct DataTileset {
    path: String,
    sprites: Vec<Vec<String>>,
}

pub struct Tileset {
    pub texture: Texture2D,
    // NOTE: I am resisting the urge to optimize this with all my might, if it becomes a problem we could use pre-hashed strings or better hashmap implementation
    pub sprites: HashMap<String, Sprite>,
}

impl Tileset {
    pub async fn new(data_path: &str) -> Self {
        let data_tileset: DataTileset = {
            let input = &std::fs::read_to_string(data_path).unwrap();
            let mut state = DeRonState::default();
            let mut chars = input.chars();
            state.next(&mut chars);
            state.next_tok(&mut chars).unwrap();
            DeRon::de_ron(&mut state, &mut chars)
        }
        .unwrap();

        let texture = load_texture(&data_tileset.path).await.unwrap();
        texture.set_filter(FilterMode::Nearest);

        let mut sprites = HashMap::new();

        for (y, row) in data_tileset.sprites.iter().enumerate() {
            for (x, spr_str) in row.iter().enumerate() {
                if spr_str == &"" {
                    continue;
                }
                if let Some(_old) = sprites.insert((spr_str).into(), Sprite::new(y as u8, x as u8))
                {
                    log::warn!("Sprite {:?} overwritten!", spr_str);
                }
            }
        }

        Tileset { texture, sprites }
    }

    pub fn sprite_rect(&self, sprite: &Sprite) -> Rect {
        Rect {
            // Adding the 0.1 margin helps avoid slight gaps between tiles
            // I'm not totally sure why, it seems to be a floating point error?
            // See: https://github.com/not-fl3/macroquad/blob/master/tiled/src/lib.rs#L80
            x: (sprite.col as u32 * TILE_SIZE.x as u32) as f32 + 0.1,
            y: (sprite.row as u32 * TILE_SIZE.y as u32) as f32 + 0.1,
            w: (TILE_SIZE.x as u32) as f32 - 0.2,
            h: (TILE_SIZE.y as u32) as f32 - 0.2,
        }
    }

    pub fn draw_tile(&self, sprite: &str, dest: &Rect, color: Color) {
        if let Some(sprite) = self.sprites.get(sprite) {
            self.draw_sprite(sprite, dest, color);
        }
    }

    pub fn draw_tile_ex(&self, sprite: &str, color: Color, dest: &Rect, flip: bool) {
        if let Some(sprite) = self.sprites.get(sprite) {
            self.draw_sprite_ex(sprite, color, dest, flip);
        }
    }

    pub fn draw_sprite(&self, sprite: &Sprite, dest: &Rect, color: Color) {
        self.draw_sprite_ex(sprite, color, dest, false);
    }

    // pub fn draw_tile_flip(&self, sprite: Sprite, color: Color, dest: &Rect, rotation: f32) {
    //     self.draw_tile_ex(sprite, color, dest, rotation, true);
    // }

    pub fn draw_sprite_ex(&self, sprite: &Sprite, color: Color, dest: &Rect, flip: bool) {
        draw_texture_ex(&self.texture, dest.x, dest.y, color, DrawTextureParams {
            dest_size: Some(vec2(dest.w, dest.h)),
            source: Some(self.sprite_rect(sprite)),
            flip_x: flip,
            ..Default::default()
        });
    }
}
