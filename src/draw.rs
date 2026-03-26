use macroquad::{
    color::{Color, colors},
    shapes::draw_rectangle,
};

use crate::{
    game::Game,
    insect::ant::Scents,
    map::{CellType, FACTION_CENTIPEDE, FACTION_PILLBUG, FACTION_SPIDER, Faction, Map},
    pos::Pos,
};

pub const FOOD_COLOR: Color = colors::GREEN;

pub const PILLBUG_COLOR: Color = colors::PINK;
pub const SPIDER_COLOR: Color = colors::ORANGE;

pub fn draw_game(game: &Game) {
    draw_map(&game.map);

    if game.show_scents != 0 {
        draw_scents( &game.map, game.show_scents);
    }
    // for colony in &game.ant_colonies {
    //     draw_colony(colony, &game.map);
    // }
    for (_id, insect) in &game.insects {
        // let insect = self.insects_idinsect.insect();
        insect.spec.draw(&insect.base, &game.map);
    //     draw_ant(ants, &map, {
    //         let mut color = color(colony.faction);
    //         color.r += 0.2;
    //         color.g += 0.2;
    //         color.b += 0.2;
    //         color
    //     });
    // }
    }
}

pub fn draw_map(map: &Map) {
    let pos = map.screen_pos((0, 0).into());
    draw_rectangle(
        pos.x,
        pos.y,
        map.size.x as f32 * Map::TILE_SIZE_DEFAULT * map.camera.zoom,
        map.size.y as f32 * Map::TILE_SIZE_DEFAULT * map.camera.zoom,
        colors::DARKBROWN,
    );

    for y in 0..map.occupied.len() {
        let row = &map.occupied[y];
        for x in 0..row.len() {
            let point: Pos = Pos::new(x as i16, y as i16);
            let cell = &row[x];
            match cell.m_type {
                CellType::Empty => {}
                CellType::Food => draw_cell(map, point, FOOD_COLOR),
                CellType::Nest(_) => draw_cell(map, point, colors::WHITE),
                CellType::Rock => draw_cell(map, point, colors::GRAY),
                CellType::Wall => {}
            }
        }
    }
}

pub fn draw_scents(map: &Map, id: Faction) {
    let mut pos = Pos::new(0, 0);
    for cell in map.get_scent_grid(id as u8).grid.iter() {
        pos.x += 1;
        if pos.x >= map.size.x {
            pos.x = 0;
            pos.y += 1;
        }
        // draw scents
        if cell.get_scent(Scents::Attack) > 0 {
            let scent_alpha = cell.get_scent(Scents::Attack) as f32 / u8::MAX as f32;
            let mut color = colors::RED;
            color.a = scent_alpha / 2.;
            draw_cell(map, pos, color);
        } else if cell.get_scent(Scents::Food) > 0 {
            let scent_alpha = cell.get_scent(Scents::Food) as f32 / u8::MAX as f32;
            let mut color = colors::GREEN;
            color.a = scent_alpha / 2.;
            draw_cell(map, pos, color);
        } else if cell.get_scent(Scents::Nest) > 0 {
            let scent_alpha = cell.get_scent(Scents::Nest) as f32 / u8::MAX as f32;
            let mut color = colors::LIGHTGRAY;
            color.a = scent_alpha / 2.;
            draw_cell(map, pos, color);
        }
    }
}

pub fn draw_cell(map: &Map, pos: Pos, color: Color) {
    let rect = map.screen_rect(pos);
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, color);
}

pub fn draw_cell_medium(map: &Map, pos: Pos, color: Color) {
    let rect = map.screen_rect(pos);
    let margin = rect.w / 8.;
    draw_rectangle(
        rect.x + margin,
        rect.y + margin,
        rect.w - margin * 2.,
        rect.h - margin * 2.,
        color,
    );
}

pub fn draw_cell_small(map: &Map, pos: Pos, color: Color) {
    let rect = map.screen_rect(pos);
    let margin = rect.w / 4.;
    draw_rectangle(
        rect.x + margin,
        rect.y + margin,
        rect.w - margin * 2.,
        rect.h - margin * 2.,
        color,
    );
}

pub fn color(faction: Faction) -> Color {
    match faction {
        0 => colors::WHITE,
        1 => colors::BLUE,
        2 => colors::RED,
        FACTION_PILLBUG => PILLBUG_COLOR,
        FACTION_SPIDER => SPIDER_COLOR,
        FACTION_CENTIPEDE => colors::PURPLE,
        _ => unimplemented!("Faction: {}", faction),
    }
}

pub fn name(faction: Faction) -> &'static str {
    match faction {
        0 => "UNKNOWN",
        1 => "BLUE ANTS",
        2 => "RED ANTS",
        FACTION_PILLBUG => "PILLBUGS",
        FACTION_SPIDER => "SPIDERS",
        FACTION_CENTIPEDE => "CENTIPEDE",
        _ => unimplemented!("Faction: {}", faction),
    }
}
