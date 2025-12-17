use std::path::PathBuf;

use iced::Task;
use iced::advanced::image;

use crate::app::{App, Message};
use crate::state::{
    MAX_CONCURRENT_THUMBNAILS, PlayerViewState, RenderStatus, ScanStatus, SelectionState,
};
use crate::video::VideoMetadata;

impl App {
    pub(crate) fn handle_scan_videos_directory(&mut self) -> Task<Message> {
        self.grid_state.scan_status = ScanStatus::Scanning;

        let videos_dir = self.videos_directory.clone();
        Task::perform(
            async move { crate::video::scan_videos_directory(videos_dir).await },
            Message::VideosScanned,
        )
    }

    pub(crate) fn handle_videos_scanned(
        &mut self,
        result: Result<Vec<VideoMetadata>, String>,
    ) -> Task<Message> {
        match result {
            Ok(metadata_list) => {
                use crate::ui::grid::{ThumbnailState, VideoItem};

                self.grid_state.videos = metadata_list
                    .into_iter()
                    .map(|metadata| VideoItem {
                        metadata,
                        thumbnail_state: ThumbnailState::Pending,
                        duration: None,
                    })
                    .collect();

                self.grid_state.thumbnail_generation_queue = self
                    .grid_state
                    .videos
                    .iter()
                    .map(|v| v.metadata.path.clone())
                    .rev()
                    .collect();

                self.grid_state.scan_status = ScanStatus::Complete;

                // Start multiple thumbnail generations in parallel
                let mut tasks = Vec::new();
                for _ in 0..MAX_CONCURRENT_THUMBNAILS {
                    tasks.push(Task::perform(
                        async { Message::GenerateNextThumbnail },
                        |msg| msg,
                    ));
                }
                Task::batch(tasks)
            }
            Err(error_msg) => {
                self.grid_state.scan_status = ScanStatus::Error(error_msg);
                Task::none()
            }
        }
    }

    pub(crate) fn handle_generate_next_thumbnail(&mut self) -> Task<Message> {
        // Check if we're already at max concurrency
        if self.grid_state.thumbnails_in_progress >= MAX_CONCURRENT_THUMBNAILS {
            return Task::none();
        }

        if let Some(video_path) = self.grid_state.thumbnail_generation_queue.pop() {
            use crate::ui::grid::ThumbnailState;

            if let Some(item) = self
                .grid_state
                .videos
                .iter_mut()
                .find(|v| v.metadata.path == video_path)
            {
                item.thumbnail_state = ThumbnailState::Generating;
            }

            self.grid_state.thumbnails_in_progress += 1;

            let path_for_thumbnail = video_path.clone();
            let path_for_info = video_path.clone();
            let path_for_message = video_path;
            Task::perform(
                async move {
                    let thumbnail_result = crate::video::extract_frame(path_for_thumbnail, 1.0)
                        .await
                        .map_err(|e| e.to_string());
                    let duration_result = crate::video::get_video_stream_info(&path_for_info)
                        .await
                        .map(|info| info.duration)
                        .map_err(|e| e.to_string());
                    (thumbnail_result, duration_result)
                },
                move |(thumbnail_result, duration_result)| Message::ThumbnailGenerated {
                    video_path: path_for_message.clone(),
                    thumbnail_result,
                    duration_result,
                },
            )
        } else {
            Task::none()
        }
    }

    pub(crate) fn handle_thumbnail_generated(
        &mut self,
        video_path: PathBuf,
        thumbnail_result: Result<Vec<u8>, String>,
        duration_result: Result<f64, String>,
    ) -> Task<Message> {
        use crate::ui::grid::ThumbnailState;

        if let Some(item) = self
            .grid_state
            .videos
            .iter_mut()
            .find(|v| v.metadata.path == video_path)
        {
            item.thumbnail_state = match thumbnail_result {
                Ok(bytes) => {
                    let width = (bytes.len() / 360 / 4) as u32; // Kinda hacky, but height is always 360
                    ThumbnailState::Ready(image::Handle::from_rgba(width, 360, bytes))
                }
                Err(error_msg) => ThumbnailState::Failed(error_msg),
            };
            item.duration = duration_result.ok();
        }

        self.grid_state.thumbnails_in_progress -= 1;

        // Start the next one if there are more in the queue
        if !self.grid_state.thumbnail_generation_queue.is_empty() {
            Task::perform(async { Message::GenerateNextThumbnail }, |msg| msg)
        } else {
            Task::none()
        }
    }

    pub(crate) fn handle_video_selected(&mut self, video_path: PathBuf) -> Task<Message> {
        use crate::app::Screen;

        let file_size_mb = std::fs::metadata(&video_path)
            .map(|m| m.len() as f64 / (1024.0 * 1024.0))
            .unwrap_or(0.0);

        // Initialize player state with default values, most of which are updated later
        self.player_state = Some(PlayerViewState {
            frame_handle: None,
            timestamp: 0.0,
            video_path: video_path.clone(),
            duration: 0.0,
            video_width: 0,
            video_height: 0,
            video_fps: 30.0,
            preview_width: 0.0,
            preview_height: 360.0,
            is_extracting: false,
            last_extraction_request_id: 0,
            pending_timestamp: None,
            crop_selection: None,
            selection_state: SelectionState::Idle,
            mouse_position: (0.0, 0.0),
            canvas_bounds: (1.0, 1.0),
            trim_start: 0.0,
            trim_end: 0.0,
            render_status: RenderStatus::Idle,
            is_command_hovered: false,
            target_size_mb: String::new(),
            video_file_size_mb: file_size_mb,
            is_streaming: false,
            audio_position: 0.0,
            has_audio_stream: false,
        });

        // Switch to editor screen
        self.current_screen = Screen::Editor;

        // Get video stream info (initial frame extraction is triggered after info is received)
        Task::perform(
            async move { crate::video::get_video_stream_info(&video_path).await },
            |result| Message::VideoStreamInfoReceived(result.map_err(|e| e.to_string())),
        )
    }
}
