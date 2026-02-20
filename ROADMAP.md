# Roadmap

This is an unordered list of planned features. Nothing here is guaranteed to be added.

## Core

- 2D particle systems[^1]
- Add smoothstep interpolation for `GradientEdit`
- Auto instantiate/reuse materials
- Support deleting entity when particle system finishes
- Add `editor_only` checkbox for colliders (useful if already present in-game)

## Editor

### QoL

- <kbd>⌘</kbd> + <kbd>Z</kbd> (undo/redo)
- Reorder emitters and colliders via drag and drop
- Better open project error messages (too vague)
- In-editor docs

### Project selector / hamburger menu

I want to replace the project selector button, and have a hamburger icon instead with:

- _(all existing options)_
- About Sprinkles
- Check for updates
- Preferences
- Quit

And then have the project name as an item, on top of the emitters (needs design). Clicking on it will show the project settings in the inspector on the right, with the following options:

- Project name
- Project/assets folder location
- Authors
- Folder location + "Reveal in Finder" button

### Preferences (editor settings)

- Bloom
- Anti-aliasing
- Toggle FPS counter
- Check for updates automatically
- Grid / Floor / Skybox
- Tonemapper
- Light mode (maybe?)

## Testing

- Stress test example
- Regression tests with screenshots

## Docs

- Guide for using Sprinkles in-game

[^1]: Eventually.
