# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Crorecut is a minimalistic video editing tool built with Rust and iced-rs, powered by FFmpeg.

## Tech Stack

- **Language**: Rust
- **GUI Framework**: iced 0.15.0-dev (pinned via git in Cargo.toml)
  - **Enabled features**: `tokio`, `advanced`, `image`, `canvas`, `svg`
- **Video Processing**: FFmpeg (via subprocess) for frame extraction, transcoding, and real-time frame streaming
- **Audio Playback**: cpal for cross-platform audio output, also via FFmpeg subprocess
- **Async Runtime**: tokio for subprocess execution and async operations
- **Configuration**: TOML with serde serialization
- **Other**: `image` (frame handling), `open` (file/folder opening), `dirs` (platform paths)

## Development Commands

```bash
# Check code without building
cargo check

# Build the project
cargo build

# Run the application
cargo run

# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Directory Structure

```
src/
    app.rs           # App struct, Screen enum, nested Message enum
    main.rs          # Entry point
    icons.rs         # SVG icon constants
    audio/           # Audio playback via cpal, A/V sync clock
    config/          # Settings persistence (TOML)
    utils/           # Formatting helpers
    video/           # FFmpeg operations, metadata, frame streaming
    ui/
        mod.rs       # Module declarations and re-exports
        grid/        # Grid screen (video browser)
        editor/      # Editor screen (video editing)
        settings/    # Settings screen
        about/       # About screen
```

Each screen module (e.g., `ui/grid/`) follows this structure:
- `mod.rs` - Module declarations and public re-exports
- `grid.rs` (the screen name) - `Grid` state struct and `Message` enum
- `view.rs` - `impl Grid { fn view() }` implementation
- Additional files for screen-specific types (e.g., `video_item.rs`)

## iced Documentation & Source Code Lookup

**IMPORTANT**: This project uses `iced` from the HEAD of the GitHub repository, not a stable release.

### Documentation Preferences

When looking up iced documentation, **ALWAYS** prefer:
- **In-development documentation**: `https://docs.iced.rs/iced` (NOT `https://docs.rs/iced`)
- **All items reference page (list of all functions, structs, macros, etc.)**: `https://docs.iced.rs/iced/all.html` (useful for finding specific items by name)

The docs.iced.rs site reflects the latest development version, while docs.rs only has stable releases.

### Source Code Lookup

When looking up source code, **ALWAYS prefer raw GitHub URLs**:
- **Correct**: `https://raw.githubusercontent.com/iced-rs/iced/refs/heads/master/core/src/widget.rs`
- **Avoid**: `https://github.com/iced-rs/iced/blob/master/core/src/widget.rs`

Raw GitHub URLs provide direct access to source code without HTML wrappers, making them faster and more reliable for programmatic access.

More examples:
- https://raw.githubusercontent.com/iced-rs/iced/refs/heads/master/examples/gallery/src/main.rs
- https://raw.githubusercontent.com/iced-rs/iced/refs/heads/master/widget/src/button.rs
- https://raw.githubusercontent.com/iced-rs/iced_aw/refs/heads/main/src/widget/color_picker.rs

### Repository Exploration Workflow

When exploring unfamiliar repositories, follow this two-step process:

1. **Discover structure with GitHub API** - List files and directories:
   - `https://api.github.com/repos/iced-rs/iced/contents/examples`
   - `https://api.github.com/repos/iced-rs/iced_aw/contents/src`

2. **Fetch file content with raw URLs** - Once you know what file you need:
   - `https://raw.githubusercontent.com/iced-rs/iced/refs/heads/master/examples/game_of_life/src/main.rs`

The API returns JSON with file/directory metadata (name, type, size). Use it to navigate, then switch to raw URLs for actual source code.

## Architecture Patterns

### iced Application Structure

This project uses iced's Elm-inspired architecture with `update()`, `view()`, and `Task<Message>` for async operations. Entry point uses `iced::application(boot, update, view).run()`.

### Multi-Screen Architecture

Each screen is a self-contained module under `src/ui/` with its own state, message enum, and update/view methods. The app routes messages to screens using nested enums and `.map()`.

### FFmpeg Integration Pattern

FFmpeg operations are executed via tokio subprocess (`tokio::process::Command`):

**Key principles:**
- Execute asynchronously using iced's `Task` API to avoid blocking the UI
- Wrap FFmpeg calls in async functions within the `video` module
- Return `Result<T, VideoError>` for proper error handling
- Use custom error types for user-friendly error messages

**Performance tip for frame extraction:**
- Place `-ss` (seek) flag **before** `-i` (input) for fast keyframe seeking
- Example: `ffmpeg -ss 1.0 -i video.mp4 -frames:v 1 output.png`

**Common FFmpeg operations:**
- **Crop**: `-vf crop=w:h:x:y`
- **Resize**: `-vf scale=w:h`
- **Cut**: `-ss start_time -to end_time`
- **Volume**: `-af volume=<factor>`
- **Mute**: `-an` (remove audio stream)

### Audio Playback

Audio is decoded via FFmpeg subprocess (raw PCM output) and played through `cpal`. `AudioClock` provides A/V synchronization.

### UI Component Organization

- **Use iced's canvas** for custom 2D graphics and overlays (requires `canvas` feature)
- **Prefer `mouse_area`** for interactive regions that need custom cursor or event handling

## Code Organization Guidelines

### Module Structure

**`mod.rs` files should only contain:**
- Module declarations (`mod foo;`)
- Re-exports (`pub use foo::Bar;`)
- Module-level documentation

**Keep actual implementation in named files** (e.g., `selection_canvas.rs`, `ffmpeg.rs`), not in `mod.rs`.

### Error Handling

This is a personal pet project, so **keep error handling pragmatic and minimal**:
- Use `Result` where it makes sense, but don't over-engineer
- `unwrap()` and `expect()` are fine when appropriate
- Focus on user experience over exhaustive error recovery

## Platform Notes

**Primary target: Windows**

While the project is technically cross-platform (thanks to iced and Rust), **Windows is the primary development and testing platform**. When making decisions or testing features, Windows compatibility takes priority.
