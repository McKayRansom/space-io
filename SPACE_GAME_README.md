# Space-IO: 2D Space Game

A Rust-based 2D space game built with macroquad featuring realistic orbital mechanics with fixed-point arithmetic.

## Features

- **Fixed-Point Physics**: Deterministic physics calculations using i64 fixed-point math for position, velocity, and acceleration vectors
- **Realistic Gravity**: Two-body gravitational simulation between planets and moons
- **Orbital Mechanics**: See predicted orbit paths with periapsis (green) and apoapsis (red) markers
- **Landing System**: Land on both the planet and moon when approaching at safe velocities
- **Fuel Management**: Limited fuel system that depletes with thrust usage and refuels when landed
- **Interactive Controls**: Arrow keys or WASD to control the spaceship

## Controls

- **Left Arrow / A**: Rotate left
- **Right Arrow / D**: Rotate right  
- **Up Arrow / W**: Apply thrust (only when flying)
- **Space**: Launch from planet/moon (only when landed)

## Game Elements

- **Blue Circle**: Home planet (Earth) - starting location
- **Gray Circle**: Moon orbiting the planet
- **White Square**: Your spaceship with red nose indicator
- **Yellow Line**: Predicted orbit path
- **Green Dot**: Periapsis (closest approach to planet)
- **Red Dot**: Apoapsis (farthest point from planet)

## Physics System

### Fixed-Point Math
The game uses 32-bit fixed-point arithmetic (32-bit integer, 32-bit fractional parts) stored in i64 for perfect determinism. This ensures:
- No floating-point rounding errors
- Perfect reproducibility across different platforms
- Accurate physics over long simulations

### Gravitational Model
Both the planet and moon exert gravitational pull on the spaceship using the formula:
```
acceleration = (G * M) / distance²
```

Where:
- G = gravitational constant (0.00001)
- M = mass of the celestial body
- distance = distance from the spaceship to the body

### Landing Mechanics
- Approach within the landing zone with velocity < 50 m/s
- Ship will automatically land and refuel
- Fuel refuels by 10 liters per update frame while landed

## Building and Running

### Debug Build
```bash
cargo build
cargo run
```

### Release Build (Optimized)
```bash
cargo build --release
cargo run --release
```

## Project Structure

```
src/
├── main.rs          - Entry point, game loop
├── lib.rs           - Library exports
├── game.rs          - Main game struct and logic
├── physics.rs       - Fixed-point math and vector operations
├── spaceship.rs     - Spaceship state and behavior
└── celestial.rs     - Celestial bodies and orbital mechanics
```

## Technical Details

### Coordinate System
- World coordinates: floating-point positions in game space
- Screen coordinates: pixels, with camera centered on spaceship
- Scale: 0.001 pixels per unit (1000 units = 1 pixel at default zoom)

### Time Step
- Fixed timestep: 16ms (~60 FPS)
- Physics updates: Deterministic with fixed-point math

### Orbital Prediction
The orbit prediction line is calculated using classical orbital mechanics:
- Semi-major axis computation from specific orbital energy
- Eccentricity calculation from orbit equation
- Position along orbit using the vis-viva equation

## Game Tips

1. **Starting**: Press Space to launch from the planet
2. **Orbiting**: Use gentle thrust to achieve stable orbits - watch your speed!
3. **Landing**: Slow down before approaching to land safely
4. **Moon Visits**: The moon orbits, so you may need to lead it with your trajectory
5. **Fuel Economics**: Plan your burns carefully - refueling requires landing
6. **Orbit Visualization**: Use the yellow prediction line to plan your maneuvers

## Future Enhancements

- Multiple planets and moons
- Asteroid fields
- Atmospheric entry/re-entry mechanics
- Time acceleration controls
- Save/load game state
- Missions and objectives
- More sophisticated orbital mechanics calculations
