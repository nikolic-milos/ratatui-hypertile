![demo](assets/demo.gif)

[![CI](https://github.com/nikolic-milos/ratatui-hypertile/actions/workflows/ci.yml/badge.svg)](https://github.com/nikolic-milos/ratatui-hypertile/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/ratatui-hypertile.svg)](https://crates.io/crates/ratatui-hypertile)
[![Docs.rs](https://docs.rs/ratatui-hypertile/badge.svg)](https://docs.rs/ratatui-hypertile)

Cook up delicious terminal interfaces with Hyprland-style tiling for [Ratatui](https://github.com/ratatui/ratatui). Splits, tabs, animations, persistence.

## What's in the box

[`ratatui-hypertile`](https://crates.io/crates/ratatui-hypertile) is the core engine. You give it an area, it gives you rectangles. Handles the tree, focus, movement. Use this when you want full control.

[`ratatui-hypertile-extras`](https://crates.io/crates/ratatui-hypertile-extras) wraps the core into a runtime with plugins, vim keymaps, a command palette, workspace tabs and pane-move animations. Implement `HypertilePlugin` and you're set.

## Quickstart

From the repo root:

```sh
# full runtime: plugins, tabs, palette, animations
cargo run -p ratatui-hypertile-extras --example basic

# core only, no extras dependency
cargo run --example core_only
```

**Keys (basic example):**

`s` `v` split &ensp; `d` close &ensp; `hjkl` focus &ensp; `HJKL` move &ensp; `[` `]` resize
`p` palette &ensp; `i` plugin input &ensp; `Ctrl+t` new tab &ensp; `Ctrl+w` close tab &ensp; `Ctrl+c` quit

## Wire it up

```toml
# just the layout engine
ratatui-hypertile = "0.3"

# or the full runtime with plugins
ratatui-hypertile-extras = "0.3"
```

```rust
use ratatui::layout::Direction;
use ratatui_hypertile::Hypertile;

let mut layout = Hypertile::new();
let pane = layout.split_focused(Direction::Horizontal).unwrap();

layout.compute_layout(area);
for pane in layout.panes_iter() {
    // pane.id, pane.rect, pane.is_focused
}
```

## License

This project is licensed under the [MIT License](LICENSE).
