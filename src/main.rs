use macroquad::window::next_frame;

use ant_io::game::Game;

#[macroquad::main("Space-IO")]
async fn main() {
    let mut game = Game::new();

    loop {
        game.update();
        game.draw();
        next_frame().await;
    }
}
