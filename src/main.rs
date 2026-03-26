use macroquad::window::next_frame;

use ant_io::game::Game;



#[macroquad::main("Ant-IO")]
async fn main() {

    let mut game = Game::new();
    game.generate();

    let mut won = 0;
    let mut lost = 0;

    loop {
        if game.update(won, lost) {
            if game.game_won {
                won += 1;
            } else {
                lost += 1;
            }
            game = Game::new();
        }
        next_frame().await;
    }
}
