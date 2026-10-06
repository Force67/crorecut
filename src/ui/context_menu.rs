use iced::widget::{button, column, container, mouse_area, opaque, pin, space, stack, text};
use iced::{Border, Color, Length, Point, Widget};

use super::color as app_colors;
use super::ripple;

pub struct ContextMenuItem<M> {
    label: &'static str,
    message: Option<M>,
    color: Option<Color>,
}

impl<M> ContextMenuItem<M> {
    pub fn new(label: &'static str, message: M) -> Self {
        Self {
            label,
            message: Some(message),
            color: None,
        }
    }

    pub fn label(label: &'static str) -> Self {
        Self {
            label,
            message: None,
            color: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

pub fn context_menu_overlay<'a, M: Clone + 'a>(
    position: Point,
    items: Vec<ContextMenuItem<M>>,
    on_dismiss: M,
) -> impl Widget<M> + 'a {
    let menu = context_menu_widget(items);

    // Overlay: transparent backdrop (for click-outside dismiss) + positioned menu
    stack![
        // Backdrop catches clicks to dismiss
        mouse_area(space().width(Length::Fill).height(Length::Fill)).on_press(on_dismiss),
        // Menu pinned at cursor position
        pin(menu).x(position.x).y(position.y),
    ]
}

fn context_menu_widget<'a, M: Clone + 'a>(items: Vec<ContextMenuItem<M>>) -> impl Widget<M> + 'a {
    let menu_items = items.into_iter().map(|item| {
        let text_color = item.color;
        let enabled = item.message.is_some();
        ripple(
            button(text(item.label).size(13))
                .on_press_maybe(item.message)
                .width(Length::Fill)
                .padding([8, 16])
                .style(move |theme, status| menu_item_style(theme, status, text_color)),
        )
        .ripple_color(app_colors::RIPPLE_ON_GRAY)
        .enabled(enabled)
    });

    opaque(
        container(column(menu_items).width(Length::Fixed(160.0)))
            .padding(2)
            .style(menu_container_style),
    )
}

fn menu_item_style(
    _theme: &iced::Theme,
    status: button::Status,
    custom_color: Option<Color>,
) -> button::Style {
    button::Style {
        background: Some(iced::Background::Color(match status {
            button::Status::Hovered | button::Status::Pressed => iced::color!(0x3A3A42, 0.3),
            _ => app_colors::BUTTON_BG,
        })),
        text_color: custom_color.unwrap_or(app_colors::TEXT),
        border: Border::default(),
        ..Default::default()
    }
}

fn menu_container_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(app_colors::BUTTON_BG)),
        border: Border {
            color: app_colors::BUTTON_BORDER,
            width: 1.0,
            radius: 2.0.into(),
        },
        ..Default::default()
    }
}
