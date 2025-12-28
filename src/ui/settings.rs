#![allow(dead_code)] // TODO: Remove this

use iced::{Element, Length::Fill, Task};

#[derive(Debug, Clone, Default)]
struct Settings {
    // Placeholder for future settings state
}

#[derive(Debug, Clone)]
enum Message {
    // Placeholder for future settings messages
}

impl Settings {
    fn new() -> Self {
        Self::default()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // ...
        }
    }

    fn view(&self) -> Element<'_, Message> {
        use iced::widget::{container, text};

        container(text("Settings").size(20))
            .center_x(Fill)
            .center_y(Fill)
            .into()
    }
}
