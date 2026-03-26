use std::collections::VecDeque;

use crate::{
    draw::{color, draw_cell_medium},
    insect::{Action, BaseInsect, Event, Hunger, Insect, InsectBehaviour, InsectInfo},
    map::{FACTION_CENTIPEDE, Map},
    pos::Pos,
};

pub struct Centipede {
    body: VecDeque<Pos>,
}

const CENTIPEDE_REPRODUCE_COST: Hunger = 512;
const CENTIPEDE_EAT_VAL: Hunger = 256;
const CENTIPEDE_MAX_LENGTH: usize = 8;

const CENTEPEDE_INFO: InsectInfo = InsectInfo {
    max_health: 3,
    max_speed: 1,
    damage: 3,
};

impl Centipede {
    pub fn new(pos: Pos) -> Insect {
        super::Insect {
            base: super::BaseInsect::new(pos, FACTION_CENTIPEDE, &CENTEPEDE_INFO),
            spec: Box::new(Self {
                body: vec![pos, pos, pos].into(),
            }),
        }
    }

    fn update_length(&mut self, base: &mut BaseInsect, map: &mut Map) {
        if self.body.len() < base.health as usize {
            self.body.push_back(*self.body.back().unwrap());
        } else if self.body.len() > base.health as usize {
            let pos = self.body.pop_back().unwrap();
            let _ = map.free(pos, (base.faction, base.id));
        }

        if self.body.front().unwrap() != &base.pos {
            self.body.pop_back();
            self.body.push_front(base.pos);
        }
    }

    fn grow(&mut self, base: &mut BaseInsect) -> Option<Event> {
        if base.health < CENTIPEDE_MAX_LENGTH as u8 {
            base.health += 1;
        } else {
            if base.update_reproduce(CENTIPEDE_REPRODUCE_COST) {
                return Some(Event::Birth(Box::new(Self::new(
                    *self.body.back().unwrap(),
                ))));
            }
        }
        None
    }
}

impl InsectBehaviour for Centipede {
    fn update(
        &mut self,
        base: &mut super::BaseInsect,
        map: &mut crate::map::Map,
    ) -> Option<super::Event> {
        self.update_length(base, map);
        // update food
        if base.try_eat(map) {
            if base.hunger < CENTIPEDE_REPRODUCE_COST {
                base.hunger += CENTIPEDE_EAT_VAL;
                return None;
            } else {
                return self.grow(base);
            }
        }

        match base.seek_omnivore(map) {
            super::Seek::Food(pos) => base.try_move(pos, map),
            super::Seek::Enemy(pos) => base.try_move(pos, map),
            super::Seek::Nothing => base.try_move(base.move_random(), map),
        }
    }

    fn player_action(
        &mut self,
        base: &mut super::BaseInsect,
        map: &mut crate::map::Map,
        action: super::Action,
    ) -> Option<super::Event> {
        self.update_length(base, map);

        match action {
            Action::ActionA => {
                if base.hunger > CENTIPEDE_REPRODUCE_COST {
                    base.hunger -= CENTIPEDE_EAT_VAL;
                    self.grow(base)
                } else {
                    None
                }
            }

            _ => None,
        }
    }

    fn draw(&self, base: &BaseInsect, map: &Map) {
        for pos in &self.body {
            draw_cell_medium(map, *pos, color(base.faction));
        }
    }
}
