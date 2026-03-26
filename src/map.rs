use macroquad::{
    math::{Rect, Vec2},
    prelude::rand,
};
use quad_lib::camera::Camera;

use crate::{
    insect::{Id, ant::ScentGrid},
    pos::{Pos, dirs},
};

pub type Faction = u8;

pub const FACTION_NONE: u8 = 0;

// pub const FACTION_PLAYER: u8
pub const FACTION_PILLBUG: u8 = u8::MAX - 1;
pub const FACTION_SPIDER: u8 = u8::MAX - 2;
pub const FACTION_CENTIPEDE: u8 = u8::MAX - 3;

pub const MAP_SIZE: i16 = 256;

pub const FOOD_DROP_ODDS: u32 = 10;
pub const FOOD_DROP_MIN: u32 = 20;
pub const FOOD_DROP_MAX: u32 = 70;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CellType {
    #[default]
    Empty,
    Food,
    Nest(Faction),
    Rock,
    Wall,
}

#[derive(Debug, Default, Clone)]
pub struct Cell {
    pub m_type: CellType,
    occupied: Option<(Faction, Id)>,
}

impl Cell {
    pub fn is_type(&self, flag: CellType) -> bool {
        self.m_type == flag
    }
    pub fn set_type(&mut self, flag: CellType) {
        if self.m_type == CellType::Empty {
            self.m_type = flag
        }
    }
    // pub fn clear_type(&mut self, _flag: CellType) {
    //     self.m_type = CellType::Empty
    // }

    pub fn take_type(&mut self, flag: CellType) -> Option<()> {
        if self.is_type(flag) {
            self.m_type = CellType::Empty;
            Some(())
        } else {
            None
        }
    }

    // pub fn _is_occupied(&self) -> bool {
    //     self.occupied.strong_count() > 0
    // }

    pub fn occupied_faction(&self) -> Faction {
        self.occupied
            .map(|(faction, _id)| faction)
            .unwrap_or(FACTION_NONE)
    }

    pub fn occupied_id(&self) -> Id {
        self.occupied
            .map(|(_faction, id)| id)
            .unwrap_or(0)
    }

    pub fn try_occupy(&mut self, faction: Faction, id: Id) -> Result<(), OccupyError> {
        if self.m_type == CellType::Rock {
            return Err(OccupyError::Solid);
        }
        if let Some((my_faction, my_id)) = self.occupied {
            if my_faction == faction {
                return Ok(());
            } else {
                return Err(OccupyError::Fight(my_id));
            }
        }
        self.occupied = Some((faction, id));
        Ok(())
    }

    pub fn free(&mut self, faction: Faction, id: Id) -> Result<(), OccupyError> {
        if let Some((my_faction, my_id)) = self.occupied {
            if my_faction == faction && my_id == id {
                self.occupied = None;
            }
        }
        Ok(())
    }
}

pub struct Map {
    pub occupied: Vec<Vec<Cell>>,
    pub size: Pos,
    pub camera: Camera,

    pub scent_grids: Vec<ScentGrid>,
}

pub struct Sight {
    pub cell_type: CellType,
    pub faction: Faction,
}

impl Sight {
    pub fn new(cell_type: CellType, faction: Faction) -> Self {
        Self { cell_type, faction }
    }
}

pub enum OccupyError {
    Solid,
    Fight(Id),
}

impl Map {
    pub fn new() -> Self {
        let size = Pos::new(MAP_SIZE, MAP_SIZE);
        let mut map: Map = Self {
            occupied: vec![vec![Cell::default(); MAP_SIZE as usize]; MAP_SIZE as usize],
            size,
            camera: Camera::new(),
            // TODO: SPARSE??
            scent_grids: vec![ScentGrid::new(size), ScentGrid::new(size),  ScentGrid::new(size)],
        };
        map.camera.zoom = 0.5;
        for _ in 0..10 {
            map.drop_rand_bunch(CellType::Rock);
        }
        for _ in 0..10 {
            map.drop_rand_bunch(CellType::Food);
        }
        map
    }

    pub fn rand_pos(&self) -> Pos {
        Pos::rand(self.size)
    }

    pub fn drop_rand_bunch(&mut self, t: CellType) {
        let mut pos = self.rand_pos();
        for _ in 0..rand::gen_range(FOOD_DROP_MIN, FOOD_DROP_MAX) {
            let Some(cell) = self.get_cell_mut(pos) else {
                continue;
            };
            cell.set_type(t);

            let new_pos = pos + dirs::rand();
            if self.is_valid(new_pos) {
                pos = new_pos;
            }
        }
    }

    pub const TILE_SIZE_DEFAULT: f32 = 16.;

    pub fn screen_pos(&self, pos: Pos) -> Vec2 {
        let world_pos: Vec2 = Vec2 {
            x: pos.x as f32 * Self::TILE_SIZE_DEFAULT,
            y: pos.y as f32 * Self::TILE_SIZE_DEFAULT,
        };
        self.camera.to_screen(world_pos)
    }

    pub fn screen_rect(&self, pos: Pos) -> Rect {
        let pos = self.screen_pos(pos);
        Rect {
            x: pos.x,
            y: pos.y,
            w: Self::TILE_SIZE_DEFAULT * self.camera.zoom,
            h: Self::TILE_SIZE_DEFAULT * self.camera.zoom,
        }
    }

    pub fn update_size(&mut self, player_pos: Pos) {
        // self.camera.zoom = 0.5;
        self.camera.keep_centered(self.screen_pos(player_pos));
    }

    pub fn update(&mut self) {
        if rand::gen_range(0, FOOD_DROP_ODDS) == 0 {
            self.drop_rand_bunch(crate::map::CellType::Food);
        }

        // OPTIMIZE: only check this every few ticks
        for scent_grid in &mut self.scent_grids {
            for cell in scent_grid.grid.iter_mut() {
                cell.update();
            }
        }
    }

    pub fn is_valid(&self, pos: Pos) -> bool {
        pos.x >= 0 && pos.x < self.size.x && pos.y >= 0 && pos.y < self.size.y
    }

    pub fn get_cell_mut(&mut self, pos: Pos) -> Option<&mut Cell> {
        if self.is_valid(pos) {
            Some(&mut self.occupied[pos.y as usize][pos.x as usize])
        } else {
            None
        }
    }

    pub fn get_cell(&self, pos: Pos) -> Option<&Cell> {
        if self.is_valid(pos) {
            Some(&self.occupied[pos.y as usize][pos.x as usize])
        } else {
            None
        }
    }

    pub(crate) fn occupy(
        &mut self,
        next_pos: Pos,
        (faction, id): (Faction, Id),
    ) -> Result<(), OccupyError> {
        self.get_cell_mut(next_pos)
            .ok_or(OccupyError::Solid)?
            .try_occupy(faction, id)
    }

    pub(crate) fn free(
        &mut self,
        next_pos: Pos,
        (faction, id): (Faction, Id),
    ) -> Result<(), OccupyError> {
        self.get_cell_mut(next_pos)
            .ok_or(OccupyError::Solid)?
            .free(faction, id)
    }

    pub fn sight(&self, pos: Pos) -> Sight {
        self.get_cell(pos)
            .map(|cell| Sight::new(cell.m_type, cell.occupied_faction()))
            .unwrap_or(Sight::new(CellType::Wall, FACTION_NONE))
    }

    pub fn get_scent_grid(&self, faction: Faction) -> &ScentGrid {
        self.scent_grids.get(faction as usize).unwrap()
    }

    pub fn get_scent_grid_mut(&mut self, faction: Faction) -> &mut ScentGrid {
        self.scent_grids.get_mut(faction as usize).unwrap()
    }

    // pub fn occupy(&mut self, pos: Point) -> bool {
    //     if pos.0 < 0 || pos.1 < 0 || pos.0 >= SQUARES || pos.1 >= SQUARES {
    //         return true;
    //     }

    //     if self.occupied[pos.1 as usize][pos.0 as usize] {
    //         return true;
    //     }
    //     self.occupied[pos.1 as usize][pos.0 as usize] = true;
    //     return false;
    // }
}
