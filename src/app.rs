use iced::{Element, Task};
use std::path::PathBuf;

use crate::ui::grid::VideoItem;
use crate::video::VideoMetadata;

#[derive(Debug, Clone)]
pub enum ScanStatus {
    NotStarted,
    Scanning,
    Complete { count: usize },
    Error(String),
}

pub struct GridViewState {
    pub videos: Vec<VideoItem>,
    pub scan_status: ScanStatus,
    pub thumbnail_generation_queue: Vec<PathBuf>,
    pub thumbnails_in_progress: usize,
}

const MAX_CONCURRENT_THUMBNAILS: usize = 3;

pub struct PlayerViewState {
    pub frame_handle: Option<iced::advanced::image::Handle>,
    pub timestamp: f64,
    pub video_path: PathBuf,
    pub duration: f64,
}

pub enum AppView {
    Grid(GridViewState),
    Player(PlayerViewState),
}

pub struct App {
    current_view: AppView,
    videos_directory: PathBuf,
}

impl Default for App {
    fn default() -> Self {
        let videos_dir = dirs::video_dir().unwrap_or_else(|| PathBuf::from("."));

        Self {
            current_view: AppView::Grid(GridViewState {
                videos: Vec::new(),
                scan_status: ScanStatus::NotStarted,
                thumbnail_generation_queue: Vec::new(),
                thumbnails_in_progress: 0,
            }),
            videos_directory: videos_dir,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    // Grid View
    ScanVideosDirectory,
    VideosScanned(Result<Vec<VideoMetadata>, String>),
    GenerateNextThumbnail,
    ThumbnailGenerated {
        video_path: PathBuf,
        result: Result<Vec<u8>, String>,
    },

    // Player View (existing, for future use)
    LoadFrame,
    FrameExtracted(Result<Vec<u8>, String>),
    DurationReceived(Result<f64, String>),
    SliderChanged(f64),
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let app = Self::default();
        let initial_task = Task::perform(async { Message::ScanVideosDirectory }, |msg| msg);
        (app, initial_task)
    }

    pub fn title(&self) -> String {
        String::from("Crorecut")
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ScanVideosDirectory => {
                if let AppView::Grid(ref mut grid_state) = self.current_view {
                    grid_state.scan_status = ScanStatus::Scanning;

                    let videos_dir = self.videos_directory.clone();
                    return Task::perform(
                        async move { crate::video::scan_videos_directory(videos_dir).await },
                        Message::VideosScanned,
                    );
                }
                Task::none()
            }

            Message::VideosScanned(result) => {
                if let AppView::Grid(ref mut grid_state) = self.current_view {
                    match result {
                        Ok(metadata_list) => {
                            use crate::ui::grid::{ThumbnailState, VideoItem};

                            let count = metadata_list.len();

                            grid_state.videos = metadata_list
                                .into_iter()
                                .map(|metadata| VideoItem {
                                    metadata,
                                    thumbnail_state: ThumbnailState::Pending,
                                })
                                .collect();

                            grid_state.thumbnail_generation_queue = grid_state
                                .videos
                                .iter()
                                .map(|v| v.metadata.path.clone())
                                .collect();

                            grid_state.scan_status = ScanStatus::Complete { count };

                            // Start multiple thumbnail generations in parallel
                            let mut tasks = Vec::new();
                            for _ in 0..MAX_CONCURRENT_THUMBNAILS {
                                tasks.push(Task::perform(
                                    async { Message::GenerateNextThumbnail },
                                    |msg| msg,
                                ));
                            }
                            return Task::batch(tasks);
                        }
                        Err(error_msg) => {
                            grid_state.scan_status = ScanStatus::Error(error_msg);
                        }
                    }
                }
                Task::none()
            }

            Message::GenerateNextThumbnail => {
                if let AppView::Grid(ref mut grid_state) = self.current_view {
                    // Check if we're already at max concurrency
                    if grid_state.thumbnails_in_progress >= MAX_CONCURRENT_THUMBNAILS {
                        return Task::none();
                    }

                    if let Some(video_path) = grid_state.thumbnail_generation_queue.pop() {
                        use crate::ui::grid::ThumbnailState;

                        if let Some(item) = grid_state
                            .videos
                            .iter_mut()
                            .find(|v| v.metadata.path == video_path)
                        {
                            item.thumbnail_state = ThumbnailState::Generating;
                        }

                        grid_state.thumbnails_in_progress += 1;

                        let path_for_async = video_path.clone();
                        let path_for_message = video_path;
                        return Task::perform(
                            async move {
                                crate::video::extract_frame(path_for_async, 1.0)
                                    .await
                                    .map_err(|e| e.to_string())
                            },
                            move |result| Message::ThumbnailGenerated {
                                video_path: path_for_message.clone(),
                                result,
                            },
                        );
                    }
                }
                Task::none()
            }

            Message::ThumbnailGenerated { video_path, result } => {
                if let AppView::Grid(ref mut grid_state) = self.current_view {
                    use crate::ui::grid::ThumbnailState;

                    if let Some(item) = grid_state
                        .videos
                        .iter_mut()
                        .find(|v| v.metadata.path == video_path)
                    {
                        item.thumbnail_state = match result {
                            Ok(bytes) => ThumbnailState::Ready(
                                iced::advanced::image::Handle::from_bytes(bytes),
                            ),
                            Err(error_msg) => ThumbnailState::Failed(error_msg),
                        };
                    }

                    grid_state.thumbnails_in_progress -= 1;

                    // Start the next one if there are more in the queue
                    if !grid_state.thumbnail_generation_queue.is_empty() {
                        return Task::perform(async { Message::GenerateNextThumbnail }, |msg| msg);
                    }
                }
                Task::none()
            }

            // Player view messages (not implemented yet)
            Message::LoadFrame => Task::none(),
            Message::FrameExtracted(_) => Task::none(),
            Message::DurationReceived(_) => Task::none(),
            Message::SliderChanged(_) => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.current_view {
            AppView::Grid(grid_state) => {
                use crate::ui::grid::grid_view;
                use iced::Length;
                use iced::widget::{column, container, text};

                match &grid_state.scan_status {
                    ScanStatus::NotStarted | ScanStatus::Scanning => {
                        container(text("Scanning videos...").size(20))
                            .center_x(Length::Fill)
                            .center_y(Length::Fill)
                            .into()
                    }

                    ScanStatus::Complete { .. } => grid_view(&grid_state.videos),

                    ScanStatus::Error(error_msg) => container(
                        column![
                            text("Error scanning videos").size(20),
                            text(error_msg).size(14),
                        ]
                        .spacing(10),
                    )
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into(),
                }
            }

            AppView::Player(_player_state) => {
                use iced::widget::text;
                text("Player view (not implemented yet)").into()
            }
        }
    }
}
