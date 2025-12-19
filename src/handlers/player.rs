use iced::Task;

use crate::app::{App, Message};
use crate::video::VideoStreamInfo;

impl App {
    pub(crate) fn handle_video_stream_info(
        &mut self,
        result: Result<VideoStreamInfo, String>,
    ) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            match result {
                Ok(info) => {
                    player_state.video_width = info.width;
                    player_state.video_height = info.height;
                    player_state.video_fps = info.fps;
                    player_state.duration = info.duration;
                    player_state.trim_end = info.duration;
                    player_state.has_audio_stream = info.has_audio;
                    player_state.audio_bitrate_bps = info.audio_bitrate_bps;

                    // Calculate preview dimensions (360p with maintained aspect ratio)
                    if info.height > 0 {
                        player_state.preview_width =
                            (info.width as f32 * 360.0) / info.height as f32;
                        player_state.preview_height = 360.0;
                    }

                    log::info!(
                        "Video: {}x{} @ {:.2}fps, {:.2}s, audio: {}",
                        info.width,
                        info.height,
                        info.fps,
                        info.duration,
                        if info.has_audio { "yes" } else { "no" }
                    );

                    // Now that dimensions are set, extract initial frame
                    let video_path = player_state.video_path.clone();
                    return Task::perform(
                        async move {
                            crate::video::extract_frame(video_path, 0.0)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        |result| Message::FrameExtracted {
                            request_id: 0,
                            result,
                        },
                    );
                }
                Err(error_msg) => {
                    log::error!("Failed to get video info: {error_msg}");
                }
            }
        }
        Task::none()
    }

    pub(crate) fn handle_frame_extracted(
        &mut self,
        request_id: u64,
        result: Result<Vec<u8>, String>,
    ) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            // Only accept frame from most recent extraction
            if request_id != player_state.last_extraction_request_id {
                log::debug!(
                    "Discarding stale frame (request_id {} != current {})",
                    request_id,
                    player_state.last_extraction_request_id
                );
                return Task::none();
            }

            match result {
                Ok(frame_bytes) => {
                    player_state.frame_handle = Some(iced::advanced::image::Handle::from_rgba(
                        player_state.preview_width as u32,
                        player_state.preview_height as u32,
                        frame_bytes,
                    ));
                }
                Err(error_msg) => {
                    log::error!("Frame extraction failed: {error_msg}");
                }
            }

            // Mark extraction as complete
            player_state.is_extracting = false;

            // If there's a pending timestamp, extract it now
            if let Some(pending_ts) = player_state.pending_timestamp {
                return self.handle_slider_changed(pending_ts);
            }
        }
        Task::none()
    }

    pub(crate) fn handle_slider_changed(&mut self, new_timestamp: f64) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            // Update timestamp immediately for responsive UI
            player_state.timestamp = new_timestamp;

            // If streaming, increment seek_generation to restart subscriptions from new position
            if player_state.is_streaming {
                player_state.seek_generation += 1;
                return Task::none();
            }

            if player_state.is_extracting {
                // Extraction in progress - store as pending
                player_state.pending_timestamp = Some(new_timestamp);
            } else {
                // No extraction in progress - spawn FFmpeg immediately
                player_state.is_extracting = true;
                player_state.last_extraction_request_id += 1;
                player_state.pending_timestamp = None;

                let video_path = player_state.video_path.clone();
                let request_id = player_state.last_extraction_request_id;

                return Task::perform(
                    async move {
                        crate::video::extract_frame(video_path, new_timestamp)
                            .await
                            .map_err(|e| e.to_string())
                    },
                    move |result| Message::FrameExtracted { request_id, result },
                );
            }
        }
        Task::none()
    }

    pub(crate) fn handle_trim_start_changed(&mut self, value: f64) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            player_state.trim_start = value.min(player_state.trim_end - 0.1).max(0.0);
            // Send `SliderChanged` so that user can instantly see the preview frame
            Task::done(Message::SliderChanged(player_state.trim_start))
        } else {
            Task::none()
        }
    }

    pub(crate) fn handle_trim_end_changed(&mut self, value: f64) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            player_state.trim_end = value
                .max(player_state.trim_start + 0.1)
                .min(player_state.duration);
            // Send `SliderChanged` so that user can instantly see the preview frame
            Task::done(Message::SliderChanged(player_state.trim_end))
        } else {
            Task::none()
        }
    }

    // Streaming handlers (experimental)

    pub(crate) fn handle_start_streaming(&mut self) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state
            && !player_state.is_streaming
        {
            log::info!(
                "Starting frame streaming at timestamp {}",
                player_state.timestamp
            );
            player_state.is_streaming = true;
        }
        Task::none()
    }

    pub(crate) fn handle_stop_streaming(&mut self) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state
            && player_state.is_streaming
        {
            log::info!("Stopping frame streaming");
            player_state.is_streaming = false;
        }
        Task::none()
    }

    pub(crate) fn handle_streamed_frame(
        &mut self,
        frame_bytes: Vec<u8>,
        video_timestamp: f64,
    ) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state
            && player_state.is_streaming
        {
            player_state.frame_handle = Some(iced::advanced::image::Handle::from_rgba(
                player_state.preview_width as u32,
                player_state.preview_height as u32,
                frame_bytes,
            ));
            player_state.timestamp = video_timestamp;
        }
        Task::none()
    }

    // Audio handlers

    pub(crate) fn handle_audio_position_update(&mut self, position: f64) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state
            && player_state.is_streaming
        {
            player_state.audio_position = position;
        }
        Task::none()
    }

    pub(crate) fn handle_audio_stream_ended(&mut self) -> Task<Message> {
        log::info!("Audio stream ended, stopping playback");
        self.handle_stop_streaming()
    }
}
