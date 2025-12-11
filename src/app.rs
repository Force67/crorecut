use iced::{Element, Task};
use std::path::PathBuf;

use crate::ui::editor::{
    ImageTransform, editor_view, get_resize_handle, point_in_selection, resize_selection,
};
use crate::ui::grid::VideoItem;
use crate::ui::selection_canvas::CropSelection;
use crate::video::VideoMetadata;

#[derive(Debug, Clone)]
pub enum ScanStatus {
    NotStarted,
    Scanning,
    Complete,
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

#[derive(Debug, Clone, Default, PartialEq)]
pub enum RenderStatus {
    #[default]
    Idle,
    Rendering,
    Completed(PathBuf),
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct PlayerViewState {
    pub frame_handle: Option<iced::advanced::image::Handle>,
    pub timestamp: f64,
    pub video_path: PathBuf,
    pub duration: f64,

    // Video properties
    pub video_width: u32,
    pub video_height: u32,
    pub preview_width: f32,
    pub preview_height: f32,

    // Extraction state
    pub is_extracting: bool,
    pub last_extraction_request_id: u64,
    pub pending_timestamp: Option<f64>,

    // Frame cache
    pub frame_cache: crate::video::FrameCache,

    // Crop selection state (in preview image space, not canvas space)
    pub crop_selection: Option<CropSelection>,
    pub selection_state: SelectionState,
    pub mouse_position: (f32, f32),
    pub canvas_bounds: (f32, f32), // (width, height) of the canvas container

    // Trim state
    pub trim_start: f64,
    pub trim_end: f64,

    // Render state
    pub render_status: RenderStatus,

    // Command preview hover state
    pub is_command_hovered: bool,
}

impl PlayerViewState {
    /// Get the current image transform based on canvas bounds and preview dimensions
    pub fn get_image_transform(&self) -> ImageTransform {
        ImageTransform::from_contain(
            self.canvas_bounds.0,
            self.canvas_bounds.1,
            self.preview_width,
            self.preview_height,
        )
    }
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

    // Player View
    FrameExtracted {
        request_id: u64,
        result: Result<Vec<u8>, String>,
    },
    DurationReceived(Result<f64, String>),
    DimensionsReceived(Result<(u32, u32), String>),
    SliderChanged(f64),

    // Crop selection
    CropMousePressed,
    CropMouseMoved {
        x: f32,
        y: f32,
        canvas_width: f32,
        canvas_height: f32,
    },
    CropMouseReleased,

    // Trim selection
    TrimStartChanged(f64),
    TrimEndChanged(f64),

    // Command display
    CopyCommand(String),
    RunCommand {
        args: Vec<String>,
        output_path: PathBuf,
    },
    CommandCompleted(Result<PathBuf, String>),
    OpenFile(PathBuf),
    DeleteFile(PathBuf),
    OpenParentDir(PathBuf),
    CommandPreviewHovered(bool),
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
                            log::error!("Failed to load video duration: {error_msg}");
                        }
                    }
                }
                Task::none()
            }

            Message::DimensionsReceived(result) => {
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
                            // Cache the frame before converting to Handle
                            player_state.frame_cache.insert(
                                player_state.video_path.clone(),
                                player_state.timestamp,
                                frame_bytes.clone(),
                            );

                            player_state.frame_handle =
                                Some(iced::advanced::image::Handle::from_bytes(frame_bytes));
                        }
                        Err(error_msg) => {
                            log::error!("Frame extraction failed: {error_msg}");
                            // Keep existing frame visible on error
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

            Message::SliderChanged(new_timestamp) => self.handle_slider_changed(new_timestamp),

            // Crop selection handlers
            Message::CropMousePressed => {
                if let Some(ref mut player_state) = self.player_state {
                    let (canvas_x, canvas_y) = player_state.mouse_position;

                    // Transform from canvas space to image space (no clamping)
                    let transform = player_state.get_image_transform();
                    let (img_x, img_y) = transform.canvas_to_image(canvas_x, canvas_y);

                    // Check if clicking on an existing selection (selection is in image space)
                    if let Some(selection) = player_state.crop_selection {
                        // Check if clicking on a resize handle
                        if let Some(handle) = get_resize_handle(img_x, img_y, &selection) {
                            player_state.selection_state = SelectionState::Resizing {
                                handle,
                                original: selection,
                            };
                        }
                        // Check if clicking inside selection (for moving)
                        else if point_in_selection(img_x, img_y, &selection) {
                            player_state.selection_state = SelectionState::Moving {
                                offset_x: img_x - selection.x,
                                offset_y: img_y - selection.y,
                            };
                        }
                        // Clicking outside - start new selection
                        else {
                            player_state.crop_selection = None;
                            player_state.selection_state = SelectionState::Creating {
                                start_x: canvas_x,
                                start_y: canvas_y,
                            };
                        }
                    } else {
                        // No existing selection - start creating new one
                        player_state.selection_state = SelectionState::Creating {
                            start_x: canvas_x,
                            start_y: canvas_y,
                        };
                    }
                }
                Task::none()
            }

            Message::CropMouseMoved {
                x,
                y,
                canvas_width,
                canvas_height,
            } => {
                if let Some(ref mut player_state) = self.player_state {
                    // Update canvas bounds with actual values from canvas
                    player_state.canvas_bounds = (canvas_width, canvas_height);

                    // Transform from canvas space to image space (no clamping - allow free mouse movement)
                    let transform = player_state.get_image_transform();
                    let (img_x, img_y) = transform.canvas_to_image(x, y);

                    // Store mouse position in canvas space
                    player_state.mouse_position = (x, y);

                    match player_state.selection_state {
                        SelectionState::Creating { start_x, start_y } => {
                            // Transform start position from canvas to image space
                            let (start_img_x, start_img_y) =
                                transform.canvas_to_image(start_x, start_y);

                            // Create selection in image space with clamped bounds
                            let min_x = start_img_x
                                .min(img_x)
                                .clamp(0.0, player_state.preview_width);
                            let min_y = start_img_y
                                .min(img_y)
                                .clamp(0.0, player_state.preview_height);
                            let max_x = start_img_x
                                .max(img_x)
                                .clamp(0.0, player_state.preview_width);
                            let max_y = start_img_y
                                .max(img_y)
                                .clamp(0.0, player_state.preview_height);

                            player_state.crop_selection = Some(CropSelection {
                                x: min_x,
                                y: min_y,
                                width: max_x - min_x,
                                height: max_y - min_y,
                            });
                        }
                        SelectionState::Moving { offset_x, offset_y } => {
                            if let Some(ref mut selection) = player_state.crop_selection {
                                let new_x = img_x - offset_x;
                                let new_y = img_y - offset_y;

                                // Clamp to keep selection within image bounds
                                selection.x =
                                    new_x.clamp(0.0, player_state.preview_width - selection.width);
                                selection.y = new_y
                                    .clamp(0.0, player_state.preview_height - selection.height);
                            }
                        }
                        SelectionState::Resizing { handle, original } => {
                            if let Some(ref mut selection) = player_state.crop_selection {
                                resize_selection(selection, handle, img_x, img_y, &original);

                                // Clamp selection to image bounds
                                selection.x = selection.x.clamp(0.0, player_state.preview_width);
                                selection.y = selection.y.clamp(0.0, player_state.preview_height);
                                selection.width = selection
                                    .width
                                    .clamp(10.0, player_state.preview_width - selection.x);
                                selection.height = selection
                                    .height
                                    .clamp(10.0, player_state.preview_height - selection.y);
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

            Message::RunCommand { args, output_path } => {
                if let Some(ref mut player_state) = self.player_state {
                    player_state.render_status = RenderStatus::Rendering;
                }
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

            Message::CommandCompleted(result) => {
                if let Some(ref mut player_state) = self.player_state {
                    match result {
                        Ok(output_path) => {
                            log::info!("FFmpeg command completed successfully");
                            player_state.render_status = RenderStatus::Completed(output_path);
                        }
                        Err(ref error_msg) => {
                            log::error!("FFmpeg command failed: {error_msg}");
                            player_state.render_status = RenderStatus::Failed(error_msg.clone());
                        }
                    }
                }
                Task::none()
            }

            Message::OpenFile(path) => {
                if let Err(e) = open::that_detached(&path) {
                    log::error!("Failed to open file: {e}");
                }
                Task::none()
            }

            Message::DeleteFile(path) => {
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

            Message::OpenParentDir(filepath) => {
                if let Some(parent) = filepath.parent() {
                    if let Err(e) = open::that_detached(parent) {
                        log::error!("Failed to open parent directory: {e}");
                    }
                } else {
                    log::warn!("Path has no parent directory: {}", filepath.display());
                }
                Task::none()
            }

            Message::CommandPreviewHovered(is_hovered) => {
                if let Some(ref mut player_state) = self.player_state {
                    player_state.is_command_hovered = is_hovered;
                }
                Task::none()
            }
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
            video_width: 0,     // Will be set when dimensions are received
            video_height: 0,    // Will be set when dimensions are received
            preview_width: 0.0, // Will be calculated when dimensions are received
            preview_height: 360.0,
            is_extracting: false,
            last_extraction_request_id: 0,
            pending_timestamp: None,
            frame_cache: crate::video::FrameCache::new(75),
            crop_selection: None,
            selection_state: SelectionState::Idle,
            mouse_position: (0.0, 0.0),
            canvas_bounds: (1.0, 1.0), // Will be updated on first mouse event
            trim_start: 0.0,
            trim_end: 0.0, // Will be set when duration is received
            render_status: RenderStatus::Idle,
            is_command_hovered: false,
        });

        // Switch to editor screen
        self.current_screen = Screen::Editor;

        // Get video metadata and load initial frame
        let path_for_duration = video_path.clone();
        let path_for_dimensions = video_path.clone();
        let path_for_frame = video_path;

        Task::batch(vec![
            Task::perform(
                async move { crate::video::get_video_duration_sec(path_for_duration).await },
                |result| Message::DurationReceived(result.map_err(|e| e.to_string())),
            ),
            Task::perform(
                async move { crate::video::get_video_dimensions(path_for_dimensions).await },
                |result| Message::DimensionsReceived(result.map_err(|e| e.to_string())),
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

    /// Handle slider changes with single-extraction limit to avoid `FFmpeg` process spam
    fn handle_slider_changed(&mut self, new_timestamp: f64) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            // Update timestamp immediately for responsive UI
            player_state.timestamp = new_timestamp;

            // Check cache first
            if let Some(cached_bytes) = player_state
                .frame_cache
                .get(&player_state.video_path, new_timestamp)
            {
                // Cache hit! Update frame immediately without spawning FFmpeg
                player_state.frame_handle =
                    Some(iced::advanced::image::Handle::from_bytes(cached_bytes));
                player_state.pending_timestamp = None;
                return Task::none();
            }

            // Cache miss - check if extraction is already in progress
            if player_state.is_extracting {
                // Extraction in progress - store as pending (will be processed when current extraction finishes)
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

            ScanStatus::Complete => grid_view(&grid_state.videos),

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
