use std::path::PathBuf;

use iced::Task;
use iced::advanced::image;
use iced::widget::scrollable;

use crate::ui::grid::video_item::{ThumbnailState, VideoItem};
use crate::video::VideoMetadata;

mod video_item;
mod view;

const THUMBNAIL_BATCH_SIZE: usize = 6;

pub fn grid_scrollable_id() -> iced::widget::Id {
    iced::widget::Id::new("grid_scrollable")
}

#[derive(Debug, Clone)]
enum ScanStatus {
    NotStarted,
    Scanning,
    Complete,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Grid {
    videos_directory: PathBuf,
    videos: Vec<VideoItem>,
    scan_status: ScanStatus,
    thumbnail_generation_queue: Vec<PathBuf>,
    batch_in_progress: bool,
    scroll_offset: scrollable::RelativeOffset,
}

#[derive(Debug, Clone)]
pub enum Message {
    ScanVideosDirectory,
    VideosScanned(Result<Vec<VideoMetadata>, String>),
    GenerateNextBatch,
    BatchThumbnailsGenerated {
        video_paths: Vec<PathBuf>,
        thumbnail_results: Vec<Result<Vec<u8>, String>>,
        duration_results: Vec<Result<f64, String>>,
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
            batch_in_progress: false,
            scroll_offset: scrollable::RelativeOffset::default(),
        }
    }

    pub fn scroll_offset(&self) -> scrollable::RelativeOffset {
        self.scroll_offset
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

                    // Start batch thumbnail generation
                    Task::done(Message::GenerateNextBatch)
                }
                Err(error_msg) => {
                    self.scan_status = ScanStatus::Error(error_msg);
                    Task::none()
                }
            },

            Message::GenerateNextBatch => {
                // Check if we're already processing a batch
                if self.batch_in_progress {
                    return Task::none();
                }

                if self.thumbnail_generation_queue.is_empty() {
                    return Task::none();
                }

                // Pop up to THUMBNAIL_BATCH_SIZE paths from the queue
                let batch_size = self
                    .thumbnail_generation_queue
                    .len()
                    .min(THUMBNAIL_BATCH_SIZE);
                let batch_paths: Vec<PathBuf> = self
                    .thumbnail_generation_queue
                    .drain(self.thumbnail_generation_queue.len() - batch_size..)
                    .collect();

                // Mark these videos as generating
                for path in &batch_paths {
                    if let Some(item) = self.videos.iter_mut().find(|v| &v.metadata.path == path) {
                        item.thumbnail_state = ThumbnailState::Generating;
                    }
                }

                self.batch_in_progress = true;

                let paths_for_message = batch_paths.clone();
                Task::perform(
                    async move {
                        // Run frame extraction and duration fetching in parallel
                        let thumbnail_results =
                            crate::video::extract_first_frames_batch(batch_paths.clone()).await;
                        let duration_futures: Vec<_> = batch_paths
                            .iter()
                            .map(|p| crate::video::get_video_stream_info(p))
                            .collect();
                        let duration_results: Vec<_> =
                            iced::futures::future::join_all(duration_futures).await;

                        (
                            thumbnail_results
                                .into_iter()
                                .map(|r| r.map_err(|e| e.to_string()))
                                .collect::<Vec<_>>(),
                            duration_results
                                .into_iter()
                                .map(|r| r.map(|info| info.duration).map_err(|e| e.to_string()))
                                .collect::<Vec<_>>(),
                        )
                    },
                    move |(thumbnail_results, duration_results)| {
                        Message::BatchThumbnailsGenerated {
                            video_paths: paths_for_message,
                            thumbnail_results,
                            duration_results,
                        }
                    },
                )
            }

            Message::BatchThumbnailsGenerated {
                video_paths,
                thumbnail_results,
                duration_results,
            } => {
                for ((path, thumb_result), dur_result) in video_paths
                    .into_iter()
                    .zip(thumbnail_results)
                    .zip(duration_results)
                {
                    if let Some(item) = self.videos.iter_mut().find(|v| v.metadata.path == path) {
                        item.thumbnail_state = match thumb_result {
                            Ok(bytes) => ThumbnailState::Ready(image::Handle::from_rgba(
                                crate::video::THUMBNAIL_WIDTH,
                                crate::video::THUMBNAIL_HEIGHT,
                                bytes,
                            )),
                            Err(error_msg) => ThumbnailState::Failed(error_msg),
                        };
                        item.duration = dur_result.ok();
                    }
                }

                self.batch_in_progress = false;

                // Start the next batch if there are more in the queue
                if !self.thumbnail_generation_queue.is_empty() {
                    Task::done(Message::GenerateNextBatch)
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
