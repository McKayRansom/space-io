use macroquad::color::colors;

use crate::{
    draw::draw_cell,
    insect::{Insect, InsectBehaviour},
    map::CellType,
};

pub struct InsectPlayer {
    sub_insect: Box<dyn InsectBehaviour>,
}

impl InsectPlayer {
    pub fn new(insect: Insect) -> Insect {
        Insect {
            base: insect.base,
            spec: Box::new(Self {
                sub_insect: insect.spec,
            }),
        }
    }
}

impl InsectBehaviour for InsectPlayer {
    fn update(
        &mut self,
        base: &mut super::BaseInsect,
        map: &mut crate::map::Map,
    ) -> Option<super::Event> {
        // Do nothing, because we want the player to do stuff...

        if base.hunger < 512 {
            if let Some(_food) = map
                .get_cell_mut(base.pos)
                .unwrap()
                .take_type(CellType::Food)
            {
                base.hunger += 255;
            }
        }
        None
    }

    fn player_action(
        &mut self,
        base: &mut super::BaseInsect,
        map: &mut crate::map::Map,
        action: super::Action,
    ) -> Option<super::Event> {
        match action {
            super::Action::Move(pos) => {
                // WHAAAAA
                // if BaseInsect::will_move(&mut self.speed, max_speed)
                if let Some(event) = base.try_move(base.pos + pos, map) {
                    return Some(event);
                }
            }
            _ => {},
        }
        self.sub_insect.player_action(base, map, action)
    }

    fn draw(&self, base: &super::BaseInsect, map: &crate::map::Map) {
        draw_cell(map, base.pos, colors::WHITE);
        // change color???
        self.sub_insect.draw(base, map);
        // draw_
    }
}
