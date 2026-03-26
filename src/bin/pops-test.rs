use ant_io::game::Game;

pub fn main() {
    let mut game: Game = Game::new();

    for _i in 0..4096 {
        game.update();
    }
    
    println!("Simulation ran for 4096 frames");
}
