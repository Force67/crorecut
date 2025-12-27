//! Styled ripple button with decorative corner dots and an optional icon

use iced::Length::{Fill, Shrink};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, container, row, stack, svg, text};
use iced::{Alignment, Color, Element, color};

use super::ripple;

fn button_style(_theme: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(iced::Background::Color(color!(0x232326))),
        text_color: Color::WHITE,
        border: iced::Border {
            color: color!(0x3D3D43),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn crorecut_button<'a, Message>(
    icon: Option<&'static [u8]>,
    label: &'a str,
    on_press: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let corner_dots = svg(svg::Handle::from_memory(crate::icons::CORNER_DOTS_SVG))
        .width(7)
        .height(7)
        .style(|_, _| svg::Style {
            color: Some(color!(0x525259)),
        });

    let content: Element<'_, Message> = match icon {
        Some(icon_data) => {
            let icon_svg = svg(svg::Handle::from_memory(icon_data))
                .width(24)
                .height(24);
            row![icon_svg, text(label).color(color!(0xB3B3B3)).size(14)]
                .spacing(10)
                .align_y(Alignment::Center)
                .into()
        }
        None => text(label).color(color!(0xB3B3B3)).size(14).into(),
    };

    let corner_overlay = container(corner_dots)
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Right)
        .align_y(Vertical::Bottom);

    let btn = {
        let mut b = button(stack![
            container(content)
                .width(Shrink)
                .height(Shrink)
                .center_x(Shrink)
                .center_y(Shrink)
                .padding([8, 14]),
            corner_overlay,
        ])
        .style(button_style)
        .padding(4);

        if let Some(msg) = on_press {
            b = b.on_press(msg);
        }
        b
    };

    ripple(btn).ripple_color(color!(0x696977, 0.25)).into()
}
