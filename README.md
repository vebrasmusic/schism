# Schism

Schism is a small Rust simulation for watching religions branch, persist, and disappear across long stretches of simulated history.

The project is an experiment in treating religious change as an emergent population story rather than a fixed timeline. Communities grow, drift, fracture, and sometimes leave descendants that become meaningful traditions of their own. The details will keep changing, but the goal is the same: produce plausible religious family trees from simple rules.

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
