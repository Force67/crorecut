use std::path::PathBuf;

use iced::widget::scrollable;
use iced::{Element, Subscription, Task};

use crate::state::{GridViewState, ScanStatus, grid_scrollable_id};
use crate::ui::editor::editor_screen;
use crate::video::{VideoMetadata, VideoStreamInfo};

pub use crate::state::{PlayerViewState, RenderStatus, ResizeHandle};

#[derive(Debug, Clone)]
pub enum Screen {
    Home,
    Editor,
    Settings,
    About,
}

pub struct Crorecut {
    pub(crate) screen: Screen,
    pub(crate) videos_directory: PathBuf,
    pub(crate) grid_state: GridViewState,
    pub(crate) player_state: Option<PlayerViewState>,
}

impl Default for Crorecut {
    fn default() -> Self {
        let videos_dir = dirs::video_dir().unwrap_or_else(|| PathBuf::from("."));

        Self {
            screen: Screen::Home,
            videos_directory: videos_dir,
            grid_state: GridViewState::default(),
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
        thumbnail_result: Result<Vec<u8>, String>,
        duration_result: Result<f64, String>,
    },
    VideoSelected(PathBuf),
    GridScrollChanged(scrollable::Viewport),

    // Player View
    PrepareEditor,
    FrameExtracted {
        request_id: u64,
        result: Result<Vec<u8>, String>,
    },
    VideoStreamInfoReceived(Result<VideoStreamInfo, String>),
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
    CopyCommand,
    RunCommand,
    CommandCompleted(Result<PathBuf, String>),
    OpenFile(PathBuf),
    DeleteFile(PathBuf),
    OpenParentDir(PathBuf),
    CommandPreviewHovered(bool),
    TargetSizeChanged(String),

    // Streaming playback
    StartPlayback,
    StopPlayback,
    StreamedFrame {
        frame_bytes: Vec<u8>,
        timestamp: f64,
    },

    // Audio playback
    AudioPositionUpdate(f64),
    AudioStreamEnded,
}

impl Crorecut {
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
            // Navigation
            Message::NavigateTo(screen) => {
                let restore_scroll = matches!(screen, Screen::Home);

                self.screen = screen;

                if restore_scroll {
                    iced::widget::operation::snap_to(
                        grid_scrollable_id(),
                        self.grid_state.scroll_offset,
                    )
                } else {
                    Task::none()
                }
            }

            // Grid handlers
            Message::ScanVideosDirectory => self.handle_scan_videos_directory(),
            Message::VideosScanned(result) => self.handle_videos_scanned(result),
            Message::GenerateNextThumbnail => self.handle_generate_next_thumbnail(),
            Message::ThumbnailGenerated {
                video_path,
                thumbnail_result,
                duration_result,
            } => self.handle_thumbnail_generated(video_path, thumbnail_result, duration_result),
            Message::VideoSelected(path) => self.handle_video_selected(path),
            Message::GridScrollChanged(viewport) => {
                self.grid_state.scroll_offset = viewport.relative_offset();
                Task::none()
            }

            // Player handlers
            Message::PrepareEditor => self.handle_prepare_editor(),
            Message::VideoStreamInfoReceived(result) => self.handle_video_stream_info(result),
            Message::FrameExtracted { request_id, result } => {
                self.handle_frame_extracted(request_id, result)
            }
            Message::SliderChanged(ts) => self.handle_slider_changed(ts),
            Message::TrimStartChanged(value) => self.handle_trim_start_changed(value),
            Message::TrimEndChanged(value) => self.handle_trim_end_changed(value),

            // Crop handlers
            Message::CropMousePressed => self.handle_crop_mouse_pressed(),
            Message::CropMouseMoved {
                x,
                y,
                canvas_width,
                canvas_height,
            } => self.handle_crop_mouse_moved(x, y, canvas_width, canvas_height),
            Message::CropMouseReleased => self.handle_crop_mouse_released(),

            // Command handlers
            Message::CopyCommand => self.handle_copy_command(),
            Message::RunCommand => self.handle_run_command(),
            Message::CommandCompleted(result) => self.handle_command_completed(result),
            Message::OpenFile(path) => self.handle_open_file(path),
            Message::DeleteFile(path) => self.handle_delete_file(path),
            Message::OpenParentDir(path) => self.handle_open_parent_dir(path),
            Message::CommandPreviewHovered(hovered) => self.handle_command_preview_hovered(hovered),
            Message::TargetSizeChanged(value) => self.handle_target_size_changed(value),

            // Streaming handlers
            Message::StartPlayback => self.handle_start_streaming(),
            Message::StopPlayback => self.handle_stop_streaming(),
            Message::StreamedFrame {
                frame_bytes,
                timestamp,
            } => self.handle_streamed_frame(frame_bytes, timestamp),

            // Audio handlers
            Message::AudioPositionUpdate(position) => self.handle_audio_position_update(position),
            Message::AudioStreamEnded => self.handle_audio_stream_ended(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.screen {
            Screen::Home => self.home_screen(&self.grid_state),
            Screen::Editor => {
                if let Some(ref player_state) = self.player_state {
                    editor_screen(player_state)
                } else {
                    // Fallback if no player state (shouldn't happen)
                    self.home_screen(&self.grid_state)
                }
            }
            Screen::Settings => self.settings_screen(),
            Screen::About => self.about_screen(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // Frame + audio streaming subscriptions with clock for A/V sync
        match &self.player_state {
            Some(p) if p.is_streaming => {
                let video_sub = crate::video::frame_streaming::frame_stream_subscription(
                    p.video_path.clone(),
                    p.timestamp,
                    (p.preview_width as u32, p.preview_height as u32),
                    p.video_fps,
                    p.audio_clock.clone(),
                    p.has_audio_stream,
                    p.seek_generation,
                );

                // Add audio subscription if video has audio track
                if p.has_audio_stream {
                    let audio_sub = crate::audio::audio_stream_subscription(
                        p.video_path.clone(),
                        p.timestamp,
                        p.audio_clock.clone(),
                        p.seek_generation,
                    );
                    Subscription::batch([video_sub, audio_sub])
                } else {
                    video_sub
                }
            }
            _ => Subscription::none(),
        }
    }

    fn home_screen<'a>(&self, grid_state: &'a GridViewState) -> Element<'a, Message> {
        use crate::ui::grid::videos_grid;
        use iced::Length;
        use iced::widget::{column, container, text};

        match &grid_state.scan_status {
            ScanStatus::NotStarted | ScanStatus::Scanning => {
                container(text("Scanning videos...").size(20))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into()
            }

            ScanStatus::Complete => videos_grid(&grid_state.videos),

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

    fn settings_screen(&self) -> Element<'_, Message> {
        use iced::Length;
        use iced::widget::container;
        use iced::widget::text;

        container(text("Settings").size(20))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn about_screen(&self) -> Element<'_, Message> {
        use iced::Length;
        use iced::widget::container;
        use iced::widget::text;

        container(text("About").size(20))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
