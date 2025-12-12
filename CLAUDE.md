# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Crorecut is a minimalistic video editing tool built with Rust and iced-rs, powered by FFmpeg.

## Tech Stack

- **Language**: Rust
- **GUI Framework**: iced 0.14.0-dev (from "master" branch on GitHub)
- **Video Processing**: FFmpeg (via command-line subprocess execution)
- **Async Runtime**: tokio for subprocess execution and async operations
- **Configuration**: TOML with serde serialization

## Development Commands

```bash
# Build the project
cargo build

# Check code without building
cargo check

# Run the application
cargo run

# Run in release mode
cargo run --release

# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy
```

## iced Documentation & Source Code Lookup

**IMPORTANT**: This project uses `iced` version `0.14.0-dev` from the HEAD of the GitHub repository, not a stable release.

### Documentation Preferences

When looking up iced documentation, **ALWAYS** prefer:
- **Documentation**: `https://docs.iced.rs/iced` (NOT `https://docs.rs/iced`)
- **Search**: `https://docs.iced.rs/iced/?search=<query>`

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

The application uses a `Screen` enum to manage different views (Home, Editor, Settings, etc.). The `view()` method routes to screen-specific view functions based on `current_screen`.

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
- **Compress**: `-crf <value>` with codec settings
- **Volume**: `-af volume=<factor>`
- **Mute**: `-an` (remove audio stream)

### UI Component Organization

- **Extract complex UI components** to separate files in `src/ui/` (e.g., `selection_canvas.rs`)
- **Use iced's canvas** for custom 2D graphics and overlays (requires `canvas` feature)
- **Prefer `mouse_area`** for interactive regions that need custom cursor or event handling

### Async Operations Pattern

When spawning async work from the UI:
1. Create a `Task::perform()` with an async block
2. Return a `Message` variant when the operation completes
3. Handle the result in the `update()` function
4. Update state and trigger re-render

### Settings Management Pattern

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
