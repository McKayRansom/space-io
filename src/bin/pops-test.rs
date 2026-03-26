use ant_io::game::Game;

pub fn main() {
    let mut game: Game = Game::new();

    for i in 0..4096 {
        game.update_sim();
        // assert!(!game.pillbugs.is_empty(), "Pillbugs died out at gen {}\n{}", i, game);
        // assert!(game.pillbugs.len() < 500, "Pillbugs overpoped at gen {}\n{}", i, game);

        // assert!(!game.spiders.is_empty(), "Spiders died out at gen {}\n{}", i, game);
        // assert!(game.spiders.len() < 500, "Spiders overpoped at gen {}\n{}", i, game);

        // assert!(
        //     !game.ant_colonies[0].workers.is_empty(),
        //     "Ants[0] died out at gen {}\n{}",
        //     i, game
        // );
        // assert!(
        //     game.ant_colonies[0].workers.len() < 500,
        //     "Ants[0] overpoped at gen {}\n{}",
        //     i, game
        // );

        // assert!(
        //     !game.ant_colonies[1].workers.is_empty(),
        //     "Ants[1] died out at gen {}\n{}",
        //     i, game
        // );
        // assert!(
        //     game.ant_colonies[1].workers.len() < 500,
        //     "Ants[1] overpoped at gen {}\n{}",
        //     i, game
        // );
    }

    // println!("{}", game);
}
