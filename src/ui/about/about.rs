use iced::{Element, Task};

#[derive(Debug, Clone, Default)]
pub struct About {
    // Placeholder for future about state
}

#[derive(Debug, Clone)]
pub enum Message {
    // Placeholder for future about messages
}

impl About {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // Handle about messages here
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        use iced::Length;
        use iced::widget::{container, text};

        container(text("About").size(20))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
