use iced::Task;

use crate::app::{App, Message};

impl App {
    pub(crate) fn handle_duration_received(
        &mut self,
        result: Result<f64, String>,
    ) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            match result {
                Ok(duration) => {
                    player_state.duration = duration;
                    player_state.trim_end = duration;
                }
                Err(error_msg) => {
                    log::error!("Failed to load video duration: {error_msg}");
                }
            }
        }
        Task::none()
    }

    pub(crate) fn handle_dimensions_received(
        &mut self,
        result: Result<(u32, u32), String>,
    ) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            match result {
                Ok((width, height)) => {
                    player_state.video_width = width;
                    player_state.video_height = height;

                    // Calculate preview dimensions (360p with maintained aspect ratio)
                    if height > 0 {
                        player_state.preview_width = (width as f32 * 360.0) / height as f32;
                        player_state.preview_height = 360.0;
                    }
                }
                Err(error_msg) => {
                    log::error!("Failed to load video dimensions: {error_msg}");
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
        timestamp: f64,
    ) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state
            && player_state.is_streaming
        {
            player_state.frame_handle = Some(iced::advanced::image::Handle::from_rgba(
                player_state.preview_width as u32,
                player_state.preview_height as u32,
                frame_bytes,
            ));
            player_state.timestamp = timestamp;
        }
        Task::none()
    }
}
