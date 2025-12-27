use iced::{Element, Length::Fill, Task};

#[derive(Debug, Clone, Default)]
pub struct Settings {
    // Placeholder for future settings state
}

#[derive(Debug, Clone)]
pub enum Message {
    // Placeholder for future settings messages
}

impl Settings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // ...
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        use iced::widget::{container, text};

        container(text("Settings").size(20))
            .center_x(Fill)
            .center_y(Fill)
            .into()
    }
}
