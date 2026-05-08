use iced::widget::{
    column, container, iced as iced_logo, mouse_area, opaque, row, rule, space, stack, text,
    text_input,
};
use iced::{Alignment, Background, Border, Color, Element, Length};

use super::{Message, Section, Settings};
use crate::ui::color as app_colors;
use crate::ui::{corner_dots, crorecut_button, styles};

impl Settings {
    pub fn view(&self) -> Element<'_, Message> {
        let sidebar = column![
            sidebar_button("General", Section::General, self.active_section),
            sidebar_button("About", Section::About, self.active_section),
        ]
        .spacing(4)
        .width(150);

        let divider = rule::vertical(1).style(|_| rule::Style {
            color: app_colors::BUTTON_BORDER,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        });

        let content = match self.active_section {
            Section::General => self.general_view(),
            Section::About => about_view(),
        };

        let body = row![sidebar, divider, content]
            .spacing(18)
            .height(Length::Fill)
            .width(Length::Fill);

        container(stack![container(body).padding(20), corner_dots(),])
            .width(720)
            .height(420)
            .style(|_| container::Style {
                background: Some(Background::Color(app_colors::BUTTON_BG)),
                border: Border {
                    color: app_colors::BUTTON_BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .padding(2)
            .into()
    }

    fn general_view(&self) -> Element<'_, Message> {
        column![
            settings_input(
                "Default directory",
                "Default videos directory",
                &self.default_directory,
                Message::DefaultDirectoryChanged,
            ),
            settings_input(
                "FFmpeg path",
                "(leave empty to use from PATH)",
                &self.ffmpeg_path,
                Message::FfmpegPathChanged,
            ),
        ]
        .spacing(18)
        .width(Length::Fill)
        .into()
    }
}

fn about_view<'a>() -> Element<'a, Message> {
    container(iced_logo(24))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub fn modal_overlay<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    on_blur: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
        opaque(
            mouse_area(
                iced::widget::center(opaque(content)).style(|_| container::Style {
                    background: Some(Background::Color(Color {
                        a: 0.78,
                        ..app_colors::ALMOST_BLACK
                    })),
                    ..Default::default()
            }),
        )
        .on_press(on_blur),
    )
    .into()
}

fn sidebar_button(label: &str, section: Section, active_section: Section) -> Element<'_, Message> {
    let button: Element<'_, Message> = crorecut_button(
        None,
        label.to_string(),
        None,
        Some(Message::SelectSection(section)),
    )
    .width(Length::Fill)
    .into();

    let indicator = container("").width(2).height(24).style(move |_| {
        let background = if section == active_section {
            Some(Background::Color(app_colors::LIGHT_GRAY))
        } else {
            None
        };

        container::Style {
            background,
            ..Default::default()
        }
    });

    row![indicator, space().width(1), button]
        .width(Length::Fill)
        .align_y(Alignment::Center)
        .height(Length::Shrink)
        .into()
}

fn settings_input<'a>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    column![
        text(label).size(12).color(app_colors::TEXT_SECONDARY),
        text_input(placeholder, value)
            .on_input(on_input)
            .padding(8)
            .style(styles::text_input)
            .width(Length::Fill),
    ]
    .spacing(6)
    .align_x(Alignment::Start)
    .width(Length::Fill)
    .into()
}
