use iced::{Element, Task};
use std::path::PathBuf;

use crate::ui::grid::VideoItem;
use crate::ui::selection_canvas::{CropSelection, SelectionCanvas};
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

            Message::VideoSelected(video_path) => {
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

            // Player view messages
            Message::DurationReceived(result) => {
                if let Some(ref mut player_state) = self.player_state {
                    match result {
                        Ok(duration) => {
                            player_state.duration = duration;
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

            Message::SliderChanged(new_timestamp) => {
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
                                tokio::time::sleep(std::time::Duration::from_millis(remaining_ms))
                                    .await;
                                Message::ExtractPendingFrame
                            },
                            |msg| msg,
                        );
                    }
                }
                Task::none()
            }

            Message::ExtractPendingFrame => {
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
                        if let Some(handle) = Self::get_resize_handle(x, y, &selection) {
                            player_state.selection_state = SelectionState::Resizing {
                                handle,
                                original: selection,
                            };
                        }
                        // Check if clicking inside selection (for moving)
                        else if Self::point_in_selection(x, y, &selection) {
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
                                Self::resize_selection(selection, handle, x, y, &original);
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
        }
    }

    // Helper methods for crop selection
    fn get_resize_handle(x: f32, y: f32, selection: &CropSelection) -> Option<ResizeHandle> {
        const HANDLE_SIZE: f32 = 10.0;

        let left = selection.x;
        let right = selection.x + selection.width;
        let top = selection.y;
        let bottom = selection.y + selection.height;

        // Check corner handles
        if (x - left).abs() < HANDLE_SIZE && (y - top).abs() < HANDLE_SIZE {
            return Some(ResizeHandle::TopLeft);
        }
        if (x - right).abs() < HANDLE_SIZE && (y - top).abs() < HANDLE_SIZE {
            return Some(ResizeHandle::TopRight);
        }
        if (x - left).abs() < HANDLE_SIZE && (y - bottom).abs() < HANDLE_SIZE {
            return Some(ResizeHandle::BottomLeft);
        }
        if (x - right).abs() < HANDLE_SIZE && (y - bottom).abs() < HANDLE_SIZE {
            return Some(ResizeHandle::BottomRight);
        }

        None
    }

    fn point_in_selection(x: f32, y: f32, selection: &CropSelection) -> bool {
        x >= selection.x
            && x <= selection.x + selection.width
            && y >= selection.y
            && y <= selection.y + selection.height
    }

    fn resize_selection(
        selection: &mut CropSelection,
        handle: ResizeHandle,
        x: f32,
        y: f32,
        original: &CropSelection,
    ) {
        match handle {
            ResizeHandle::TopLeft => {
                selection.x = x;
                selection.y = y;
                selection.width = (original.x + original.width) - x;
                selection.height = (original.y + original.height) - y;
            }
            ResizeHandle::TopRight => {
                selection.y = y;
                selection.width = x - original.x;
                selection.height = (original.y + original.height) - y;
            }
            ResizeHandle::BottomLeft => {
                selection.x = x;
                selection.width = (original.x + original.width) - x;
                selection.height = y - original.y;
            }
            ResizeHandle::BottomRight => {
                selection.width = x - original.x;
                selection.height = y - original.y;
            }
        }

        // Ensure minimum size and prevent negative dimensions
        if selection.width < 10.0 {
            selection.width = 10.0;
        }
        if selection.height < 10.0 {
            selection.height = 10.0;
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.current_screen {
            Screen::Home => self.home_view(&self.grid_state),
            Screen::Editor => {
                if let Some(ref player_state) = self.player_state {
                    self.editor_view(player_state)
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

    fn editor_view<'a>(&self, player_state: &'a PlayerViewState) -> Element<'a, Message> {
        use iced::widget::{
            button, canvas, column, container, image, mouse_area, row, slider, stack, text,
        };
        use iced::{Alignment, Length};

        // Helper function to format timestamps as HH:MM:SS
        fn format_timestamp(seconds: f64) -> String {
            let hours = (seconds / 3600.0).floor() as u32;
            let minutes = ((seconds % 3600.0) / 60.0).floor() as u32;
            let secs = (seconds % 60.0).floor() as u32;
            format!("{:02}:{:02}:{:02}", hours, minutes, secs)
        }

        // Back button and header
        let back_button = button(text("← Back"))
            .on_press(Message::NavigateTo(Screen::Home))
            .padding(10);

        let video_name = player_state
            .video_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown");

        let header = row![back_button, text(video_name).size(20)]
            .spacing(20)
            .padding(10)
            .align_y(Alignment::Center);

        // Frame display area
        let frame_display = if let Some(ref handle) = player_state.frame_handle {
            let image_widget = image(handle.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(iced::ContentFit::Contain);

            // Create canvas for selection overlay
            let selection_canvas = canvas(SelectionCanvas {
                selection: player_state.crop_selection,
            })
            .width(Length::Fill)
            .height(Length::Fill);

            // Stack image and overlay
            let content = stack![image_widget, selection_canvas];

            // Wrap in mouse area for interaction
            let interactive_content = mouse_area(content)
                .on_press(Message::CropMousePressed)
                .on_move(|point: iced::Point| Message::CropMouseMoved {
                    x: point.x,
                    y: point.y,
                })
                .on_release(Message::CropMouseReleased)
                .interaction(iced::mouse::Interaction::Crosshair);

            container(interactive_content)
                .width(Length::Fill)
                .height(Length::FillPortion(4))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(iced::Background::Color(iced::Color::BLACK)),
                    ..Default::default()
                })
        } else {
            // Placeholder while loading
            container(
                text("Loading video...")
                    .size(20)
                    .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
            )
            .width(Length::Fill)
            .height(Length::FillPortion(4))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::BLACK)),
                ..Default::default()
            })
        };

        // Timeline controls
        let current_time = text(format_timestamp(player_state.timestamp))
            .size(14)
            .color(iced::Color::WHITE);

        let total_time = text(format_timestamp(player_state.duration))
            .size(14)
            .color(iced::Color::from_rgb(0.7, 0.7, 0.7));

        // Timeline slider
        let timeline_slider = if player_state.duration > 0.0 {
            slider(
                0.0..=player_state.duration,
                player_state.timestamp,
                Message::SliderChanged,
            )
            .step(0.1) // 100ms precision
            .width(Length::Fill)
        } else {
            // Disabled slider while duration loads
            slider(0.0..=1.0, 0.0, |_| Message::SliderChanged(0.0)).width(Length::Fill)
        };

        let timeline_controls = row![current_time, timeline_slider, total_time]
            .spacing(12)
            .padding(20)
            .align_y(Alignment::Center)
            .width(Length::Fill);

        // Main layout
        column![header, frame_display, timeline_controls]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
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
