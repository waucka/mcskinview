# mcskinview
A Minecraft skin viewer (written in Rust)

Use arrow keys to rotate.  Press A to toggle animation.  Press Q to quit.

Licensed under CC0, because I don't care what you do with this.  It's a toy.

## Compiling

1. If you are running Python 3.5 or later, make sure you have wheel 0.25.0 or later installed.  Otherwise, numpy fails to build.  Well, kind of.  It looks like it fails, but apparently it succeeds.  LOL WUT?
2. `pip install -r requirements.txt`
3. `./vtx.py -c src/steve_common.rs -m steve.dae:src/steve.rs -m steve_1.7.dae:src/steve17.rs`
4. `cargo build`

## Running

- `cargo run -- -s some_minecraft_1.8_skin.png`
- `cargo run -- -s some_minecraft_1.7_skin.png -m`

## Getting your skin

Don't have your skin file handy?  Just run `./getskin.py YOUR_USERNAME_HERE`.
