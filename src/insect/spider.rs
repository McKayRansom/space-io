use crate::{
    insect::{Action, BaseInsect, Event, Insect, InsectBehaviour, InsectInfo},
    map::{FACTION_SPIDER, Map},
    pos::Pos,
};

use super::Hunger;

pub struct Spider {}

const SPIDER_REPRODUCE_COST: Hunger = 512;
const SPIDER_EAT_VAL: Hunger = 255;

const SPIDER_INFO: InsectInfo = InsectInfo {
    max_health: 3,
    max_speed: 2,
    damage: 2,
};

impl Spider {
    pub fn new(pos: Pos) -> Insect {
        Insect {
            base: BaseInsect::new(pos, FACTION_SPIDER, &SPIDER_INFO),
            spec: Box::new(Self {}),
        }
    }
}

impl InsectBehaviour for Spider {
    fn update(&mut self, base: &mut BaseInsect, map: &mut Map) -> Option<Event> {
        if base.update_reproduce(SPIDER_REPRODUCE_COST) {
            return Some(Event::Birth(Box::new(Self::new(base.pos))));
        }

        if base.hunger < SPIDER_REPRODUCE_COST && base.try_eat(map) {
            base.hunger += SPIDER_EAT_VAL;
            // eating should take a tick IMO
            return None;
        }

        match base.seek_omnivore(map) {
            super::Seek::Food(pos) => base.try_move(pos, map),
            super::Seek::Enemy(pos) => base.try_move(pos, map),
            super::Seek::Nothing => None,
        }
    }

    fn player_action(
        &mut self,
        base: &mut BaseInsect,
        _map: &mut Map,
        action: Action,
    ) -> Option<Event> {
        match action {
            Action::ActionA => {
                if base.update_reproduce(SPIDER_REPRODUCE_COST) {
                    return Some(Event::Birth(Box::new(Self::new(base.pos))));
                }
            }

            Action::ActionB => todo!(),
            _ => {}
        }
        None
    }
    
    fn draw(&self, base: &BaseInsect, map: &Map) {
        base.draw(map);
    }
}
