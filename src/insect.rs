use macroquad::prelude::rand;

use crate::{
    draw::{color, draw_cell_medium},
    map::{CellType, FACTION_NONE, Faction, Map, OccupyError, Sight},
    pos::{
        Pos,
        dirs::{self, invert, rotate_left, rotate_right},
    },
};

pub mod ant;
pub mod centipede;
pub mod egg;
pub mod pillbug;
pub mod player;
pub mod spider;

// #[derive(Debug, Clone, Copy)]
// pub struct Perception {
// raw: [(Pos, u8); 5],
// }

pub type Perception = [(Pos, Sight); 5];

pub type Hunger = u16;
pub type Health = u8;

pub struct Interact {
    pub src: Id,
    pub dst: Id,
    pub amt: i8, // for now: negative is attack, positive is feed
}

impl Interact {
    pub fn fight(src: Id, dst: Id, amt: u8) -> Self {
        Self {
            src,
            dst,
            amt: -(amt as i8),
        }
    }
    pub fn feed(src: Id, dst: Id, amt: u8) -> Self {
        Self {
            src,
            dst,
            amt: amt as i8,
        }
    }
}

pub enum Event {
    Birth(Box<Insect>),
    Rebirth(Box<Insect>),
    Death(),
    Interact(Interact),
}

#[derive(Clone, Copy)]
pub enum Action {
    Move(Pos),
    ActionA,
    ActionB,
}

pub trait InsectBehaviour {
    fn update(&mut self, base: &mut BaseInsect, map: &mut Map) -> Option<Event>;
    fn player_action(
        &mut self,
        base: &mut BaseInsect,
        map: &mut Map,
        action: Action,
    ) -> Option<Event>;
    fn draw(&self, base: &BaseInsect, map: &Map);
    // fn sprite(&mut self) -> Sprite // can change easily...
    // fn attacked(&mut self, strength: u8);
}

pub struct Insect {
    pub base: BaseInsect,
    pub spec: Box<dyn InsectBehaviour>,
}

impl Insect {
    pub fn die(&self, map: &mut Map) {
        let _ = map.free(self.base.pos, (self.base.faction, self.base.id));
        // what if we dropped some food?
        if let Some(cell) = map.get_cell_mut(self.base.pos) {
            if cell.is_type(crate::map::CellType::Empty) {
                cell.set_type(crate::map::CellType::Food);
            }
        }
    }

    pub fn update(&mut self, map: &mut Map) -> Option<Event> {
        if !self.base.will_move() {
            return None;
        }
        self.base
            .update()
            .or_else(|| self.spec.update(&mut self.base, map))
    }
    pub fn player_action(&mut self, map: &mut Map, action: Action) -> Option<Event> {
        self.spec.player_action(&mut self.base, map, action)
    }
    pub fn interact(&mut self, interact: Interact) {
        if interact.amt < 0 {
            // TODO: This is way too simple, have spec respond?
            self.base.health = self.base.health.saturating_sub((-interact.amt) as Health);
        } else {
            self.base.hunger = self.base.hunger.saturating_add(interact.amt as Hunger);
        }
    }
}

#[derive(Debug)]
pub struct InsectInfo {
    pub max_health: Health,
    pub max_speed: u8,
    pub damage: Health,
}

// pub const DE

pub type Id = u32;
// pub

/// Base class-ish for different insect types
#[derive(Debug, Clone)]
pub struct BaseInsect {
    pub pos: Pos,
    pub dir: Pos,
    pub hunger: Hunger,
    pub health: Health,
    pub faction: Faction,
    pub id: Id,
    pub speed: u8,
    pub info: &'static InsectInfo,
}

pub enum Seek {
    Food(Pos),
    Enemy(Pos),
    Nothing,
}

impl BaseInsect {
    pub fn new(pos: Pos, faction: Faction, info: &'static InsectInfo) -> Self {
        Self {
            pos,
            dir: dirs::rand(),
            hunger: rand::gen_range(u8::MAX as u16 / 2, u8::MAX as u16),
            health: info.max_health,
            faction,
            id: 0,
            speed: rand::gen_range(0, info.max_speed),
            info,
        }
    }

    pub fn sight(pos: Pos, map: &Map) -> (Pos, Sight) {
        (pos, map.sight(pos))
    }

    pub fn perception(&self, map: &Map) -> Perception {
        [
            Self::sight(self.pos + rotate_left(rotate_left(self.dir)), map),
            Self::sight(self.pos + rotate_left(self.dir), map),
            Self::sight(self.pos + self.dir, map),
            Self::sight(self.pos + rotate_right(self.dir), map),
            Self::sight(self.pos + rotate_right(rotate_right(self.dir)), map),
        ]
    }

    pub fn move_random(&self) -> Pos {
        // 50/50 chance to turn
        let val = rand::rand();
        if val < u32::MAX / 2 {
            // 50/50 chance of dir
            if val < u32::MAX / 4 {
                self.pos + dirs::rotate_left(self.dir)
            } else {
                self.pos + dirs::rotate_right(self.dir)
            }
        } else {
            self.pos + self.dir
        }
    }

    pub fn will_move(&mut self) -> bool {
        if self.speed == 0 {
            self.speed = self.info.max_speed;
            true
        } else {
            self.speed -= 1;
            false
        }
    }

    pub fn update_reproduce(&mut self, reproduce_cost: u16) -> bool {
        // TODO: save some hunger so we don't starve
        if self.hunger > reproduce_cost + 100 {
            self.hunger -= reproduce_cost;
            true
        } else {
            false
        }
    }

    pub fn update(&mut self) -> Option<Event> {
        if self.health == 0 {
            return Some(Event::Death());
        }
        // check if we die of hunger
        self.hunger = self.hunger.saturating_sub(1);
        if self.hunger == 0 {
            // we die
            return Some(Event::Death());
        }
        None
    }

    pub fn try_eat(&mut self, map: &mut Map) -> bool {
        map.get_cell_mut(self.pos)
            .unwrap()
            .take_type(CellType::Food)
            .is_some()
    }

    // Seek out enimies, food in that order
    pub fn seek_omnivore(&self, map: &Map) -> Seek {
        let perception = self.perception(map);
        let mut best_pos: Option<Pos> = None;
        for percep in &perception {
            if percep.1.faction != self.faction && percep.1.faction != FACTION_NONE {
                return Seek::Enemy(percep.0);
            }
            if percep.1.cell_type == CellType::Food {
                best_pos = Some(percep.0);
            }
        }

        match best_pos {
            Some(pos) => Seek::Food(pos),
            None => Seek::Nothing,
        }
    }

    pub fn try_move(&mut self, next_pos: Pos, map: &mut Map) -> Option<Event> {
        match map.occupy(next_pos, (self.faction, self.id)) {
            Ok(_) => {
                let _ = map.free(self.pos, (self.faction, self.id));
                self.dir = next_pos - self.pos;
                self.pos = next_pos;
                // self.occupy = occupy;
            }
            Err(OccupyError::Solid) => self.dir = invert(self.dir),
            Err(OccupyError::Fight(id)) => {
                // TODO: REAL FIGHTS
                // map.occupy() should have marked opponent as occupied, so they should die too
                // we die now
                return Some(Event::Interact(Interact::fight(
                    self.id,
                    id,
                    self.info.damage,
                )));
            }
        }

        None
    }

    pub fn draw(&self, map: &Map) {
        draw_cell_medium(map, self.pos, color(self.faction));
    }
}
