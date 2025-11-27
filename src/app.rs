use iced::{Element, Task};

pub struct App;

#[derive(Debug, Clone)]
pub enum Message {}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (Self, Task::none())
    }

    pub fn title(&self) -> String {
        String::from("Crorecut")
    }

    pub fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        iced::widget::container(
            iced::widget::text("Welcome to Crorecut - a minimalistic video editing tool")
        )
        .center_x(iced::Fill)
        .center_y(iced::Fill)
        .into()
    }
}
