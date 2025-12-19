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
    app.rs           # App struct, Message enum, update dispatcher
    main.rs          # Entry point
    icons.rs         # SVG icon constants
    audio/           # Audio playback via cpal, A/V sync clock
    config/          # Settings persistence (TOML)
    handlers/        # Message handlers
    state/           # State types
    ui/              # View components
    utils/           # Formatting helpers
    video/           # FFmpeg operations, metadata, frame streaming
```

## iced Documentation & Source Code Lookup

**IMPORTANT**: This project uses `iced` from the HEAD of the GitHub repository, not a stable release.

### Documentation Preferences

When looking up iced documentation, **ALWAYS** prefer:
- **In-development documentation**: `https://docs.iced.rs/iced` (NOT `https://docs.rs/iced`)
- **All items reference page (list of all functions, structs, macros, etc.)**: `https://docs.iced.rs/iced/all.html` (useful for finding specific items by name)

The docs.iced.rs site reflects the latest development version, while docs.rs only has stable releases.

### Source Code Lookup Preferences

When looking up iced source code, **ALWAYS prefer raw GitHub URLs**:
- **Correct**: `https://raw.githubusercontent.com/iced-rs/iced/refs/heads/master/widget/src/mouse_area.rs`
- **Avoid**: `https://github.com/iced-rs/iced/blob/master/widget/src/mouse_area.rs`

Raw GitHub URLs provide direct access to source code without HTML wrappers, making them faster and more reliable for programmatic access.

## Architecture Patterns

### iced Application Structure

This project uses iced's Elm-inspired architecture:

- **State struct** (`App`): Holds application state
- **Message enum**: Defines all events that can update state
- **update()**: Pure function that handles messages and returns `Task<Message>` for async operations
- **view()**: Pure function that renders UI based on current state, returns `Element<'_, Message>`
- **Entry point**: Uses `iced::application(boot, update, view).run()` builder pattern

The codebase follows a clean separation of concerns:

**`src/app.rs`** - Core application scaffolding
- Defines the `App` struct and `Message` enum
- Contains the `update()` method that **delegates** to handler methods
- Minimal business logic - acts as a dispatcher
- Re-exports state types for convenience

**`src/state/`** - Application state modules
- Each state module contains related types and default implementations

**`src/handlers/`** - Message handling logic
- Handler methods are implemented as `impl App { pub(crate) fn handle_*(...) -> Task<Message> }`
- Create new handler files when adding a distinct feature area (not just for every new handler)

### Multi-Screen Architecture

The application uses a `Screen` enum to manage different views. The `view()` method routes to screen-specific view functions based on `current_screen`.

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
- **Cut**: `-ss start_time -to end_time`
- **Volume**: `-af volume=<factor>`
- **Mute**: `-an` (remove audio stream)

### Audio Playback

Audio is decoded via FFmpeg subprocess (raw PCM output) and played through `cpal`. `AudioClock` provides A/V synchronization.

### UI Component Organization

- **Extract complex UI components** to separate files in `src/ui/` (e.g., `selection_canvas.rs`)
- **Use iced's canvas** for custom 2D graphics and overlays (requires `canvas` feature)
- **Prefer `mouse_area`** for interactive regions that need custom cursor or event handling

### Settings Management Pattern

(This is not yet implemented!)

Settings should:
- Persist between sessions using TOML format with `serde`
- Store config file using `dirs` crate for cross-platform paths
- Include: FFmpeg binary path, default working directory, etc.

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
