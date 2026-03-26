
use macroquad::prelude::rand;

use crate::draw::{FOOD_COLOR, draw_cell_small};
use crate::insect::{Action, BaseInsect, Event, Hunger, Id, Insect, InsectBehaviour, InsectInfo, Interact, Perception};
use crate::map::{CellType, FACTION_SPIDER, Faction, Map};

use crate::pos::Pos;
use crate::pos::dirs::invert;

// const

#[derive(Debug, Clone, Copy)]
pub enum Scents {
    Food,
    Nest,
    Attack,
    Len,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ScentCell {
    scents: [u8; Scents::Len as usize],
}

impl ScentCell {
    pub fn get_scent(&self, scent: Scents) -> u8 {
        self.scents[scent as usize]
    }
    pub fn drop_scent(&mut self, scent: Scents, val: u8) -> u8 {
        let val = self.scents[scent as usize].max(val);
        self.scents[scent as usize] = val;
        val.saturating_sub(1)
    }

    pub fn update(&mut self) {
        // self.nest_scent = self.nest_scent.saturating_sub(1);
        self.scents[Scents::Food as usize] = self.scents[Scents::Food as usize].saturating_sub(1);
        self.scents[Scents::Attack as usize] =
            self.scents[Scents::Attack as usize].saturating_sub(1);
    }
}

pub struct ScentGrid {
    size: Pos,
    pub grid: Vec<ScentCell>,
}

impl ScentGrid {
    pub fn new(size: Pos) -> Self {
        Self {
            grid: vec![ScentCell::default(); size.x as usize * size.y as usize],
            size,
        }
    }

    pub fn get(&self, pos: Pos) -> Option<&ScentCell> {
        if pos.x < 0 || pos.y < 0 || pos.x >= self.size.x {
            None
        } else {
            self.grid
                .get(pos.x as usize + (pos.y as usize * self.size.x as usize))
        }
    }
    pub fn get_mut(&mut self, pos: Pos) -> Option<&mut ScentCell> {
        if pos.x < 0 || pos.y < 0 || pos.x >= self.size.x {
            None
        } else {
            self.grid
                .get_mut(pos.x as usize + pos.y as usize * self.size.x as usize)
        }
    }
}
// pub type ScentGrid = HashMap<Pos, ScentCell>;

// #[derive(Debug)]
pub struct AntQueen {
    // pub faction: Faction,
    // pub food: Food,
    // pub workers: Vec<Ant>,
    // pub soldiers: Vec<Ant>,
    // pub nest_pos: Pos,
    // the plan is to have the map much bigger, so most of it will be scent-less...
    // pub scents: ScentGrid,
}

// const STARTING_FOOD: Hunger = 128;
const ANT_FOOD_HUNGER: Hunger = 255;
const ANT_QUEEN_INFO: InsectInfo = InsectInfo {
    max_health: 10,
    max_speed: 3,
    damage: 1,
};

impl AntQueen {
    pub fn new(pos: Pos, faction: Faction) -> Insect {
        // seems jank, but leave for now
        // map.get_cell_mut(pos)
        //     .unwrap()
        //     .set_type(crate::map::CellType::Nest(faction));

        Insect {
            base: BaseInsect::new(pos, faction, &ANT_QUEEN_INFO),
            spec: Box::new(Self {
                // faction,
                // food: STARTING_FOOD,
                // workers: vec![Ant::new(pos, faction); STARTING_ANTS],
                // soldiers: vec![Ant::new(pos, faction); STARTING_ANTS / 4],
                // soldiers: Vec::new(),
                // scents: ScentGrid::new(map.size),
                // nest_pos: pos,
            }),
        }
    }
}

impl InsectBehaviour for AntQueen {
    fn update(&mut self, base: &mut BaseInsect, _map: &mut Map) -> Option<super::Event> {
        if base.hunger > ANT_FOOD_HUNGER * 2 {
            // create new ants!
            // if rand::rand() % 4 == 0 {
            //     self.soldiers.push(Ant::new(self.nest_pos, self.faction));
            // }
            base.hunger -= ANT_FOOD_HUNGER;
            return Some(super::Event::Birth(Box::new(Ant::new(
                base.pos,
                base.faction,
                base.id,
            ))));
        }
        None
    }

    fn player_action(
        &mut self,
        _base: &mut BaseInsect,
        _map: &mut Map,
        _action: super::Action,
    ) -> Option<super::Event> {
        todo!()
    }
    
    fn draw(&self, base: &BaseInsect, map: &Map) {
        base.draw(map);
    }
}

// #[derive(Debug, Clone)]
pub struct Ant {
    pub food: Option<()>,
    nest_scent: u8,
    pub food_scent: u8,
    pub attack_scent: u8,
    pub timeout: u8,
    pub queen: Id,
}

pub const ACTIVITY_TIMEOUT: u8 = u8::MAX;

const ANT_INFO: InsectInfo = InsectInfo {
    max_health: 2,
    max_speed: 0,
    damage: 2,
};

impl Ant {
    pub fn new(pos: Pos, faction: Faction, queen: Id) -> Insect {
        Insect {
            base: BaseInsect::new(pos, faction, &ANT_INFO),
            spec: Box::new(Self {
                food: None,
                food_scent: 0,
                nest_scent: u8::MAX,
                attack_scent: 0,
                timeout: ACTIVITY_TIMEOUT,
                queen,
            }),
        }
    }

    // smell pheremones only
    fn smell(scents: &ScentGrid, pos: Pos, scent: Scents) -> u8 {
        scents
            .get(pos)
            .map(|cell| cell.get_scent(scent))
            .unwrap_or(0)
    }

    // fn perceive(&self, map: &Map, scents: &ScentGrid, seeking: (Scents, CellType)) -> Perception {
    // base
    // .perception(map, seeking.1, |pos| Self::smell(scents, pos, seeking.0))
    // }

    pub fn update_scents(&mut self, base: &BaseInsect, map: &mut Map) {

        let scents = map.get_scent_grid_mut(base.faction);
        let scent_cell = scents.get_mut(base.pos).unwrap();

        if self.food.is_some() {
            let dist_approx = u8::MAX - self.nest_scent;
            self.food_scent = dist_approx
                .saturating_add(dist_approx)
                .saturating_add(dist_approx / 5)
                .saturating_add(48);
        } else {
            self.food_scent = 0;
        }
        let _ = scent_cell
            .drop_scent(Scents::Food, self.food_scent)
            .saturating_sub(1);
        // self.food_scent = self.food_scent.saturating_sub(2);

        // mark nest scent
        self.nest_scent = scent_cell.drop_scent(Scents::Nest, self.nest_scent);
        // self.nest_scent = self.nest_scent.saturating_sub(1);

        if self.attack_scent > 0 {
            self.attack_scent = scent_cell.drop_scent(Scents::Attack, self.attack_scent);
        }
    }

    pub fn update_behaviour(
        &mut self,
        base: &mut BaseInsect,
        map: &mut Map
    ) -> Option<Event> {
        // take food
        let cell = map.get_cell_mut(base.pos).expect("Ant in invalid pos");

        if cell.occupied_id() == self.queen {
            self.timeout = ACTIVITY_TIMEOUT;
            if self.food.is_some() {
                self.food = None;
                self.nest_scent = u8::MAX - 1;
                self.food_scent = 0;
                base.dir = invert(base.dir);

                // TODO: THIS AMONT IS DIFF
                return Some(Event::Interact(Interact::feed(base.id, self.queen, i8::MAX as u8)))
            }
        }
        // find food!
        else if self.food.is_none() {
            if let Some(food) = cell.take_type(CellType::Food) {
                if base.hunger < ANT_FOOD_HUNGER as u16 {
                    // eat it for ourselves
                    base.hunger += ANT_FOOD_HUNGER as u16;
                } else {
                    // take it back to the nest I guess
                    self.food = Some(food);
                    base.dir = invert(base.dir);
                }
            }
        } else if base.hunger < ANT_FOOD_HUNGER as u16 {
            // eat it for ourselves
            base.hunger += ANT_FOOD_HUNGER as u16;
            self.food = None;
        }

        None
    }

    // pub fn update_behaviour_soldier(
    //     &mut self,
    //     base: &BaseInsect,
    //     map: &mut Map,
    //     faction: Faction,
    // ) -> (Scents, CellType) {
    //     // take food
    //     let cell = map.get_cell_mut(base.pos).expect("Ant in invalid pos");

    //     self.timeout = self.timeout.saturating_sub(1);
    //     if self.timeout == 0 {
    //         (Scents::Nest, CellType::Nest(faction)) // hmmmmm
    //     } else {
    //         (Scents::Attack, CellType::Nest(u8::MAX)) // hmmmmm
    //     }

    //     if cell.occupied_id() == self.queen {
    //         self.timeout = ACTIVITY_TIMEOUT;
    //     }

    // }

    pub fn update_movement(
        &mut self,
        base: &BaseInsect,
        perception: &mut Perception,
        scents: &ScentGrid,
        // seeking: (Scents, CellType),
    ) -> Option<Pos> {
        self.timeout = self.timeout.saturating_sub(1);

        let seeking = if self.food.is_some() || self.timeout == 0 {
            (Scents::Nest, CellType::Nest(base.faction))
        } else {
            (Scents::Food, CellType::Food)
        };

        for percep in perception.iter() {
            if percep.1.cell_type == seeking.1 {
                // found it, no reason not to go there
                return Some(percep.0);
            }
            // only attack spiders for now
            if percep.1.faction == FACTION_SPIDER {
                // worker run away
                // if seeking.1 != CellType::Nest(u8::MAX) {
                //     let dist_approx = u8::MAX - self.nest_scent;
                //     self.attack_scent = dist_approx
                //         .saturating_add(dist_approx)
                //         .saturating_add(dist_approx / 5)
                //         .saturating_add(48);
                // return Some(base.pos + invert(percep.0 - base.pos));
                // }
                // else {
                // // soldier attack!
                return Some(percep.0);
                // return
                // }
            }
        }

        // chance to just go randomly
        let val = rand::rand();
        if val < u32::MAX / 4 {
            return Some(base.move_random());
        }

        // didn't see anything interesting, time to go by scents
        // I wish we could do a weighted thing, but it just doesn't seem to work right...
        let mut total: u16 = 0;
        for percep in perception.iter_mut() {
            // re-using cus lazy? or should we only smell directly ahead maybe?
            let scent = Self::smell(scents, percep.0, seeking.0);
            percep.1.faction = scent;
            total += scent as u16;
        }

        // no scents, just go randomly
        if total == 0 {
            return Some(base.move_random());
        }

        // pick a dir based on random chance, but weighted towards the highest scent dir
        // let mut pick = rand::gen_range(0, total);
        let mut pos: Pos = base.pos;
        let mut max: u8 = 0;
        for val in perception.iter() {
            if max <= val.1.faction {
                pos = val.0;
                max = val.1.faction;
            }
        }
        return Some(pos);
    }

    // fn update_soldier(&mut self, base: &mut BaseInsect, map: &mut Map) -> Option<super::Event> {
        // TEMP
        // if base.hunger < u8::MAX / 8 && *food > 0 {
        //     *food = *food - 1;
        //     base.hunger = u8::MAX;
        // }

        // self.update_behaviour_soldier(base, map)?;

        // let scents = map.get_scent_grid_mut(base.faction);
        // self.update_scents(scents);

        // let mut perception = base.perception(map);
        // let scents = map.get_scent_grid(base.faction);
        // let next_pos = self.update_movement(&base, &mut perception, scents, seeking);

        // if let Some(pos) = next_pos {
        //     base.try_move(pos, map)
        // } else {
        //     None
        // }
    // }

    // pub fn has_food(&self) -> bool {
    //     self.food.is_some()
    // }
}

impl InsectBehaviour for Ant {
    fn update(&mut self, base: &mut BaseInsect, map: &mut Map) -> Option<super::Event> {
        // TEMP
        // if base.hunger < u8::MAX / 8 && *food > 0 {
        //     *food = *food - 1;
        //     base.hunger = u8::MAX;
        // }

        if let Some(action) =  self.update_behaviour(base, map) { 
            return Some(action);
        }

        self.update_scents(base, map);

        let mut perception = base.perception(map);
        // get immutabe so borrow checker is happy...
        let scents = map.get_scent_grid(base.faction);
        let next_pos = self.update_movement(base, &mut perception, scents);

        if let Some(pos) = next_pos {
            base.try_move(pos, map)
        } else {
            None
        }
    }

    fn player_action(
        &mut self,
        _base: &mut BaseInsect,
        _map: &mut Map,
        _action: Action,
    ) -> Option<super::Event> {
        todo!()
    }
    
    fn draw(&self, base: &BaseInsect, map: &Map) {
        base.draw(map);
        if self.food.is_some() {
            draw_cell_small(map, base.pos, FOOD_COLOR);
        }
    }
}
