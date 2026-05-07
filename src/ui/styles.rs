use iced::widget::{button, pick_list as iced_pick_list, text_input as iced_text_input};
use iced::{Background, Border};

use super::color as app_colors;

pub(crate) fn rounded_icon_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => iced::color!(0x414449),
        button::Status::Pressed => iced::color!(0x292B2D),
        _ => iced::color!(0x202020),
    };

    button::Style {
        background: Some(Background::Color(background)),
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(crate) fn text_link_button(_theme: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: iced::color!(0x5197ED),
        ..Default::default()
    }
}

pub(crate) fn square_icon_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Pressed => app_colors::COMMAND_PREVIEW_BG,
        _ => app_colors::BUTTON_BG,
    };

    button::Style {
        background: Some(Background::Color(background)),
        border: Border {
            color: app_colors::BUTTON_BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn text_input(
    _theme: &iced::Theme,
    status: iced_text_input::Status,
) -> iced_text_input::Style {
    let background = match status {
        iced_text_input::Status::Hovered
        | iced_text_input::Status::Focused { is_hovered: true } => app_colors::COMMAND_PREVIEW_BG,
        _ => app_colors::BUTTON_BG,
    };

    iced_text_input::Style {
        background: Background::Color(background),
        border: Border {
            color: app_colors::BUTTON_BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        icon: app_colors::TEXT_SECONDARY,
        placeholder: app_colors::TEXT_SECONDARY,
        value: app_colors::TEXT,
        selection: app_colors::CONTROL_ACCENT,
    }
}

pub(crate) fn pick_list(
    _theme: &iced::Theme,
    status: iced_pick_list::Status,
) -> iced_pick_list::Style {
    let background = match status {
        iced_pick_list::Status::Hovered | iced_pick_list::Status::Opened { .. } => {
            app_colors::COMMAND_PREVIEW_BG
        }
        iced_pick_list::Status::Disabled => app_colors::BUTTON_BG_DISABLED,
        iced_pick_list::Status::Active => app_colors::BUTTON_BG,
    };

    let text_color = match status {
        iced_pick_list::Status::Disabled => app_colors::TEXT_SECONDARY,
        _ => app_colors::TEXT,
    };

    iced_pick_list::Style {
        text_color,
        placeholder_color: app_colors::TEXT_SECONDARY,
        handle_color: text_color,
        background: Background::Color(background),
        border: Border {
            color: app_colors::BUTTON_BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
    }
}

pub(crate) fn pick_list_menu(_theme: &iced::Theme) -> iced::overlay::menu::Style {
    iced::overlay::menu::Style {
        background: Background::Color(app_colors::BUTTON_BG),
        border: Border {
            color: app_colors::BUTTON_BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        text_color: app_colors::TEXT,
        selected_text_color: app_colors::TEXT,
        selected_background: Background::Color(app_colors::RIPPLE_ON_GRAY),
        shadow: iced::Shadow::default(),
    }
}
