use crate::ui::{editor, grid};
use iced::{Element, Subscription, Task, theme};
use std::path::PathBuf;

/// Screen enum - Grid state is stored separately on Crorecut since it's the home screen
enum Screen {
    Grid,
    Editor(editor::Editor),
}

/// Nested message enum with screen-specific variants
#[derive(Debug, Clone)]
pub enum Message {
    // App-level navigation
    NavigateToGrid,
    NavigateToEditor(PathBuf),

    // Screen-specific messages (nested)
    Grid(grid::Message),
    Editor(editor::Message),
    // TODO: Settings(settings::Message),
    // TODO: About(about::Message),
}

pub struct Crorecut {
    screen: Screen,
    grid_state: grid::Grid,
}

impl Crorecut {
    pub fn new() -> (Self, Task<Message>) {
        let videos_dir = dirs::video_dir().unwrap_or_else(|| PathBuf::from("."));
        let grid_state = grid::Grid::new(videos_dir);

        let app = Self {
            screen: Screen::Grid,
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
                        self.grid_state.scroll_offset(),
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
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.screen {
            Screen::Grid => self.grid_state.view().map(Message::Grid),
            Screen::Editor(state) => state.view().map(Message::Editor),
        }
    }

    pub fn style(&self, _theme: &iced::Theme) -> iced::theme::Style {
        theme::Style {
            background_color: crate::ui::color::CRORECUT_BG,
            text_color: crate::ui::color::CRORECUT_TEXT,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match &self.screen {
            Screen::Editor(state) => state.subscription().map(Message::Editor),
            _ => Subscription::none(),
        }
    }
}
