//! Styled ripple button with decorative corner dots and an optional icon

use iced::widget::{button, column, container, row, stack, svg, text};
use iced::{Alignment, Color, Length, Widget};

use super::color as app_colors;
use super::{corner_dots, ripple};

fn button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Disabled => app_colors::BUTTON_BG_DISABLED,
        _ => app_colors::BUTTON_BG,
    };

    button::Style {
        background: Some(iced::Background::Color(background)),
        text_color: Color::WHITE,
        border: iced::Border {
            color: app_colors::BUTTON_BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub struct CrorecutButton<Message> {
    icon: Option<&'static [u8]>,
    label: String,
    sublabel: Option<String>,
    on_press: Option<Message>,
    width: Length,
}

impl<Message> CrorecutButton<Message> {
    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

pub fn crorecut_button<Message>(
    icon: Option<&'static [u8]>,
    label: String,
    sublabel: Option<String>,
    on_press: Option<Message>,
) -> CrorecutButton<Message> {
    CrorecutButton {
        icon,
        label,
        sublabel,
        on_press,
        width: Length::Fit,
    }
}

impl<Message> CrorecutButton<Message> {
    pub fn into_widget<'a>(self) -> impl Widget<Message> + 'a
    where
        Message: Clone + 'a,
    {
        let icon = self.icon.map(|icon_data| {
            svg(svg::Handle::from_memory(icon_data))
                .width(28)
                .height(28)
        });

        let label = column![
            text(self.label).color(app_colors::TEXT),
            self.sublabel
                .map(|sub| text(sub).size(13).color(app_colors::TEXT_SECONDARY)),
        ];

        let content = row![icon, label].spacing(12).align_y(Alignment::Center);

        let enabled = self.on_press.is_some();

        let btn = button(stack![
            container(content)
                .center_x(self.width)
                .center_y(Length::Fit)
                .padding([12, 16]), // [vertical, horizontal]
            corner_dots(),
        ])
        .on_press_maybe(self.on_press)
        .width(self.width)
        .style(button_style)
        .padding(2);

        ripple(btn)
            .ripple_color(app_colors::RIPPLE_ON_GRAY)
            .enabled(enabled)
    }
}
