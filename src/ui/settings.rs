mod view;
use iced::Task;
use iced::widget::image;
pub use view::modal_overlay;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    General,
    About,
}

#[derive(Debug, Clone)]
pub struct Settings {
    active_section: Section,
    default_directory: String,
    ffmpeg_path: String,
    window_icon: image::Handle,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectSection(Section),
    DefaultDirectoryChanged(String),
    FfmpegPathChanged(String),
    OpenGithub,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            active_section: Section::default(),
            default_directory: dirs::video_dir()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            ffmpeg_path: String::new(),
            window_icon: image::Handle::from_bytes(crate::icons::WINDOW_ICON),
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
            Message::OpenGithub => {
                if let Err(error) = open::that_detached("https://github.com/miredirex/crorecut") {
                    log::error!("Failed to open GitHub repository: {error}");
                }
            }
        }

        Task::none()
    }
}
