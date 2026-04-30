//! Styled ripple button with decorative corner dots and an optional icon

use iced::Length::Shrink;
use iced::widget::{button, column, container, row, stack, svg, text};
use iced::{Alignment, Color, Element};

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

pub fn crorecut_button<'a, Message>(
    icon: Option<&'static [u8]>,
    label: String,
    sublabel: Option<String>,
    on_press: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let label_content: Element<'_, Message> = match sublabel {
        Some(sub) => column![
            text(label).color(app_colors::TEXT).size(14),
            text(sub).color(app_colors::TEXT_SECONDARY).size(12)
        ]
        .into(),
        None => text(label).color(app_colors::TEXT).size(14).into(),
    };

    let content: Element<'_, Message> = match icon {
        Some(icon_data) => {
            let icon_svg = svg(svg::Handle::from_memory(icon_data))
                .width(28)
                .height(28);
            row![icon_svg, label_content]
                .spacing(12)
                .align_y(Alignment::Center)
                .into()
        }
        None => label_content,
    };

    let enabled = on_press.is_some();

    let mut btn = button(stack![
        container(content)
            .center_x(Shrink)
            .center_y(Shrink)
            .padding([12, 16]), // [vertical, horizontal]
        corner_dots(),
    ])
    .style(button_style)
    .padding(2);

    if let Some(msg) = on_press {
        btn = btn.on_press(msg);
    }

    ripple(btn)
        .ripple_color(app_colors::RIPPLE_ON_GRAY)
        .enabled(enabled)
        .into()
}
