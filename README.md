# mcskinview
A Minecraft skin viewer (written in Rust)

Use arrow keys to rotate.  Press Q to quit.

Licensed under CC0, because I don't care what you do with this.  It's a toy.

## Compiling

1. If you are running Python 3.5 or later, make sure you have wheel 0.25.0 or later installed.  Otherwise, numpy fails to build.  Well, kind of.  It looks like it fails, but apparently it succeeds.  LOL WUT?
2. `pip install -r requirements.txt`
3. `python3 vtx.py --rust -o src/steve.rs steve.dae`
4. `cargo build`
