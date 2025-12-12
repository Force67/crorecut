use std::path::PathBuf;

use iced::Task;

use crate::app::{App, Message, RenderStatus};

impl App {
    pub(crate) fn handle_copy_command(&self, cmd: String) -> Task<Message> {
        iced::clipboard::write(cmd)
    }

    pub(crate) fn handle_run_command(&mut self) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            player_state.render_status = RenderStatus::Rendering;

            let video_path = player_state.video_path.clone();
            let crop_selection = player_state.crop_selection;
            let trim_start = player_state.trim_start;
            let trim_end = player_state.trim_end;
            let duration = player_state.duration;
            let video_width = player_state.video_width;
            let video_height = player_state.video_height;
            let preview_width = player_state.preview_width;
            let preview_height = player_state.preview_height;
            let target_size_str = player_state.target_size_mb.clone();

            // Check if compression is enabled
            let target_size: Option<f64> = target_size_str
                .trim()
                .parse()
                .ok()
                .filter(|&s: &f64| s > 0.0);

            if let Some(target_size_mb) = target_size {
                // Two-pass mode with compression
                Task::perform(
                    async move {
                        // Get audio bitrate
                        let audio_bitrate = crate::video::get_audio_bitrate(video_path.clone())
                            .await
                            .map_err(|e| e.to_string())?;

                        let dimen = crate::video::VideoDimensions {
                            video_width,
                            video_height,
                            preview_width,
                            preview_height,
                        };

                        let params = crate::video::CompressionParams {
                            target_size_mb,
                            duration_sec: duration,
                            audio_bitrate_bps: audio_bitrate,
                        };

                        let cmd = crate::video::build_compression_command(
                            &video_path,
                            &params,
                            crop_selection.as_ref(),
                            trim_start,
                            trim_end,
                            &dimen,
                        )
                        .map_err(|e| e.to_string())?;

                        let working_dir = video_path
                            .parent()
                            .map(|p| p.to_path_buf())
                            .unwrap_or_else(|| PathBuf::from("."));

                        crate::video::run_two_pass_compression(
                            cmd.pass1_args,
                            cmd.pass2_args,
                            working_dir,
                        )
                        .await
                        .map_err(|e| e.to_string())?;

                        Ok(cmd.output_path)
                    },
                    Message::CommandCompleted,
                )
            } else {
                // Single-pass mode (no compression)
                let dimen = crate::video::VideoDimensions {
                    video_width,
                    video_height,
                    preview_width,
                    preview_height,
                };

                let ffmpeg_cmd = crate::video::build_ffmpeg_command(
                    &video_path,
                    crop_selection.as_ref(),
                    trim_start,
                    trim_end,
                    duration,
                    &dimen,
                );

                let args = ffmpeg_cmd.args;
                let output_path = ffmpeg_cmd.output_path;

                Task::perform(
                    async move {
                        crate::video::run_ffmpeg_command(&args).await?;
                        Ok(output_path)
                    },
                    |result: Result<PathBuf, crate::video::error::FfmpegCommandError>| {
                        Message::CommandCompleted(result.map_err(|e| e.to_string()))
                    },
                )
            }
        } else {
            Task::none()
        }
    }

    pub(crate) fn handle_command_completed(
        &mut self,
        result: Result<PathBuf, String>,
    ) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            match result {
                Ok(output_path) => {
                    log::info!("Command completed successfully");
                    player_state.render_status = RenderStatus::Completed(output_path);
                }
                Err(ref error_msg) => {
                    log::error!("Command failed: {error_msg}");
                    player_state.render_status = RenderStatus::Failed(error_msg.clone());
                }
            }
        }
        Task::none()
    }

    pub(crate) fn handle_open_file(&self, path: PathBuf) -> Task<Message> {
        if let Err(e) = open::that_detached(&path) {
            log::error!("Failed to open file: {e}");
        }
        Task::none()
    }

    pub(crate) fn handle_delete_file(&mut self, path: PathBuf) -> Task<Message> {
        match std::fs::remove_file(&path) {
            Ok(()) => {
                log::info!("Deleted file: {}", path.display());
                if let Some(ref mut player_state) = self.player_state {
                    player_state.render_status = RenderStatus::Idle;
                }
            }
            Err(e) => {
                log::error!("Failed to delete file: {e}");
            }
        }
        Task::none()
    }

    pub(crate) fn handle_open_parent_dir(&self, filepath: PathBuf) -> Task<Message> {
        #[cfg(windows)]
        {
            if let Err(e) = std::process::Command::new("explorer.exe")
                .arg("/select,")
                .arg(&filepath)
                .spawn()
            {
                log::error!("Failed to open Explorer with file selected: {e}");
            }
        }
        #[cfg(not(windows))]
        {
            if let Some(parent) = filepath.parent() {
                if let Err(e) = open::that_detached(parent) {
                    log::error!("Failed to open parent directory: {e}");
                }
            } else {
                log::warn!("Path has no parent directory: {}", filepath.display());
            }
        }
        Task::none()
    }

    pub(crate) fn handle_command_preview_hovered(&mut self, is_hovered: bool) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            player_state.is_command_hovered = is_hovered;
        }
        Task::none()
    }

    pub(crate) fn handle_target_size_changed(&mut self, value: String) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            player_state.target_size_mb = value;
        }
        Task::none()
    }
}
