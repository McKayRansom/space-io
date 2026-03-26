# Space IO

A 2D orbital mechanics prototype built with [Bevy 0.15](https://bevyengine.org/) (Rust). Inspired by KSP — fly a rocket, reach orbit, land on the moon.

## Running

```
cargo run
```

## Controls

| Key | Action |
|-----|--------|
| `A` / `←` | Rotate left |
| `D` / `→` | Rotate right |
| `Space` / `↑` / `W` | Main engine thrust |
| `R` | Reset to starting orbit |
| `M` | Toggle map view (zoom out 10×) |

## What's implemented

- **N-body gravity** — the rocket and moon are both pulled by all other bodies
- **Orbital trajectory prediction** — draws the current Keplerian ellipse in real time; turns orange if the periapsis intersects the planet
- **Landing and crash detection** — safe landing if relative speed ≤ 400 m/s; crash otherwise
- **Moon in orbit** — the moon has its own mass and velocity and can be landed on
- **Fuel system** — thrust consumes fuel; running out cuts the engine
- **Exhaust flame** — child entity of the rocket, positioned and rotated automatically via Bevy's transform hierarchy
- **Map view** — press `M` to smoothly zoom out and see the whole system; press again to return
- **HUD** — altitude, velocity, fuel, and status overlays
- **FPS counter**
- **Atmosphere glow** — visual ring around the planet (no drag physics)
- **Starfield** — 300 randomly placed background stars

## Possible TODO

### Gameplay
- [ ] Multiple fuel tanks or refueling on the surface
- [ ] Separate burn stages / jettison mechanics
- [ ] Throttle control (analog, not just 0/1)
- [ ] Return-to-launch objective — score or mission structure
- [ ] More celestial bodies (second moon, asteroid belt)
- [ ] Gravity assists — use the moon's gravity to redirect trajectory

### Physics
- [ ] Atmospheric drag on low-altitude passes
- [ ] Proper Verlet or RK4 integration (currently Euler)
- [ ] Trajectory prediction that accounts for the moon's gravity

### Visuals & UI
- [ ] Animated exhaust (flickering alpha or particle effect)
- [ ] Surface terrain / elevation on the planet and moon
- [ ] Crash animation
- [ ] Maneuver node planning on the map view
- [ ] Prograde/retrograde/normal markers on the HUD
- [ ] Navball or heading indicator

### Code / Architecture
- [ ] Split `main.rs` into modules as the file grows
- [ ] Configurable constants via a TOML file or in-game editor
- [ ] Save/load state
