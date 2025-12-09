use iced::{Element, Task};
use std::path::PathBuf;

use crate::ui::editor::{editor_view, get_resize_handle, point_in_selection, resize_selection};
use crate::ui::grid::VideoItem;
use crate::ui::selection_canvas::CropSelection;
use crate::video::VideoMetadata;

#[derive(Debug, Clone)]
pub enum ScanStatus {
    NotStarted,
    Scanning,
    Complete { count: usize },
    Error(String),
}

#[derive(Debug, Clone)]
pub struct GridViewState {
    pub videos: Vec<VideoItem>,
    pub scan_status: ScanStatus,
    pub thumbnail_generation_queue: Vec<PathBuf>,
    pub thumbnails_in_progress: usize,
}

const MAX_CONCURRENT_THUMBNAILS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionState {
    Idle,
    Creating {
        start_x: f32,
        start_y: f32,
    },
    Moving {
        offset_x: f32,
        offset_y: f32,
    },
    Resizing {
        handle: ResizeHandle,
        original: CropSelection,
    },
}

#[derive(Debug, Clone)]
pub struct PlayerViewState {
    pub frame_handle: Option<iced::advanced::image::Handle>,
    pub timestamp: f64,
    pub video_path: PathBuf,
    pub duration: f64,

    // Throttling state
    pub last_extraction_time: std::time::Instant,
    pub last_extraction_request_id: u64,
    pub pending_timestamp: Option<f64>,

    // Crop selection state
    pub crop_selection: Option<CropSelection>,
    pub selection_state: SelectionState,
    pub mouse_position: (f32, f32),

    // Trim state
    pub trim_start: f64,
    pub trim_end: f64,
}

#[derive(Debug, Clone)]
pub enum Screen {
    Home,
    Editor,
    Settings,
    About,
}

pub struct App {
    current_screen: Screen,
    videos_directory: PathBuf,
    grid_state: GridViewState,
    player_state: Option<PlayerViewState>,
}

impl Default for App {
    fn default() -> Self {
        let videos_dir = dirs::video_dir().unwrap_or_else(|| PathBuf::from("."));

        Self {
            current_screen: Screen::Home,
            videos_directory: videos_dir,
            grid_state: GridViewState {
                videos: Vec::new(),
                scan_status: ScanStatus::NotStarted,
                thumbnail_generation_queue: Vec::new(),
                thumbnails_in_progress: 0,
            },
            player_state: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    NavigateTo(Screen),

    // Grid View
    ScanVideosDirectory,
    VideosScanned(Result<Vec<VideoMetadata>, String>),
    GenerateNextThumbnail,
    ThumbnailGenerated {
        video_path: PathBuf,
        result: Result<Vec<u8>, String>,
    },
    VideoSelected(PathBuf),

    // Player View (existing, for future use)
    LoadFrame,
    FrameExtracted {
        request_id: u64,
        result: Result<Vec<u8>, String>,
    },
    DurationReceived(Result<f64, String>),
    SliderChanged(f64),
    ExtractPendingFrame,

    // Crop selection
    CropMousePressed,
    CropMouseMoved {
        x: f32,
        y: f32,
    },
    CropMouseReleased,

    // Trim selection
    TrimStartChanged(f64),
    TrimEndChanged(f64),

    // Command display
    CopyCommand(String),
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
            Message::NavigateTo(screen) => {
                self.current_screen = screen;
                Task::none()
            }

            Message::ScanVideosDirectory => {
                self.grid_state.scan_status = ScanStatus::Scanning;

                let videos_dir = self.videos_directory.clone();
                Task::perform(
                    async move { crate::video::scan_videos_directory(videos_dir).await },
                    Message::VideosScanned,
                )
            }

            Message::VideosScanned(result) => {
                match result {
                    Ok(metadata_list) => {
                        use crate::ui::grid::{ThumbnailState, VideoItem};

                        let count = metadata_list.len();

                        self.grid_state.videos = metadata_list
                            .into_iter()
                            .map(|metadata| VideoItem {
                                metadata,
                                thumbnail_state: ThumbnailState::Pending,
                            })
                            .collect();

                        self.grid_state.thumbnail_generation_queue = self
                            .grid_state
                            .videos
                            .iter()
                            .map(|v| v.metadata.path.clone())
                            .rev()
                            .collect();

                        self.grid_state.scan_status = ScanStatus::Complete { count };

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

            Message::GenerateNextThumbnail => {
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

                    let path_for_async = video_path.clone();
                    let path_for_message = video_path;
                    Task::perform(
                        async move {
                            crate::video::extract_frame(path_for_async, 1.0)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        move |result| Message::ThumbnailGenerated {
                            video_path: path_for_message.clone(),
                            result,
                        },
                    )
                } else {
                    Task::none()
                }
            }

            Message::ThumbnailGenerated { video_path, result } => {
                use crate::ui::grid::ThumbnailState;

                if let Some(item) = self
                    .grid_state
                    .videos
                    .iter_mut()
                    .find(|v| v.metadata.path == video_path)
                {
                    item.thumbnail_state = match result {
                        Ok(bytes) => {
                            ThumbnailState::Ready(iced::advanced::image::Handle::from_bytes(bytes))
                        }
                        Err(error_msg) => ThumbnailState::Failed(error_msg),
                    };
                }

                self.grid_state.thumbnails_in_progress -= 1;

                // Start the next one if there are more in the queue
                if !self.grid_state.thumbnail_generation_queue.is_empty() {
                    Task::perform(async { Message::GenerateNextThumbnail }, |msg| msg)
                } else {
                    Task::none()
                }
            }

            Message::VideoSelected(video_path) => self.handle_video_selected(video_path),

            // Player view messages
            Message::DurationReceived(result) => {
                if let Some(ref mut player_state) = self.player_state {
                    match result {
                        Ok(duration) => {
                            player_state.duration = duration;
                            // Initialize trim_end to full duration
                            player_state.trim_end = duration;
                        }
                        Err(error_msg) => {
                            log::error!("Failed to load video duration: {}", error_msg);
                        }
                    }
                }
                Task::none()
            }

            Message::FrameExtracted { request_id, result } => {
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
                            player_state.frame_handle =
                                Some(iced::advanced::image::Handle::from_bytes(frame_bytes));
                        }
                        Err(error_msg) => {
                            log::error!("Frame extraction failed: {}", error_msg);
                            // Keep existing frame visible on error
                        }
                    }
                }
                Task::none()
            }

            Message::SliderChanged(new_timestamp) => self.handle_slider_changed(new_timestamp),

            Message::ExtractPendingFrame => self.handle_extract_pending_frame(),

            Message::LoadFrame => {
                if let Some(ref mut player_state) = self.player_state {
                    player_state.last_extraction_request_id += 1;
                    player_state.last_extraction_time = std::time::Instant::now();

                    let video_path = player_state.video_path.clone();
                    let timestamp = player_state.timestamp;
                    let request_id = player_state.last_extraction_request_id;

                    return Task::perform(
                        async move {
                            crate::video::extract_frame(video_path, timestamp)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        move |result| Message::FrameExtracted { request_id, result },
                    );
                }
                Task::none()
            }

            // Crop selection handlers
            Message::CropMousePressed => {
                if let Some(ref mut player_state) = self.player_state {
                    let (x, y) = player_state.mouse_position;

                    // Check if clicking on an existing selection
                    if let Some(selection) = player_state.crop_selection {
                        // Check if clicking on a resize handle
                        if let Some(handle) = get_resize_handle(x, y, &selection) {
                            player_state.selection_state = SelectionState::Resizing {
                                handle,
                                original: selection,
                            };
                        }
                        // Check if clicking inside selection (for moving)
                        else if point_in_selection(x, y, &selection) {
                            player_state.selection_state = SelectionState::Moving {
                                offset_x: x - selection.x,
                                offset_y: y - selection.y,
                            };
                        }
                        // Clicking outside - start new selection
                        else {
                            player_state.crop_selection = None;
                            player_state.selection_state = SelectionState::Creating {
                                start_x: x,
                                start_y: y,
                            };
                        }
                    } else {
                        // No existing selection - start creating new one
                        player_state.selection_state = SelectionState::Creating {
                            start_x: x,
                            start_y: y,
                        };
                    }
                }
                Task::none()
            }

            Message::CropMouseMoved { x, y } => {
                if let Some(ref mut player_state) = self.player_state {
                    // Update mouse position
                    player_state.mouse_position = (x, y);

                    match player_state.selection_state {
                        SelectionState::Creating { start_x, start_y } => {
                            // Update selection rectangle
                            let min_x = start_x.min(x);
                            let min_y = start_y.min(y);
                            let max_x = start_x.max(x);
                            let max_y = start_y.max(y);

                            player_state.crop_selection = Some(CropSelection {
                                x: min_x,
                                y: min_y,
                                width: max_x - min_x,
                                height: max_y - min_y,
                            });
                        }
                        SelectionState::Moving { offset_x, offset_y } => {
                            if let Some(ref mut selection) = player_state.crop_selection {
                                selection.x = x - offset_x;
                                selection.y = y - offset_y;
                            }
                        }
                        SelectionState::Resizing { handle, original } => {
                            if let Some(ref mut selection) = player_state.crop_selection {
                                resize_selection(selection, handle, x, y, &original);
                            }
                        }
                        SelectionState::Idle => {}
                    }
                }
                Task::none()
            }

            Message::CropMouseReleased => {
                if let Some(ref mut player_state) = self.player_state {
                    player_state.selection_state = SelectionState::Idle;
                }
                Task::none()
            }

            Message::TrimStartChanged(value) => {
                if let Some(ref mut player_state) = self.player_state {
                    // Clamp to not exceed trim_end (keep at least 0.1s gap)
                    player_state.trim_start = value.min(player_state.trim_end - 0.1).max(0.0);
                }
                Task::none()
            }

            Message::TrimEndChanged(value) => {
                if let Some(ref mut player_state) = self.player_state {
                    // Clamp to not go below trim_start (keep at least 0.1s gap)
                    player_state.trim_end = value
                        .max(player_state.trim_start + 0.1)
                        .min(player_state.duration);
                }
                Task::none()
            }

            Message::CopyCommand(cmd) => iced::clipboard::write(cmd),
        }
    }

    /// Handle video selection - initialize player state and load video
    fn handle_video_selected(&mut self, video_path: PathBuf) -> Task<Message> {
        // Initialize player state
        self.player_state = Some(PlayerViewState {
            frame_handle: None,
            timestamp: 0.0,
            video_path: video_path.clone(),
            duration: 0.0,
            last_extraction_time: std::time::Instant::now(),
            last_extraction_request_id: 0,
            pending_timestamp: None,
            crop_selection: None,
            selection_state: SelectionState::Idle,
            mouse_position: (0.0, 0.0),
            trim_start: 0.0,
            trim_end: 0.0, // Will be set when duration is received
        });

        // Switch to editor screen
        self.current_screen = Screen::Editor;

        // Get video duration and load initial frame
        let path_for_duration = video_path.clone();
        let path_for_frame = video_path;

        Task::batch(vec![
            Task::perform(
                async move { crate::video::get_video_duration_sec(path_for_duration).await },
                |result| Message::DurationReceived(result.map_err(|e| e.to_string())),
            ),
            Task::perform(
                async move {
                    crate::video::extract_frame(path_for_frame, 0.0)
                        .await
                        .map_err(|e| e.to_string())
                },
                |result| Message::FrameExtracted {
                    request_id: 0, // Initial load uses ID 0
                    result,
                },
            ),
        ])
    }

    /// Handle slider changes with throttling to avoid excessive frame extractions
    fn handle_slider_changed(&mut self, new_timestamp: f64) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            // Update timestamp immediately for responsive UI
            player_state.timestamp = new_timestamp;

            let elapsed = player_state.last_extraction_time.elapsed();

            if elapsed >= std::time::Duration::from_millis(100) {
                // Throttle window passed - spawn extraction
                player_state.last_extraction_request_id += 1;
                player_state.last_extraction_time = std::time::Instant::now();
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
            } else {
                // Within throttle window - store as pending
                player_state.pending_timestamp = Some(new_timestamp);

                // Schedule extraction after remaining throttle time
                let remaining_ms = 100 - elapsed.as_millis() as u64;
                return Task::perform(
                    async move {
                        tokio::time::sleep(std::time::Duration::from_millis(remaining_ms)).await;
                        Message::ExtractPendingFrame
                    },
                    |msg| msg,
                );
            }
        }
        Task::none()
    }

    /// Extract a pending frame after throttle delay
    fn handle_extract_pending_frame(&mut self) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            // Check if there's still a pending timestamp to extract
            if let Some(pending_ts) = player_state.pending_timestamp {
                let elapsed = player_state.last_extraction_time.elapsed();

                // Verify throttle window passed (defensive against overlapping schedules)
                if elapsed >= std::time::Duration::from_millis(100) {
                    player_state.last_extraction_request_id += 1;
                    player_state.last_extraction_time = std::time::Instant::now();
                    player_state.pending_timestamp = None;

                    let video_path = player_state.video_path.clone();
                    let request_id = player_state.last_extraction_request_id;

                    return Task::perform(
                        async move {
                            crate::video::extract_frame(video_path, pending_ts)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        move |result| Message::FrameExtracted { request_id, result },
                    );
                }
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.current_screen {
            Screen::Home => self.home_view(&self.grid_state),
            Screen::Editor => {
                if let Some(ref player_state) = self.player_state {
                    editor_view(player_state)
                } else {
                    // Fallback if no player state (shouldn't happen)
                    self.home_view(&self.grid_state)
                }
            }
            Screen::Settings => self.settings_view(),
            Screen::About => self.about_view(),
        }
    }

    fn home_view<'a>(&self, grid_state: &'a GridViewState) -> Element<'a, Message> {
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

    fn settings_view(&self) -> Element<'_, Message> {
        use iced::Length;
        use iced::widget::container;
        use iced::widget::text;

        container(text("Settings").size(20))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn about_view(&self) -> Element<'_, Message> {
        use iced::Length;
        use iced::widget::container;
        use iced::widget::text;

        container(text("About").size(20))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
