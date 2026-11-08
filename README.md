# Fire, Water, Storm, Lightning 🦖

A local multiplayer party game for 2–4 players on one keyboard, inspired by the German
gym game *Feuer, Wasser, Sturm, Blitz*. Built with [Bevy](https://bevy.org) 0.20.

## How to play

Every dino runs all the time. You can only steer it left or right.

When the game calls an element, race to a matching safe spot before the countdown ends:

| Call      | Safe spot |
|-----------|-----------|
| FIRE      | Lake      |
| WATER     | Mountain  |
| STORM     | Cave      |
| LIGHTNING | Ferns     |

When time runs out, the dino furthest from a matching spot loses one of its 3 lives.
Anyone inside a matching spot is safe, and if everyone made it, nobody loses a life.
Safe spots reshuffle every round, more of them wander around as the game goes on,
and the countdown gets shorter. Dinos are solid, so shove your rivals out of the way.
The last dino standing wins.

| Player | Keys                          |
|--------|-------------------------------|
| REX    | `A` / `D`                     |
| TRIKE  | `J` / `L`                     |
| STEGO  | `←` / `→`                     |
| RAPTOR | `Numpad 4` / `6` or `-` / `=` |

In the lobby, press one of your keys to join, then `Space`/`Enter` to start.
`Backspace` clears the players. `Esc` during a game goes back to the lobby.

Note: some keyboards can't register many keys held at once ("ghosting"). If a
player's key stops responding while others are pressed, try a different keyboard.

## Building

```sh
nix develop                  # dev shell: Rust (with wasm32 target), wasm-bindgen, binaryen
cargo run                    # in the dev shell: optimized build, quick incremental rebuilds

nix build .#native           # native binary  -> result/bin/dino-elements
nix run                      # build and run the native version
nix build .#wasm             # web build      -> result/{index.html, *.js, *.wasm}
nix run .#serve-wasm         # serve the web build on http://localhost:8080
```

Inside `nix develop`, `cargo run` is wrapped to run `cargo run --profile release-dev`.
That profile has release optimizations without LTO and with incremental compilation,
so rebuilds after a change are reasonably quick. Every other cargo command works as usual.

Bevy's `dynamic_linking` feature can't be used with Bevy 0.20 from crates.io: it
needs `bevy_dylib` 0.20.1, which isn't published there.

The `wasm-bindgen` crate version in `Cargo.lock` must match the `wasm-bindgen-cli`
used in `flake.nix` (currently 0.2.130). If you bump one, bump the other.
