
# Ant-io
Small insect simulation at Huge scale. Lead your ant/bee/beetles/pillbug population to survive in a harsh insect-eats-insect world.

## Singleplayer MVP
 - Choose your faction and battle for control of the lawn
 - You control just another bug in the battle for survival
 - Procedurally generated map

## Interesting things to do!!!
 - Enimies: Big bettles, centipedes, hordes, etc...
 - Weather: Rain, Snow, winter, summer, etc...
 - Interactions: Dig down, caves and stuff down there
   - Cave egosystem!! with insects down there eating stuff
   - Big scary insects too (depends on how deep you go?)
   - Dirt falls down eventually?
   - Water can flood stuff?
   - Caves naturally formed by insects moving around
 - Climb up trees, ecosystem up there!!!
   - Aphids eating tree
   - stuff eating fruit
   - stuff eating stuff eating fruit
   - safe from larger carnivores that can't climb

## TODO
 - [ ] New combat system
 - [ ] Rework birth to events with above, and add eggs
 - [ ] Add underground layer
 - [ ] All of UI
 - [ ] Tileset
 - [ ] Animations
 - [ ] 

## Design
- Goal: Survive (Unlocks for surviving X amount of time, reaching X pop, etc...)
  - Tutorial goals like: survive to adulthood, reproduce, etc...
- Obstacles: Starvation, predators, limited sight distance!
- Actions: Personally run around, AND manage global species traits (Soldiers vs Ants, Attack vs Defend, etc...)
- Get your "Faction" to win! By personally doing well and recruiting friends!
- For this to work like Agar.io movement needs to be intereseting
  - Animations for moving, attacking, etc...
  - Digging, attacking requires some skill 
- Health bar
  - Dmg = Attack - Armor (attacking uses hunger)
  - Regen if hunger is > 50%, uses hunger
  - Value based on body weight (less speed)
- Hunger bar
  - Refilled by eat
  - Lowered by move, attack, reproduce, dig
  - Reproduce by laying eggs


### Faction Actions
- Woodlouse: Eat X food to grow to adulthood, once adult reproduce (turorial insect due to ease of use), cheat and have war scent
- Ants: Workers vs Soldiers vs Queens, Attack vs Def, Target pop, etc... (lots of options)
  - Reach 100, 200, etc... pop in a single nest
  - Reach 1000, 2000, etc... pop globally
 
- Predators: more interesting to play, fewer pop but more likely to survive
- Termites: Ants but eat dead plants instead of seeds/carnivores
- Plants: Drop seeds around the same time
- Spiders: Web creation, catches bugs, loses straight up but can trap prey so they can't fight back
- Worms: Underground only (usually), grow by splitting (unrealistic but fun), eat regular dirt?
- Centipedes: Predators, become more powerful the more people eaten
- Millipedes: Difficult to fight herbivores, become more powerful the more eaten
- Aphids: Eat plants
- Beatles: Strong predators
- Flying insects?

Needs some kind of basic combat system
 - Shells should impart some kind of "armor" rating.
 - Large jaws have higher attack
 - Small jaws cannot overcome shells
 - Other than that
   - Health system, so that battles last a litle while and are interesting
   - Centipedes/worms global vs segment health pool?
   - Cute starcraft-esque health bars

## Questions
- RTS/Citysim vs Control single insect (more fun maybe?)
- How in-depth does the simulation need to be?
- How to manage difficulty with so many random actions

## Ideas
- Drop grid-only movement (allow free movement), wouldn't be as hard as you might think, pheremones, etc... could still be on the grid
  - If ai still follows grid, might not be that hard to code
  - Potentially more computation, depends on how it is done vs faked
  - could be cool-looking, like ant-farm
  - ants will do better due to less space to explore...
- Side-view instead of top-down (explore depths/trees more), might look better as pixel-art with CRT shaders a. la. animal well... IMO
  - Add jumping for more fun movement
  - makes height easier to convey...
  - makes sense if we really explore height and depth, otherwise it would be dumb

 - Creature creator like SPORE, but with real costs to things (wings have higher energy use, eyes have higher energy use and longer growth time, etc...)
  - Tailor your creature to a specific niche
    - Spider-eating: Anti-web feet, crushing jaws
    - Spider: Web-weaving, web sense, big eyes
    - Ant-eating: Fast
    - Vegetarian: Eats plants, shell/spikes/poison to deter predators.
    - Ants vs termites based on what they eat and their habitats
    - Diggers: Dig around and eat plant roots?
    - Aphids: Eat plants
    - Ant-lions: Camoflauge and dig holes
  - This seems very complicated, but much more interesting than just sim-ant... (or any other ant-rts honestly...)
- sandbox mode where you can just play around!

- Some kind of nest mechanic for ants and spiders
  - Drop off food in nest
  - Return to nest?
  - Queen ant in nest?
