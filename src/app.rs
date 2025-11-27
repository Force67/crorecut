use iced::{Element, Task};
use std::path::PathBuf;

pub struct App {
    video_state: VideoState,
    current_video_path: Option<PathBuf>,
    current_duration: Option<f64>,
    last_frame_request: Option<std::time::Instant>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            video_state: VideoState::Idle,
            current_video_path: None,
            current_duration: None,
            last_frame_request: None,
        }
    }
}

pub enum VideoState {
    Idle,
    Loading,
    Loaded {
        frame_handle: iced::advanced::image::Handle,
        timestamp: f64,
        video_path: PathBuf,
        duration: f64,
    },
    Error(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    LoadFrame,
    FrameExtracted(Result<Vec<u8>, String>),
    DurationReceived(Result<f64, String>),
    SliderChanged(f64),
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let initial_task = Task::perform(async { Message::LoadFrame }, |msg| msg);
        (Self::default(), initial_task)
    }

    pub fn title(&self) -> String {
        String::from("Crorecut")
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LoadFrame => {
                let video_path = PathBuf::from(r"test_video.mp4");
                self.current_video_path = Some(video_path.clone());
                self.video_state = VideoState::Loading;

                Task::perform(
                    async move {
                        let result = crate::video::get_video_duration_sec(video_path).await;
                        result.map_err(|e| e.to_string())
                    },
                    Message::DurationReceived,
                )
            }

            Message::DurationReceived(result) => match result {
                Ok(duration) => {
                    self.current_duration = Some(duration);

                    if let Some(video_path) = &self.current_video_path {
                        let video_path = video_path.clone();

                        Task::perform(
                            async move {
                                let result = crate::video::extract_frame(video_path, 0.0).await;
                                result.map_err(|e| e.to_string())
                            },
                            Message::FrameExtracted,
                        )
                    } else {
                        Task::none()
                    }
                }
                Err(error_msg) => {
                    self.video_state = VideoState::Error(error_msg);
                    Task::none()
                }
            },

            Message::FrameExtracted(result) => {
                match result {
                    Ok(frame_bytes) => {
                        let frame_handle = iced::advanced::image::Handle::from_bytes(frame_bytes);

                        let timestamp =
                            if let VideoState::Loaded { timestamp, .. } = &self.video_state {
                                *timestamp
                            } else {
                                0.0
                            };

                        if let (Some(video_path), Some(duration)) =
                            (&self.current_video_path, self.current_duration)
                        {
                            self.video_state = VideoState::Loaded {
                                frame_handle,
                                timestamp,
                                video_path: video_path.clone(),
                                duration,
                            };
                        }
                    }
                    Err(error_msg) => {
                        self.video_state = VideoState::Error(error_msg);
                    }
                }

                Task::none()
            }

            Message::SliderChanged(new_timestamp) => {
                if let VideoState::Loaded {
                    ref mut timestamp, ..
                } = self.video_state
                {
                    *timestamp = new_timestamp;
                }

                let now = std::time::Instant::now();

                let should_extract = self
                    .last_frame_request
                    .map(|last| now.duration_since(last).as_millis() >= 100)
                    .unwrap_or(true);

                if should_extract {
                    self.last_frame_request = Some(now);

                    if let Some(video_path) = &self.current_video_path {
                        let video_path = video_path.clone();

                        Task::perform(
                            async move {
                                let result =
                                    crate::video::extract_frame(video_path, new_timestamp).await;
                                result.map_err(|e| e.to_string())
                            },
                            Message::FrameExtracted,
                        )
                    } else {
                        Task::none()
                    }
                } else {
                    Task::none()
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        use iced::widget::{column, container, slider, text};
        use iced::{Alignment, Fill};

        let content = match &self.video_state {
            VideoState::Idle => column![
                text("Welcome to Crorecut").size(24),
                text("Initializing...").size(14),
            ]
            .align_x(Alignment::Center)
            .spacing(10),

            VideoState::Loading => {
                column![text("Extracting frame...").size(20)].align_x(Alignment::Center)
            }

            VideoState::Loaded {
                frame_handle,
                timestamp,
                duration,
                ..
            } => {
                let frame_display = iced::widget::image(frame_handle.clone())
                    .width(Fill)
                    .height(Fill)
                    .content_fit(iced::ContentFit::Contain);

                let timeline_slider = slider(0.0..=*duration, *timestamp, Message::SliderChanged)
                    .width(Fill)
                    .step(0.1);

                let time_display = text(format!("{:.2}s / {:.2}s", timestamp, duration)).size(14);

                column![frame_display, time_display, timeline_slider,]
                    .align_x(Alignment::Center)
                    .spacing(10)
            }

            VideoState::Error(error_msg) => column![
                text("Error").size(24).color([1.0, 0.0, 0.0]),
                text(error_msg).size(14),
                text("Please check:").size(12),
                text("1. FFmpeg is installed and in PATH").size(12),
                text("2. Video file exists at specified path").size(12),
                text("3. Video file is a valid format").size(12),
            ]
            .align_x(Alignment::Center)
            .spacing(10),
        };

        container(content).center_x(Fill).center_y(Fill).into()
    }
}
