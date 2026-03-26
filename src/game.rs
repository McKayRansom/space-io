use std::collections::HashMap;

use macroquad::color::colors;
use macroquad::prelude::*;

use crate::draw::{color, draw_game};
use crate::insect::ant::{Ant, AntQueen};
use crate::insect::centipede::Centipede;
use crate::insect::pillbug::Pillbug;
use crate::insect::player::InsectPlayer;
use crate::insect::spider::Spider;
use crate::insect::{Action, Event, Id, Insect, Interact};
use crate::map::{FACTION_CENTIPEDE, FACTION_SPIDER, Faction, MAP_SIZE, Map};
// use crate::grid::{DOWN, Grid, LEFT, RIGHT, SQUARES, UP};
use crate::pos::{Pos, dirs};

pub enum Speed {
    SLOW,
    FAST,
}

const SPEED_FAST: f64 = 0.1;
const SPEED_SLOW: f64 = 0.25;

impl Speed {
    pub fn val(&self) -> f64 {
        match self {
            Speed::SLOW => SPEED_FAST,
            Speed::FAST => SPEED_SLOW,
        }
    }

    fn invert(&self) -> Speed {
        match self {
            Speed::SLOW => Speed::FAST,
            Speed::FAST => Speed::SLOW,
        }
    }
}

pub struct Game {
    pub map: Map,

    speed: Speed,
    last_update: f64,
    game_over: bool,
    pub game_won: bool,
    pub player_id: Id,
    pub player_last_pos: Pos,
    pub player_dir: Pos,
    pub player_faction: Faction,
    pub show_scents: Faction,
    pub paused: bool,
    // pub pillbugs: Vec<Pillbug>,
    // pub spiders: Vec<Spider>,
    pub insects: HashMap<Id, Insect>,
    pub insects_id: Id,
    pub pops: HashMap<Faction, usize>,
    pub queens: HashMap<Faction, Id>,
}

const NEST_POS: Pos = Pos::new(MAP_SIZE - 20, MAP_SIZE - 10);
const NEST_POS_2: Pos = Pos::new(20, 10);

const STARTING_PILLBUGS: usize = 250;
const STARTING_SPIDERS: usize = 50;
const STARTING_ANTS: usize = 128;

const FACTION_BLUE_ANTS: Faction = 1;
const FACTION_RED_ANTS: Faction = 2;

impl Game {
    pub fn new() -> Self {
        let map = Map::new();

        Self {
            // pillbugs: (0..STARTING_PILLBUGS)
            //     .into_iter()
            //     .map(|_| Pillbug::new(map.rand_pos()))
            //     .collect(),
            // spiders: (0..STARTING_SPIDERS)
            //     .into_iter()
            //     .map(|_| Spider::new(map.rand_pos()))
            //     .collect(),
            // player: Spider::new(map.rand_pos()),
            insects: HashMap::new(),
            insects_id: 1,
            player_dir: dirs::NONE,
            player_id: 0,
            player_last_pos: dirs::NONE,
            player_faction: FACTION_CENTIPEDE,
            map,
            speed: Speed::SLOW,
            last_update: 0.,
            game_over: false,
            game_won: true,
            show_scents: 0,
            paused: false,
            pops: HashMap::new(),
            queens: HashMap::new(),
        }
    }

    pub fn spawn_insect(&mut self, mut insect: Insect) -> Id {
        let id = self.insects_id;
        self.insects_id += 1;
        insect.base.id = id;
        let _ = self.map.occupy(insect.base.pos, (insect.base.faction, id));
        self.insects.insert(id, insect);
        // oofarinos
        id
    }

    pub fn spawn_ant_colony(&mut self, pos: Pos, faction: Faction) {
        let queen = self.spawn_insect(AntQueen::new(pos, faction));
        self.queens.insert(faction, queen);
        for _ in 0..STARTING_ANTS {
            self.spawn_insect(Ant::new(pos, faction, queen));
        }
    }

    pub fn generate(&mut self) {
        for _ in 0..STARTING_PILLBUGS {
            self.spawn_insect(Pillbug::new(self.map.rand_pos()));
        }
        for _ in 0..STARTING_SPIDERS {
            self.spawn_insect(Spider::new(self.map.rand_pos()));
        }
        for _ in 0..10 {
            self.spawn_insect(Centipede::new(self.map.rand_pos()));
        }

        self.spawn_ant_colony(NEST_POS, FACTION_BLUE_ANTS);
        self.spawn_ant_colony(NEST_POS_2, FACTION_RED_ANTS);

        self.spawn_player();
        // self.player_id = self.spawn_insect(InsectPlayer::new(Spider::new(self.map.rand_pos())));
    }

    /// NOTE: will be run more than once per sim tick!! must handle this correctly
    pub fn update_player_input(&mut self) {
        let mut input_dir = dirs::NONE;
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            input_dir.x = 1;
        } else if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            input_dir.x = -1;
        }
        if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
            input_dir.y = -1;
        } else if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
            input_dir.y = 1;
        }
        self.player_dir = input_dir;

        // if is_key_down(KeyCode::LeftShift) {
        //     self.player.food_scent = u8::MAX;
        // } else {
        //     self.player.food_scent = 0;
        // }
        // TODO

        if is_key_pressed(KeyCode::Key1) {
            if self.show_scents == 1 {
                self.show_scents = 0;
            } else {
                self.show_scents = 1;
            }
        }
        if is_key_pressed(KeyCode::Key2) {
            if self.show_scents == 2 {
                self.show_scents = 0;
            } else {
                self.show_scents = 2;
            }
        }

        if is_key_pressed(KeyCode::Minus) {
            self.map.camera.change_zoom(-0.1);
        }
        if is_key_pressed(KeyCode::Equal) {
            self.map.camera.change_zoom(0.1);
        }

        if is_key_pressed(KeyCode::Space) {
            self.paused = !self.paused;
        }
        if is_key_pressed(KeyCode::F) {
            self.speed = self.speed.invert();
        }
    }

    pub fn update_player(&mut self) -> Option<Event> {
        if let Some(player) = self.insects.get_mut(&self.player_id) {
            if is_key_down(KeyCode::X) {
                //     self.player.reproduce = u8::MAX;
                //     // self.player.insect.hunger = 550;
                // } else {
                //     self.player.reproduce = 0;
                if let Some(action) = player.player_action(&mut self.map, Action::ActionA) {
                    return Some(action);
                }
            }
            self.player_last_pos = player.base.pos;
            if self.player_dir != dirs::NONE {
                return player.player_action(&mut self.map, Action::Move(self.player_dir));
            }
        }

        None

        // TODO
        // if is_key_down(KeyCode::X) {
        //     self.player.reproduce = u8::MAX;
        //     // self.player.insect.hunger = 550;
        // } else {
        //     sel

        // let colony = &mut self.ant_colonies[0];

        // let scent = self.player.food_scent;

        // let _seeking = self
        //     .player
        //     .update_behaviour(&mut self.map, &mut colony.food, 1);

        // self.player.update_scents(&mut colony.scents);

        // self.player.food_scent = scent;
        // self.player.insect.hunger = u16::MAX;

        // if self.player.insect.update(
        //     Some(self.player.insect.pos + self.player.insect.dir),
        //     &mut self.map,
        //     self.ant_colonies[0].faction,
        // ) == false
        // {
        //     // we dead, make new player
        //     self.player = Ant::new(NEST_POS, 1);
        // }
    }

    pub fn update_sim(&mut self) {
        let mut new_bugs: Vec<Box<Insect>> = Vec::new();
        let mut dead_bugs: Vec<Id> = Vec::new();
        let mut interacts: Vec<Interact> = Vec::new();
        for pop in self.pops.values_mut() {
            *pop = 0;
        }

        for (id, insect) in self.insects.iter_mut() {
            *self.pops.entry(insect.base.faction).or_insert(0) += 1;
            match insect.update(&mut self.map) {
                Some(Event::Interact(pos)) => interacts.push(pos),
                Some(Event::Rebirth(bug)) => {
                    new_bugs.push(bug);
                    dead_bugs.push(*id);
                }
                Some(Event::Birth(bug)) => new_bugs.push(bug),
                Some(Event::Death()) => dead_bugs.push(*id),
                None => {}
            };
        }

        match self.update_player() {
            Some(Event::Death()) => {
                dead_bugs.push(self.player_id);
                // creat new insect for the player
                // TODO: not a spider???
            }
            Some(Event::Interact(pos)) => interacts.push(pos),
            Some(Event::Rebirth(bug)) => {
                // new_bugs.push(bug);
                dead_bugs.push(self.player_id);
                self.player_id = self.spawn_insect(*bug);
            }
            Some(Event::Birth(bug)) => new_bugs.push(bug),
            // Some(Event::Death()) => dead_bugs.push(),
            None => {}
        }

        while let Some(bug) = dead_bugs.pop() {
            if let Some(insect) = self.insects.remove(&bug) {
                insect.die(&mut self.map);
            }
            // could add on_death call here...
        }

        while let Some(bug) = new_bugs.pop() {
            self.spawn_insect(*bug);
        }

        while let Some(interact) = interacts.pop() {
            if let Some(insect) = self.insects.get_mut(&interact.dst) {
                insect.interact(interact)
            }
        }

        if !self.insects.contains_key(&self.player_id) {
            self.spawn_player();
        }

        self.map.update();
    }

    pub fn update(&mut self, _won: u32, _lost: u32) -> bool {
        self.update_player_input();
        if !self.game_over {
            if !self.paused && get_time() - self.last_update > self.speed.val() {
                self.last_update = get_time();

                self.update_sim();

                // if all_snakes_dead {
                //     self.game_won = true;
                //     self.game_over = true;
                // }

                // self.update_player();
            }
        }

        // let mut player_color = self.ants[0].body_color;
        // player_color.r *= 0.5;
        // player_color.g *= 0.5;
        // player_color.b *= 0.5;

        clear_background(colors::BLACK);

        self.map.update_size(self.player_last_pos);
        draw_game(&self);

        let mut new_pops: Vec<(u8, usize)> =
            self.pops.iter().map(|(key, val)| (*key, *val)).collect();
        new_pops.sort_by(|a, b| b.1.cmp(&a.1)); // sort largest to smallest

        let mut y: f32 = 20.;
        for (faction, pop) in &new_pops {
            let text = if let Some(queen) = self.queens.get(faction) {
                format!(
                    "{}: Pop: {} Food: {}",
                    crate::draw::name(*faction),
                    pop,
                    self.insects[queen].base.hunger
                )
            } else {
                format!("{}: {}", crate::draw::name(*faction), pop)
            };
            draw_text(text.as_str(), 10., y, 24., color(*faction));
            y += 20.;
        }

        if let Some(player) = self.insects.get(&self.player_id) {
            draw_text(
                format!(
                    "P Health: {}, Hunger: {}",
                    player.base.health, player.base.hunger
                )
                .as_str(),
                10.,
                y,
                24.,
                colors::YELLOW,
            );
        }

        // if self.game_over {
        //     // clear_background(BLACK);
        //     let text = if self.game_won {
        //         "Game Won! Press [enter] to play agin."
        //     } else {
        //         "Game Over. Press [enter] to play again."
        //     };
        //     let font_size = 30.;
        //     let text_size = measure_text(text, None, font_size as _, 1.0);

        //     draw_text(
        //         text,
        //         screen_width() / 2. - text_size.width / 2.,
        //         screen_height() / 2. + text_size.height / 2.,
        //         font_size,
        //         WHITE,
        //     );

        //     if is_key_down(KeyCode::Enter) {
        //         // start new game
        //         return true;
        //     }
        // }
        false
    }

    fn spawn_player(&mut self) {
        if let Some((id, _insect)) = self
            .insects
            .iter()
            .find(|(_id, insect)| insect.base.faction == self.player_faction)
        {
            let id = *id;
            let insect = self.insects.remove(&id).unwrap();
            let new_insect = InsectPlayer::new(insect);
            self.player_id = new_insect.base.id;
            self.insects.insert(id, new_insect);
        }
    }
}

// impl Display for Game {
// fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
// write!(
//     f,
//     "Ant 0: Pop: {} \nAnt 1: Pop: {} \nPillbugs: {}\nSpiders: {}",
//     self.ant_colonies[0].workers.len(),
//     self.ant_colonies[1].workers.len(),
//     self.pillbugs.len(),
//     self.spiders.len()
// )
// writ("Ants[0]: {}", self.ant_colonies[0].workers.len())
// write!("GAME")
// }
// }

#[cfg(test)]
mod game_tests {
    // use super::*;

    // #[test]
    // fn test_pops() {
    //     let mut game: Game = Game::new();

    //     for _ in 0..4 {
    //         for i in 0..4096 {
    //             game.update_sim();
    //             assert!(
    //                 !game.pillbugs.is_empty(),
    //                 "Pillbugs died out at gen {}\n{}",
    //                 i,
    //                 game
    //             );
    //             assert!(
    //                 game.pillbugs.len() < 1024,
    //                 "Pillbugs overpoped at gen {}\n{}",
    //                 i,
    //                 game
    //             );

    //             assert!(
    //                 !game.spiders.is_empty(),
    //                 "Spiders died out at gen {}\n{}",
    //                 i,
    //                 game
    //             );
    //             assert!(
    //                 game.spiders.len() < 500,
    //                 "Spiders overpoped at gen {}\n{}",
    //                 i,
    //                 game
    //             );

    //             assert!(
    //                 !game.ant_colonies[0].workers.is_empty(),
    //                 "Ants[0] died out at gen {}\n{}",
    //                 i,
    //                 game
    //             );
    //             assert!(
    //                 game.ant_colonies[0].workers.len() < 500,
    //                 "Ants[0] overpoped at gen {}\n{}",
    //                 i,
    //                 game
    //             );

    //             assert!(
    //                 !game.ant_colonies[1].workers.is_empty(),
    //                 "Ants[1] died out at gen {}\n{}",
    //                 i,
    //                 game
    //             );
    //             assert!(
    //                 game.ant_colonies[1].workers.len() < 500,
    //                 "Ants[1] overpoped at gen {}\n{}",
    //                 i,
    //                 game
    //             );
    //         }

    //         println!("{}", game);
    //     }
    //     assert!(false, "PASSED");
    // }
}
