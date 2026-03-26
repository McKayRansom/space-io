use crate::{draw::{color, draw_cell_small}, insect::{BaseInsect, Insect, InsectBehaviour, InsectInfo}, pos::Pos};

pub struct Egg {
    germination: u8,
    hatch: Option<Box<Insect>>,
}

const EGG_INFO: InsectInfo = InsectInfo {
    max_health: 1,
    max_speed: 0,
    damage: 0,
};

impl Egg {
    pub fn new(pos: Pos, hatch: Insect, germination: u8) -> Insect {
        Insect {
            base: BaseInsect::new(pos, hatch.base.faction, &EGG_INFO),
            spec: Box::new(Self {
                germination,
                hatch: Some(Box::new(hatch)),
            }),
        }
    }
}

impl InsectBehaviour for Egg {
    fn update(&mut self, _base: &mut BaseInsect, _map: &mut crate::map::Map) -> Option<super::Event> {
        self.germination = self.germination.saturating_sub(1);
        if self.germination == 0 && self.hatch.is_some() {
            Some(super::Event::Rebirth(self.hatch.take().unwrap()))
        } else {
            None
        }
    }

    fn player_action(&mut self, _base: &mut BaseInsect, _map: &mut crate::map::Map, _action: super::Action) -> Option<super::Event> {
        todo!()
    }
    
    fn draw(&self, base: &BaseInsect, map: &crate::map::Map) {
        draw_cell_small(map, base.pos, color(base.faction));
    }
}
