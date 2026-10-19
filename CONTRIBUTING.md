# Contributing to Sprinkles

Everyone is welcome to contribute bug fixes, improvements, and new features to Sprinkles!

## Repository

The project is split into two crates:

| Crate                   | Purpose       |
| ----------------------- | ------------- |
| `bevy_sprinkles`        | Core library  |
| `bevy_sprinkles_editor` | Visual editor |

## Getting started

1. Fork and clone the repository
2. Build with `cargo build`
3. Run the editor with `cargo editor`

## Making changes

### Bug fixes

Bug fixes are always welcome.

For larger changes or new features, please open an issue first to discuss it.

### Documentation

The crate uses `#![deny(missing_docs)]`. If you're unsure what to document, just let the compiler tell you.

Usually, the first line is short and describes what the item is.

Make sure to state the default value on a new paragraph when applicable.

### Pull requests

- Try to keep PRs focused on a single change
- Make sure the project builds with `cargo build`
- Make sure existing tests pass with `cargo test`
- Make sure code is formatted with `cargo fmt`
- Test your changes in the editor when applicable

### Catching system ordering bugs

Systems without explicit ordering may happen to run in the right order by accident. To shake those out, run the editor with randomized schedules:

```sh
cargo editor --features shuffle-schedules
```

The seed is printed on startup. Set `SPRINKLES_SHUFFLE_SEED` to reproduce a specific order.

## Adding new examples

Want to add an example? Go for it! Just please make sure it looks interesting and adds something different from the ones we already have.

The examples live in [`crates/bevy_sprinkles_editor/src/assets/examples/`](./crates/bevy_sprinkles_editor/src/assets/examples). Each example needs a `.ron` Sprinkles project and a `.jpg` thumbnail.

To add a new example:

1. Create or move your particle system to the examples directory
2. Take a 16:9 screenshot for the thumbnail (preferably 640x360)
3. Name both files with a matching kebab-case slug (ex: `acid-pool.ron` and `acid-pool.jpg`)
4. Use Title Case for the `name` field (ex: "Acid Pool")
5. Add an `authors` section to the `.ron` file
6. Add a row to [`crates/bevy_sprinkles_editor/src/assets/examples/README.md`](./crates/bevy_sprinkles_editor/src/assets/examples/README.md) following the existing format
