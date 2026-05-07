mod checkbox;
pub mod color;
mod context_menu;
mod crorecut_button;
pub mod editor;
pub mod grid;
mod positioned_mouse_area;
mod ripple;
mod settings;
mod styles;

use iced::Length;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{container, svg};

use checkbox::crorecut_checkbox;
pub use context_menu::{ContextMenuItem, context_menu_overlay};
use crorecut_button::crorecut_button;
use positioned_mouse_area::positioned_mouse_area;
use ripple::ripple;

pub(crate) fn corner_dots<'a, Message>() -> iced::widget::Container<'a, Message> {
    let corner_dots = svg(svg::Handle::from_memory(crate::icons::CORNER_DOTS_SVG))
        .width(7)
        .height(7)
        .style(|_, _| svg::Style {
            color: Some(color::CONTROL_ACCENT),
        });

    container(corner_dots)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Right)
        .align_y(Vertical::Bottom)
}
