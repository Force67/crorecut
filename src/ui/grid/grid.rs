use std::path::PathBuf;

use iced::Task;
use iced::advanced::image;
use iced::widget::scrollable;

use crate::video::VideoMetadata;

use super::video_item::{ThumbnailState, VideoItem};

pub const MAX_CONCURRENT_THUMBNAILS: usize = 3;

pub fn grid_scrollable_id() -> iced::widget::Id {
    iced::widget::Id::new("grid_scrollable")
}

#[derive(Debug, Clone)]
pub enum ScanStatus {
    NotStarted,
    Scanning,
    Complete,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Grid {
    pub videos_directory: PathBuf,
    pub videos: Vec<VideoItem>,
    pub scan_status: ScanStatus,
    pub thumbnail_generation_queue: Vec<PathBuf>,
    pub thumbnails_in_progress: usize,
    pub scroll_offset: scrollable::RelativeOffset,
}

#[derive(Debug, Clone)]
pub enum Message {
    ScanVideosDirectory,
    VideosScanned(Result<Vec<VideoMetadata>, String>),
    GenerateNextThumbnail,
    ThumbnailGenerated {
        video_path: PathBuf,
        thumbnail_result: Result<Vec<u8>, String>,
        duration_result: Result<f64, String>,
    },
    VideoSelected(PathBuf),
    ScrollChanged(scrollable::Viewport),
}

impl Grid {
    pub fn new(videos_directory: PathBuf) -> Self {
        Self {
            videos_directory,
            videos: Vec::new(),
            scan_status: ScanStatus::NotStarted,
            thumbnail_generation_queue: Vec::new(),
            thumbnails_in_progress: 0,
            scroll_offset: scrollable::RelativeOffset::default(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ScanVideosDirectory => {
                self.scan_status = ScanStatus::Scanning;

                let videos_dir = self.videos_directory.clone();
                Task::perform(
                    async move { crate::video::scan_videos_directory(videos_dir).await },
                    Message::VideosScanned,
                )
            }

            Message::VideosScanned(result) => match result {
                Ok(metadata_list) => {
                    self.videos = metadata_list
                        .into_iter()
                        .map(|metadata| VideoItem {
                            metadata,
                            thumbnail_state: ThumbnailState::Pending,
                            duration: None,
                        })
                        .collect();

                    self.thumbnail_generation_queue = self
                        .videos
                        .iter()
                        .map(|v| v.metadata.path.clone())
                        .rev()
                        .collect();

                    self.scan_status = ScanStatus::Complete;

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
                    self.scan_status = ScanStatus::Error(error_msg);
                    Task::none()
                }
            },

            Message::GenerateNextThumbnail => {
                // Check if we're already at max concurrency
                if self.thumbnails_in_progress >= MAX_CONCURRENT_THUMBNAILS {
                    return Task::none();
                }

                if let Some(video_path) = self.thumbnail_generation_queue.pop() {
                    if let Some(item) = self
                        .videos
                        .iter_mut()
                        .find(|v| v.metadata.path == video_path)
                    {
                        item.thumbnail_state = ThumbnailState::Generating;
                    }

                    self.thumbnails_in_progress += 1;

                    let path_for_thumbnail = video_path.clone();
                    let path_for_info = video_path.clone();
                    let path_for_message = video_path;
                    Task::perform(
                        async move {
                            let thumbnail_result =
                                crate::video::extract_frame(path_for_thumbnail, 1.0)
                                    .await
                                    .map_err(|e| e.to_string());
                            let duration_result =
                                crate::video::get_video_stream_info(&path_for_info)
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

            Message::ThumbnailGenerated {
                video_path,
                thumbnail_result,
                duration_result,
            } => {
                if let Some(item) = self
                    .videos
                    .iter_mut()
                    .find(|v| v.metadata.path == video_path)
                {
                    item.thumbnail_state = match thumbnail_result {
                        Ok(bytes) => {
                            let width = (bytes.len() / 360 / 4) as u32; // Height is always 360
                            ThumbnailState::Ready(image::Handle::from_rgba(width, 360, bytes))
                        }
                        Err(error_msg) => ThumbnailState::Failed(error_msg),
                    };
                    item.duration = duration_result.ok();
                }

                self.thumbnails_in_progress -= 1;

                // Start the next one if there are more in the queue
                if !self.thumbnail_generation_queue.is_empty() {
                    Task::perform(async { Message::GenerateNextThumbnail }, |msg| msg)
                } else {
                    Task::none()
                }
            }

            Message::VideoSelected(_path) => {
                // Navigation is handled at app level
                Task::none()
            }

            Message::ScrollChanged(viewport) => {
                self.scroll_offset = viewport.relative_offset();
                Task::none()
            }
        }
    }
}
