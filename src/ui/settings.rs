mod view;

use iced::Task;

pub use view::modal_overlay;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    General,
    About,
}

#[derive(Debug, Clone, Default)]
pub struct Settings {
    active_section: Section,
    default_directory: String,
    ffmpeg_path: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectSection(Section),
    DefaultDirectoryChanged(String),
    FfmpegPathChanged(String),
}

impl Settings {
    pub fn new() -> Self {
        Self {
            default_directory: dirs::video_dir()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            ..Self::default()
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectSection(section) => {
                self.active_section = section;
            }
            Message::DefaultDirectoryChanged(value) => {
                self.default_directory = value;
            }
            Message::FfmpegPathChanged(value) => {
                self.ffmpeg_path = value;
            }
        }

        Task::none()
    }
}
