use crate::{
    draw::{color, draw_cell_small},
    insect::{Action, BaseInsect, Event, Insect, InsectBehaviour, InsectInfo},
    map::{FACTION_PILLBUG, Map},
    pos::Pos,
};

use super::Hunger;

pub struct Pillbug {
    pub curled: bool,
}

const PILLBUG_REPRODUCE_COST: Hunger = u8::MAX as Hunger;
const PILLBUG_EAT_THRESHOLD: Hunger = u8::MAX as Hunger * 2;
const PILLBUG_FOOD_VALUE: Hunger = u8::MAX as Hunger;
const PILLBUG_SPEED: u8 = 1;
const PILLBUG_CURL_TIME: u8 = 10;

const PILLBUG_INFO: InsectInfo = InsectInfo {
    max_health: 2,
    max_speed: 2,
    damage: 1,
};

impl Pillbug {
    pub fn new(pos: Pos) -> Insect {
        Insect {
            base: BaseInsect::new(pos, FACTION_PILLBUG, &PILLBUG_INFO),
            spec: Box::new(Self { curled: false }),
        }
    }
}

impl InsectBehaviour for Pillbug {
    fn update(&mut self, base: &mut BaseInsect, map: &mut Map) -> Option<Event> {
        if self.curled {
            self.curled = false;
            let _ = map.occupy(base.pos, (base.faction, base.id));
            return None;
        }

        let best_pos = match base.seek_omnivore(map) {
            super::Seek::Food(pos) => Some(pos),
            super::Seek::Enemy(_pos) => {
                // curl up and hide
                self.curled = true;
                let _ = map.free(base.pos, (base.faction, base.id));
                base.speed = PILLBUG_CURL_TIME;
                return None;
            }
            super::Seek::Nothing => None,
        };

        // only try and do these if we are "safe"
        if base.update_reproduce(PILLBUG_REPRODUCE_COST) {
            return Some(Event::Birth(Box::new(Self::new(base.pos))));
        }
        if base.hunger < PILLBUG_EAT_THRESHOLD && base.try_eat(map) {
            base.hunger += PILLBUG_FOOD_VALUE;
            return None;
        }

        base.try_move(best_pos.unwrap_or_else(|| base.move_random()), map)
    }

    fn player_action(
        &mut self,
        _base: &mut BaseInsect,
        _map: &mut Map,
        _action: Action,
    ) -> Option<Event> {
        todo!()
    }

    fn draw(&self, base: &BaseInsect, map: &Map) {
        if self.curled {
            draw_cell_small(map, base.pos, color(base.faction));
        } else {
            base.draw(map);
        }
    }
}
