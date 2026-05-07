# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## Project Overview

Crorecut is a minimalistic video editing tool built with Rust and iced-rs, powered by FFmpeg.

**What "minimalistic" means:**
- Grid-based video library browser with thumbnail previews
- Basic editing operations: crop, resize, cut, volume adjustment
- Real-time video preview with frame-accurate seeking
- No complex timeline or multi-track editing
- Focus on speed, compactness and simplicity over feature completeness

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
    utils.rs         # Formatting helpers
    audio.rs         # Re-exports from audio/
    audio/           # Audio playback via cpal, A/V sync clock
    video.rs         # Re-exports from video/
    video/           # FFmpeg operations, metadata, frame streaming
    ui.rs            # Re-exports from ui/
    ui/
        color.rs     # Color constants
        grid.rs      # Grid screen state and messages
        grid/        # Grid screen components (view, video_item)
        editor.rs    # Editor screen state and messages
        editor/      # Editor screen components (view, canvas, etc.)
```

Modules follow the modern recommended Rust pattern: `module.rs` + `module/` instead of `module/mod.rs`.

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

**Key components:**
- `boot()`: Initialize application state
- `update()`: Handle messages and return new state + tasks
- `view()`: Render UI from current state
- `Task<Message>`: Async operations that produce messages

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

**Combining filters (example):**
- Video filters: `-vf "crop=640:480:0:0,scale=1280:720"`
- Audio filters: `-af "volume=1.5,aresample=44100"`

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

### Image Allocation Pattern

When displaying images that update frequently (video frames, animations), use `iced::widget::image::allocate()` to ensure GPU readiness before display. Store `Allocation` instead of `Handle` to prevent flickering.

### Subscriptions for Streaming

For long-running async streams (video frames, audio playback), use iced's `Subscription` system with custom `Recipe` implementations. See `src/video/frame_streaming.rs` and `src/audio/subscription.rs` for examples.

**Pattern:**
- Implement `iced::advanced::subscription::Recipe`
- Use channels to communicate between subscription and app
- Return subscription from `subscription()` method in App trait

## Code Organization Guidelines

- Keep `update()` clean: if a match arm in `update()` grows beyond ~10 lines, extract it into a helper method
- Command building: prefer `.args(["-flag", "value"])` over chained `.arg("-flag").arg("value")`
- Separate view from state: put state and `update()` in `foo.rs`, put `view()` and UI helpers in `foo/view.rs`
- Use `CREATE_NO_WINDOW` when spawning `Command`s: this is a Windows-specific flag to hide console windows

### Error Handling

This is a personal pet project, so **keep error handling pragmatic and minimal**:
- Use `Result` where it makes sense, but don't over-engineer
- `unwrap()` and `expect()` are fine when appropriate
- Focus on user experience over exhaustive error recovery

## Platform Notes

**Primary target: Windows**

While the project is technically cross-platform (thanks to iced and Rust), **Windows is the primary development and testing platform**. When making decisions or testing features, Windows compatibility takes priority.
