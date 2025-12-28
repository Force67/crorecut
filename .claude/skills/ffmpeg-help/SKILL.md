---
name: ffmpeg-help
description: FFmpeg command help and documentation. Use for video encoding, frame extraction, audio processing, cropping, resizing, cutting, filtering, transcoding, seeking, or troubleshooting FFmpeg errors.
allowed-tools: Read, Bash(ffmpeg:*), Bash(ffprobe:*), WebFetch, WebSearch
---

# FFmpeg Help

## Primary Directive: Look Up Documentation

When answering FFmpeg questions, **always fetch from official documentation first** rather than relying on training data. FFmpeg evolves and docs may have changed.

### Documentation Sources

Look up these resources as needed:

- **Options, flags, general usage**: https://ffmpeg.org/ffmpeg.html
- **Video/audio filters** (`-vf`, `-af`): https://ffmpeg.org/ffmpeg-filters.html
- **Metadata extraction, stream info**: https://ffmpeg.org/ffprobe.html
- **Container formats, muxers/demuxers**: https://ffmpeg.org/ffmpeg-formats.html
- **Encoder/decoder options**: https://ffmpeg.org/ffmpeg-codecs.html
- **Time formats, color syntax, expressions**: https://ffmpeg.org/ffmpeg-utils.html

### Workflow

1. **Identify the topic** (filtering, encoding, seeking, etc.)
2. **Fetch relevant documentation** using web fetch tool
3. **Check `src/video/`** for existing project patterns
4. **Provide answer** with documentation-backed information

## Project Context

This project executes FFmpeg via `tokio::process::Command` (async subprocess). See `src/video/` for current integration patterns.

Key constraints:
- Commands must be non-blocking (spawned async)
- Stdout/stderr captured for error handling
- Raw output piped for frame streaming and audio decoding
- Always use `-nostdin` to prevent FFmpeg from reading stdin (can cause hangs in subprocess)

## Project-Specific Conventions

**Fast seeking**: Place `-ss` BEFORE `-i` for input seeking (skips to position). After `-i` causes output seeking (decodes and discards frames - much slower).

**Audio for cpal**: Raw PCM output: `-f s16le -c:a pcm_s16le -ar 48000 -ac 2 -`

**Frame streaming**: Raw RGB to stdout: `-f rawvideo -pix_fmt rgb24 -`

**Common automation flags**: `-y` (overwrite), `-n` (never overwrite), `-nostdin` (no stdin), `-hide_banner` (cleaner output)

**Progress parsing**: Use `-progress pipe:1` for machine-readable progress output. Parse `out_time_ms` for percentage calculation.

## Common FFmpeg Errors

When FFmpeg fails, stderr contains diagnostic info. Common issues:

- **`Invalid argument`**: Filter syntax error. Check `-vf`/`-af` for missing colons or wrong parameter names.
- **`Option X not found`**: Typo or unavailable option. Run `ffmpeg -h encoder=<name>` to see valid options.
- **`Unknown encoder/decoder`**: Codec not compiled in. Run `ffmpeg -codecs` to check availability.
- **`No such filter`**: Filter not available. Run `ffmpeg -filters` to list available filters.
- **`Stream map 'X' matches no streams`**: Wrong stream index. Use `ffprobe` to inspect actual stream indices.
- **`Avi does not support...`**: Container/codec mismatch. Use compatible codec or different container (mp4, mkv).
- **`height/width not divisible by 2`**: Odd dimensions with certain codecs. Add `scale=trunc(iw/2)*2:trunc(ih/2)*2`.
- **`Too many packets buffered`**: Async stream timing issues. Add `-max_muxing_queue_size 1024`.

**Debugging**: Run command manually to see full stderr, add `-v verbose` for detail, use `ffprobe -v error -show_streams <file>` to inspect input.

## Version and Capability Checking

Verify FFmpeg's build and available features: `ffmpeg -version`, `ffmpeg -codecs`, `ffmpeg -encoders`, `ffmpeg -decoders`, `ffmpeg -filters`, `ffmpeg -formats`, `ffmpeg -h encoder=<name>`, `ffmpeg -h filter=<name>`

When encountering codec/filter errors, verify availability with these commands before assuming a bug.

## Performance Tips

**Hardware acceleration** (`-hwaccel`):
- Use `-hwaccel auto` to let FFmpeg choose available acceleration
- Windows: DXVA2 (`-hwaccel dxva2`), D3D11VA (`-hwaccel d3d11va`), or CUDA if available
- For decoding only; encoding requires specific encoders (e.g., `h264_nvenc`, `h264_qsv`)
- Note: Hardware-decoded frames may need `-hwaccel_output_format` for further processing

**Threading** (`-threads`):
- FFmpeg auto-detects optimal thread count by default
- Use `-threads 0` for auto (recommended) or specify count for constrained environments
- Filter threading: `-filter_threads` controls filter graph parallelism

**Encoding speed vs quality** (`-preset`):
- For libx264/libx265: `ultrafast`, `superfast`, `veryfast`, `faster`, `fast`, `medium`, `slow`, `slower`, `veryslow`
- Faster presets = larger files or lower quality at same bitrate
- For quick previews: `-preset ultrafast`
- For final output: `-preset slow` or `medium`

**Memory considerations for this project:**
- Frame streaming uses significant memory; consider `-s` to scale down if needed
- Pipe buffers can fill up; ensure Rust side reads stdout promptly
