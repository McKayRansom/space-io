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

## MVP

- That KSP Alpha experience: Design a rocket and pilot it to the moon!

### VAB
- [x] Build a rocket with fuel tanks, engines, decouplers (landing gear optional!)
- [ ] Save and name rocket designs
- [ ] GUI with parts list, save/load at min
### Flight
- [x] Functioning parts
    - [x] Engines
    - [x] Fuel Tanks
    - [x] Decouplers/staging???
- [x] Reach orbit (Some kind of nav aids), staging, appoapsis/periapsis
- [ ] Reach the moon: Decided on distance/size, tweak until moonrise works? Or display predictions?
- [x] Land on the moon: Land if velocity is low enough, take off also allowed. Crash and restart as an option
- [ ] Re-entry: Atmospheric drag/maybe parachute or something to land?
- [ ] Quicksave/reaload would be nice
- [ ] HUD of some kind
  - [ ] ASCII horizontal Navball???

### Game
- [ ] Main menu, saves, settings
- [ ] Some kind of art for HUDs, GUIs
- [x] Sprites for spaceships
- [x] Particle effects for atmosphere, engines
- [x] Terrain for planets


## IDEAS

- Procedural parts: Lots of mods did this for KSP, and people were hoping this would be in KSP 2. Should actually make my life a lot easier if done correctly!
- Pixel rockets: Have people draw their rockets (full customizable) and draw the layout of fuel tanks/etc...

## Possible TODO

### Gameplay
- [ ] Throttle control (analog, not just 0/1)
- [ ] Objective list in top right, briefing on load for now
  - [ ] Objectives: Takeoff, reach orbit, reach munar orbit, land on moon, return from moon
- [ ] More celestial bodies (second moon, asteroid belt)

### Physics
- [ ] Atmospheric drag on low-altitude passes
- [ ] Proper Verlet or RK4 integration (currently Euler)
- [ ] Trajectory prediction that accounts for the moon's gravity

### Visuals & UI
- [x] Animated exhaust (flickering alpha or particle effect)
- [x] Surface terrain / elevation on the planet and moon
- [ ] Crash animation
- [ ] Maneuver node planning on the map view
- [ ] Prograde/retrograde/normal markers on the HUD
- [ ] Navball or heading indicator

### Code / Architecture
- [x] Split `main.rs` into modules as the file grows
- [x] Configurable constants via a TOML file or in-game editor
- [ ] Save/load state
