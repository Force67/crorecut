mod view;
use iced::animation::Easing;
use iced::widget::image;
use iced::{Subscription, Task, window};
use std::time::{Duration, Instant};

pub use view::modal_overlay;

const INDICATOR_ANIMATION_DURATION: Duration = Duration::from_millis(180);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    General = 0,
    About = 1,
}

impl Section {
    fn sidebar_index(self) -> f32 {
        self as u8 as f32
    }
}

#[derive(Debug, Clone)]
struct IndicatorAnimation {
    from: f32,
    started_at: Instant,
}

impl IndicatorAnimation {
    fn new(from: f32) -> Self {
        Self {
            from,
            started_at: Instant::now(),
        }
    }

    fn progress(&self) -> f32 {
        (self.started_at.elapsed().as_secs_f32() / INDICATOR_ANIMATION_DURATION.as_secs_f32())
            .min(1.0)
    }

    fn current_index(&self, to: f32) -> f32 {
        self.from + (to - self.from) * Easing::EaseInOutCubic.value(self.progress())
    }

    fn is_finished(&self) -> bool {
        self.progress() >= 1.0
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    active_section: Section,
    indicator_animation: Option<IndicatorAnimation>,
    default_directory: String,
    ffmpeg_path: String,
    window_icon: image::Handle,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectSection(Section),
    IndicatorFrame,
    DefaultDirectoryChanged(String),
    FfmpegPathChanged(String),
    OpenGithub,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            active_section: Section::default(),
            indicator_animation: None,
            default_directory: dirs::video_dir()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            ffmpeg_path: String::new(),
            window_icon: image::Handle::from_bytes(crate::icons::WINDOW_ICON),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.indicator_animation.is_some() {
            window::frames().map(|_| Message::IndicatorFrame)
        } else {
            Subscription::none()
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectSection(section) => {
                if section == self.active_section {
                    return Task::none();
                }

                let from = self.indicator_index();

                self.active_section = section;
                self.indicator_animation = Some(IndicatorAnimation::new(from));
            }
            Message::IndicatorFrame => {
                if self
                    .indicator_animation
                    .as_ref()
                    .is_some_and(IndicatorAnimation::is_finished)
                {
                    self.indicator_animation = None;
                }
            }
            Message::DefaultDirectoryChanged(value) => {
                self.default_directory = value;
            }
            Message::FfmpegPathChanged(value) => {
                self.ffmpeg_path = value;
            }
            Message::OpenGithub => {
                if let Err(error) = open::that_detached("https://github.com/miredirex/crorecut") {
                    log::error!("Failed to open GitHub repository: {error}");
                }
            }
        }

        Task::none()
    }

    fn indicator_index(&self) -> f32 {
        let to = self.active_section.sidebar_index();

        self.indicator_animation
            .as_ref()
            .map_or(to, |animation| animation.current_index(to))
    }
}
