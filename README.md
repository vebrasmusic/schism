# Schism

Schism is a small Rust simulation for watching religions split, survive, and die out over many generations.

The motivation is to model religious history as a population system: people are born, die, inherit some level of heterodoxy, and sometimes a faith cracks along that fault line. A schism creates a child religion, but it only matters if enough adherents convert for the new group to survive.

At a high level, the engine runs generation by generation. It updates adherents, checks whether each active religion schisms, moves heterodox adherents into new sects, marks extinct religions, and prints a compact JSON readout of the resulting religious tree.

## Run

```sh
git clone git@github.com:vebrasmusic/schism.git
cd schism/apps/engine
cargo run --release -- run
```

Use the release build. Plain `cargo run -- run` is much slower and makes larger simulations painful.

Example with more generations:

```sh
cargo run --release -- run -n 200
```
