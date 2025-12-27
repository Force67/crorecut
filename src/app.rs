use std::path::PathBuf;

use iced::{Element, Subscription, Task};

use crate::ui::{about, editor, grid, settings};

/// Screen enum - Grid state is stored separately on Crorecut since it's the home screen
pub enum Screen {
    Grid,
    Editor(editor::Editor),
    Settings(settings::Settings),
    About(about::About),
}

/// Nested message enum with screen-specific variants
#[derive(Debug, Clone)]
pub enum Message {
    // App-level navigation
    NavigateToGrid,
    NavigateToEditor(PathBuf),
    NavigateToSettings,
    NavigateToAbout,

    // Screen-specific messages (nested)
    Grid(grid::Message),
    Editor(editor::Message),
    Settings(settings::Message),
    About(about::Message),
}

pub struct Crorecut {
    screen: Screen,
    videos_directory: PathBuf,
    grid_state: grid::Grid,
}

impl Crorecut {
    pub fn new() -> (Self, Task<Message>) {
        let videos_dir = dirs::video_dir().unwrap_or_else(|| PathBuf::from("."));
        let grid_state = grid::Grid::new(videos_dir.clone());

        let app = Self {
            screen: Screen::Grid,
            videos_directory: videos_dir,
            grid_state,
        };

        let initial_task = Task::done(Message::Grid(grid::Message::ScanVideosDirectory));
        (app, initial_task)
    }

    pub fn title(&self) -> String {
        String::from("Crorecut")
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // App-level navigation
            Message::NavigateToGrid => {
                let restore_scroll = !matches!(self.screen, Screen::Grid);
                self.screen = Screen::Grid;

                if restore_scroll {
                    iced::widget::operation::snap_to(
                        grid::grid_scrollable_id(),
                        self.grid_state.scroll_offset,
                    )
                } else {
                    Task::none()
                }
            }

            Message::NavigateToEditor(video_path) => {
                self.screen = Screen::Editor(editor::Editor::new(video_path.clone()));

                // Start preparing the editor
                Task::batch([
                    Task::done(Message::Editor(editor::Message::PrepareEditor)),
                    Task::perform(
                        async move { crate::video::get_video_stream_info(&video_path).await },
                        |result| {
                            Message::Editor(editor::Message::VideoStreamInfoReceived(
                                result.map_err(|e| e.to_string()),
                            ))
                        },
                    ),
                ])
            }

            Message::NavigateToSettings => {
                self.screen = Screen::Settings(settings::Settings::new());
                Task::none()
            }

            Message::NavigateToAbout => {
                self.screen = Screen::About(about::About::new());
                Task::none()
            }

            Message::Grid(msg) => {
                // Special case: VideoSelected triggers navigation to editor
                if let grid::Message::VideoSelected(ref path) = msg {
                    return self.update(Message::NavigateToEditor(path.clone()));
                }

                self.grid_state.update(msg).map(Message::Grid)
            }

            Message::Editor(msg) => {
                // Special case: GoBack navigates to grid
                if matches!(msg, editor::Message::GoBack) {
                    return self.update(Message::NavigateToGrid);
                }

                if let Screen::Editor(ref mut state) = self.screen {
                    state.update(msg).map(Message::Editor)
                } else {
                    Task::none()
                }
            }

            Message::Settings(msg) => {
                if let Screen::Settings(ref mut state) = self.screen {
                    state.update(msg).map(Message::Settings)
                } else {
                    Task::none()
                }
            }

            Message::About(msg) => {
                if let Screen::About(ref mut state) = self.screen {
                    state.update(msg).map(Message::About)
                } else {
                    Task::none()
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.screen {
            Screen::Grid => self.grid_state.view().map(Message::Grid),
            Screen::Editor(state) => state.view().map(Message::Editor),
            Screen::Settings(state) => state.view().map(Message::Settings),
            Screen::About(state) => state.view().map(Message::About),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match &self.screen {
            Screen::Editor(state) => state.subscription().map(Message::Editor),
            _ => Subscription::none(),
        }
    }
}
