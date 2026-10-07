use iced::widget::text::Span;
use iced::widget::{
    button, column, container, iced as iced_logo, image, mouse_area, opaque, pin, rich_text, row,
    space, span, stack, text, text_input,
};
use iced::{Alignment, Background, Color, Font, Length, Widget};

use super::{ABOUT_ICON_SIZE, Message, Section, Settings};
use crate::ui::color as app_colors;
use crate::ui::{corner_dots, crorecut_button, styles};
use crate::video::FfmpegAvailability;

const SIDEBAR_WIDTH: f32 = 150.0;
const SIDEBAR_BUTTON_HEIGHT: f32 = 48.0;
const SIDEBAR_BUTTON_SPACING: f32 = 4.0;
const INDICATOR_WIDTH: f32 = 2.0;
const INDICATOR_HEIGHT: f32 = 24.0;
const INDICATOR_BUTTON_GAP: f32 = 1.0;
const SIDEBAR_ROW_SPAN: f32 = SIDEBAR_BUTTON_HEIGHT + SIDEBAR_BUTTON_SPACING;
const INDICATOR_Y_OFFSET: f32 = (SIDEBAR_BUTTON_HEIGHT - INDICATOR_HEIGHT) / 2.0;

const GITHUB_URL: &str = "https://github.com/miredirex/crorecut";
const ISSUES_URL: &str = "https://github.com/miredirex/crorecut/issues/new";
const LICENSE_URL: &str = "https://github.com/miredirex/crorecut/blob/main/LICENSE";

impl Settings {
    pub fn view(&self) -> impl Widget<Message> + '_ {
        let sidebar_buttons = column![
            sidebar_button("General", Section::General),
            sidebar_button("About", Section::About),
        ]
        .spacing(SIDEBAR_BUTTON_SPACING)
        .width(SIDEBAR_WIDTH);

        let sidebar = container(
            stack![
                sidebar_buttons,
                pin(sidebar_indicator()).x(0.0).y(self.indicator_y()),
            ]
            .width(SIDEBAR_WIDTH),
        )
        .height(Length::Fill)
        .padding(18)
        .style(|_| container::Style {
            background: Some(Background::Color(app_colors::DARK_GRAY)),
            ..Default::default()
        });

        let content = match self.active_section {
            Section::General => self.general_view().boxed(),
            Section::About => self.about_view().boxed(),
        };

        let content = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(18);

        let body = row![sidebar, content]
            .height(Length::Fill)
            .width(Length::Fill);

        container(stack![body, corner_dots()])
            .width(720)
            .height(420)
            .style(|_| container::Style {
                background: Some(Background::Color(app_colors::BUTTON_BG)),
                ..Default::default()
            })
            .padding(2)
    }

    fn general_view(&self) -> impl Widget<Message> + '_ {
        column![
            settings_input(
                "Default directory",
                None,
                "Default videos directory",
                &self.default_directory,
                Message::DefaultDirectoryChanged,
            ),
            settings_input(
                "FFmpeg path",
                self.ffmpeg_availability.as_ref().map(ffmpeg_status),
                "Leave empty to use FFmpeg from PATH",
                &self.ffmpeg_path,
                Message::FfmpegPathChanged,
            ),
        ]
        .spacing(18)
        .width(Length::Fill)
    }

    fn about_view(&self) -> impl Widget<Message> + '_ {
        let header = column![
            image::<image::Handle>(self.app_icon.clone())
                .width(ABOUT_ICON_SIZE)
                .height(ABOUT_ICON_SIZE),
            space().height(6),
            text("Crorecut").size(24).color(app_colors::TEXT),
            text(format!("v{}", env!("CARGO_PKG_VERSION")))
                .size(14)
                .font(Font::MONOSPACE)
                .color(app_colors::TEXT_SECONDARY),
            space().height(6),
            text("Minimalistic video editor powered by FFmpeg")
                .size(14)
                .color(app_colors::TEXT_SECONDARY),
            space().height(4),
            row![
                link_button("GitHub", GITHUB_URL),
                link_separator(),
                link_button("Report an issue", ISSUES_URL),
                link_separator(),
                link_button("License", LICENSE_URL),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            space().height(12),
            text(build_info())
                .size(12)
                .font(Font::MONOSPACE)
                .color(app_colors::TEXT_SECONDARY),
        ]
        .spacing(4)
        .align_x(Alignment::Center);

        column![
            space().height(Length::FillPortion(1)),
            header,
            space().height(Length::FillPortion(2)),
            row![
                text("Made with").size(13).color(app_colors::TEXT_SECONDARY),
                iced_logo(16),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
    }

    fn indicator_y(&self) -> f32 {
        INDICATOR_Y_OFFSET + self.indicator_index() * SIDEBAR_ROW_SPAN
    }
}

pub fn modal_overlay<'a, Message>(
    content: impl Widget<Message> + 'a,
    on_blur: Message,
) -> impl Widget<Message> + 'a
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
}

fn sidebar_button(label: &str, section: Section) -> impl Widget<Message> + '_ {
    let button = crorecut_button(
        None,
        label.to_string(),
        None,
        Some(Message::SelectSection(section)),
    )
    .width(Length::Fill)
    .into_widget();

    row![
        space().width(INDICATOR_WIDTH),
        space().width(INDICATOR_BUTTON_GAP),
        button
    ]
    .width(Length::Fill)
    .align_y(Alignment::Center)
    .height(SIDEBAR_BUTTON_HEIGHT)
}

fn sidebar_indicator() -> impl Widget<Message> {
    container("")
        .width(INDICATOR_WIDTH)
        .height(INDICATOR_HEIGHT)
        .style(|_| container::Style {
            background: Some(Background::Color(app_colors::LIGHT_GRAY)),
            ..Default::default()
        })
}

fn link_button(label: &'static str, url: &'static str) -> impl Widget<Message> {
    button(text(label).size(13))
        .on_press(Message::OpenUrl(url))
        .style(styles::text_link_button)
        .padding(0)
}

fn link_separator() -> impl Widget<Message> {
    text("·").size(13).color(app_colors::TEXT_SECONDARY)
}

/// e.g. "560227b · 7 Oct 2026 · windows-x86_64"
fn build_info() -> String {
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    [
        option_env!("CRORECUT_GIT_HASH"),
        option_env!("CRORECUT_GIT_DATE"),
        Some(platform.as_str()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ")
}

fn ffmpeg_status(availability: &FfmpegAvailability) -> Span<'static> {
    match availability {
        FfmpegAvailability::Detected { version } => {
            span(format!("✓ detected (version {version})")).color(app_colors::FIR_GREEN)
        }
        FfmpegAvailability::Missing { ffmpeg, ffprobe } => {
            let missing = match (ffmpeg, ffprobe) {
                (true, false) => "ffmpeg ",
                (false, true) => "ffprobe ",
                _ => "",
            };
            span(format!("✗ {missing}not found in PATH")).color(app_colors::DANGER_RED)
        }
    }
}

fn settings_input<'a>(
    label: &'a str,
    status: Option<Span<'a>>,
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> impl Widget<Message> + 'a {
    let mut label_spans = vec![span(label).color(app_colors::TEXT_SECONDARY)];
    if let Some(status) = status {
        label_spans.push(span(" · ").color(app_colors::TEXT_SECONDARY));
        label_spans.push(status);
    }

    column![
        rich_text::<(), Message, _>(label_spans).size(14),
        text_input(placeholder, value)
            .on_input(on_input)
            .padding(8)
            .font(Font::MONOSPACE)
            .style(styles::text_input)
            .width(Length::Fill),
    ]
    .spacing(6)
    .align_x(Alignment::Start)
    .width(Length::Fill)
}
