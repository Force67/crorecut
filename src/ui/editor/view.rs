use iced::widget::text::LineHeight;
use iced::widget::{
    button, canvas, column, container, image, mouse_area, pick_list, pin, rich_text, row, space,
    span, stack, svg, text, text_input,
};
use iced::{Alignment, Element, Font, Length};

use super::{
    Allocation, Editor, Message, RenderStatus, Resolution, SelectionCanvas, trim_timeline,
};
use crate::ui::color as app_colors;
use crate::ui::{corner_dots, crorecut_button, crorecut_checkbox, positioned_mouse_area};
use crate::utils::{CropSelection, format_timestamp};

const COMMAND_PREVIEW_FONT_SIZE: f32 = 12.0;

/// Wrapper for resolution picker that includes "Original" option
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionOption {
    Original,
    Scale(Resolution),
}

impl ResolutionOption {
    fn options_for(source_height: u32) -> Vec<ResolutionOption> {
        let mut options = vec![ResolutionOption::Original];
        options.extend(
            Resolution::available_for(source_height)
                .into_iter()
                .rev()
                .map(ResolutionOption::Scale),
        );
        options
    }

    fn from_resolution(res: Option<Resolution>) -> Self {
        match res {
            Some(r) => ResolutionOption::Scale(r),
            None => ResolutionOption::Original,
        }
    }

    fn to_resolution(self) -> Option<Resolution> {
        match self {
            ResolutionOption::Original => None,
            ResolutionOption::Scale(r) => Some(r),
        }
    }
}

impl std::fmt::Display for ResolutionOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolutionOption::Original => write!(f, "Original"),
            ResolutionOption::Scale(r) => write!(f, "{}", r),
        }
    }
}

impl Editor {
    pub fn view(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = column![
            editor_header(&self.video_path),
            frame_display(
                self.frame_allocation.as_ref(),
                self.crop_selection,
                self.preview_width,
                self.preview_height,
                self.is_crop_interaction_active(),
            ),
            timeline(
                self.timestamp,
                self.duration,
                self.trim_start,
                self.trim_end,
                self.is_streaming,
            ),
            container(command_preview_field(self))
                .padding([0, 20])
                .width(Length::Fill),
            container(render_status_row(&self.render_status))
                .padding([4, 20])
                .width(Length::Fill),
            render_params_section(self),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

        stack![content, toast_layer(self.copy_toast())]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn icon_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => iced::color!(0x414449),
        button::Status::Pressed => iced::color!(0x292B2D),
        _ => iced::color!(0x202020),
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        border: iced::Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn link_button_style(_theme: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: iced::color!(0x5197ED),
        ..Default::default()
    }
}

fn link_button(label: &str, message: Message) -> iced::widget::Button<'_, Message> {
    button(text(label).size(12))
        .on_press(message)
        .style(link_button_style)
        .padding(0)
}

fn square_svg(data: &'static [u8], size: u32) -> iced::widget::Svg<'static> {
    svg(svg::Handle::from_memory(data)).width(size).height(size)
}

fn playback_button(icon: &'static [u8], message: Message) -> Element<'static, Message> {
    button(stack![
        container(square_svg(icon, 24))
            .center_x(Length::Fill)
            .center_y(Length::Fill),
        corner_dots(),
    ])
    .on_press(message)
    .width(Length::Fixed(36.0))
    .height(Length::Fixed(36.0))
    .padding(2)
    .style(|_theme, status| {
        let background = match status {
            button::Status::Pressed => app_colors::COMMAND_PREVIEW_BG,
            _ => app_colors::BUTTON_BG,
        };

        button::Style {
            background: Some(iced::Background::Color(background)),
            border: iced::Border {
                color: app_colors::BUTTON_BORDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

fn compress_input_style(
    _theme: &iced::Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let background = match status {
        iced::widget::text_input::Status::Hovered
        | iced::widget::text_input::Status::Focused { is_hovered: true } => {
            app_colors::COMMAND_PREVIEW_BG
        }
        _ => app_colors::BUTTON_BG,
    };

    iced::widget::text_input::Style {
        background: iced::Background::Color(background),
        border: iced::Border {
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

fn editor_header(video_path: &std::path::Path) -> Element<'_, Message> {
    row![
        button(square_svg(crate::icons::ARROW_SVG, 22))
            .on_press(Message::GoBack)
            .padding(10)
            .style(icon_button_style),
        button(square_svg(crate::icons::FOLDER_SVG, 22))
            .on_press(Message::OpenParentDir(video_path.to_path_buf()))
            .padding(10)
            .style(icon_button_style),
        text(
            video_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
        )
        .size(20)
    ]
    .spacing(10)
    .padding(10)
    .align_y(Alignment::Center)
    .into()
}

fn frame_display<'a>(
    frame_allocation: Option<&'a Allocation>,
    crop_selection: Option<CropSelection>,
    preview_width: f32,
    preview_height: f32,
    is_crop_interaction_active: bool,
) -> iced::widget::Container<'a, Message> {
    let style = |_theme: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(iced::Color::BLACK)),
        ..Default::default()
    };

    if let Some(allocation) = frame_allocation {
        container(stack![
            image(allocation.handle().clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(iced::ContentFit::Contain),
            canvas(SelectionCanvas {
                selection: crop_selection,
                preview_width,
                preview_height,
                is_dragging: is_crop_interaction_active,
            })
            .width(Length::Fill)
            .height(Length::Fill)
        ])
        .width(Length::Fill)
        .height(Length::FillPortion(4))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(style)
    } else {
        container(
            text("Loading video...")
                .size(20)
                .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
        )
        .width(Length::Fill)
        .height(Length::FillPortion(4))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(style)
    }
}

fn timeline(
    timestamp: f64,
    duration: f64,
    trim_start: f64,
    trim_end: f64,
    is_streaming: bool,
) -> Element<'static, Message> {
    let (icon, msg) = if is_streaming {
        (crate::icons::PAUSE_SVG, Message::StopPlayback)
    } else {
        (crate::icons::PLAY_SVG, Message::StartPlayback)
    };

    row![
        playback_button(icon, msg),
        if duration > 0.0 {
            trim_timeline(0.0, duration, trim_start, trim_end, timestamp)
        } else {
            trim_timeline(0.0, 1.0, 0.0, 1.0, 0.0)
        },
        row![
            text(format_timestamp(timestamp))
                .size(14)
                .color(app_colors::TEXT),
            text("/").size(14).color(app_colors::TEXT.scale_alpha(0.5)),
            text(format_timestamp(duration))
                .size(14)
                .color(app_colors::TEXT.scale_alpha(0.5)),
        ]
        .spacing(2)
        .align_y(Alignment::Center)
    ]
    .spacing(10)
    .padding([12, 20])
    .width(Length::Fill)
    .align_y(Alignment::Center)
    .into()
}

fn render_status_row<'a>(status: &'a RenderStatus) -> Element<'a, Message> {
    match status {
        RenderStatus::Idle | RenderStatus::Rendering => row![].into(),
        RenderStatus::Completed(output_path) => row![
            text("Done! (").size(12).color(app_colors::FIR_GREEN),
            link_button("Open", Message::OpenFile(output_path.clone())),
            text(", ").size(12).color(app_colors::FIR_GREEN),
            link_button("Open Folder", Message::OpenParentDir(output_path.clone())),
            text(" or ").size(12).color(app_colors::FIR_GREEN),
            link_button("Delete", Message::DeleteFile(output_path.clone())),
            text(")").size(12).color(app_colors::FIR_GREEN),
        ]
        .into(),
        RenderStatus::Failed(err) => row![
            text(format!("Failed: {}", err))
                .size(12)
                .color(iced::Color::from_rgb(1.0, 0.4, 0.4))
        ]
        .into(),
    }
}

fn render_params_section(state: &Editor) -> Element<'_, Message> {
    let will_compress = !state.target_size_mb.is_empty();
    let render_label = match (&state.render_status, will_compress) {
        (RenderStatus::Rendering, true) => "Rendering and compressing…",
        (RenderStatus::Rendering, false) => "Rendering video…",
        (_, true) => "Render and compress…",
        (_, false) => "Render the video…",
    };

    row![
        command_header(state),
        space::horizontal().width(Length::Fill),
        positioned_mouse_area(
            crorecut_button(
                Some(crate::icons::COPY_SVG),
                "Copy Command".into(),
                None,
                Some(Message::CopyCommand),
            ),
            Message::CopyFeedbackAnchorChanged,
        ),
        crorecut_button(
            Some(crate::icons::FFMPEG_SVG),
            render_label.into(),
            state.render_progress.as_ref().map(|p| {
                if let Some(status) = &p.status {
                    status.clone()
                } else {
                    let mins = (p.out_time / 60.0) as u32;
                    let secs = (p.out_time % 60.0) as u32;
                    format!(
                        "frame {}, rendered {:02}:{:02} @ {:.2}x",
                        p.frame, mins, secs, p.speed
                    )
                }
            }),
            match state.render_status {
                RenderStatus::Rendering => None,
                _ => Some(Message::RunCommand),
            },
        ),
    ]
    .align_y(Alignment::Center)
    .spacing(12)
    .padding([10, 20])
    .width(Length::Fill)
    .into()
}

fn toast_layer(toast: Option<&super::CopyToast>) -> Element<'static, Message> {
    const BASE_OFFSET_Y: f32 = 36.0;
    const OFFSET_X: f32 = 12.0;
    const TRAVEL_Y: f32 = 24.0;

    let Some(toast) = toast else {
        return space().width(Length::Fill).height(Length::Fill).into();
    };

    let position = toast.position;
    let progress = toast.progress();
    let eased = 1.0 - (1.0 - progress).powi(3);
    let alpha = 1.0 - progress;
    let y = (position.y - BASE_OFFSET_Y - TRAVEL_Y * eased).max(0.0);

    pin(copy_toast_widget(alpha))
        .x(position.x + OFFSET_X)
        .y(y)
        .into()
}

fn copy_toast_widget(alpha: f32) -> Element<'static, Message> {
    container(
        text("Copied!")
            .size(13)
            .color(app_colors::TEXT.scale_alpha(alpha)),
    )
    .padding([6, 10])
    .style(move |_theme| container::Style {
        background: Some(iced::Background::Color(
            app_colors::BUTTON_BG.scale_alpha(alpha),
        )),
        border: iced::Border {
            color: app_colors::BUTTON_BORDER.scale_alpha(alpha),
            width: 1.0,
            radius: 2.0.into(),
        },
        ..Default::default()
    })
    .into()
}

fn command_preview_field(state: &Editor) -> Element<'_, Message> {
    mouse_area(
        container(stack![
            container(command_preview_text(state))
                .padding(6)
                .width(Length::Fill),
            corner_dots(),
        ])
        .padding(2)
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(app_colors::COMMAND_PREVIEW_BG)),
            border: iced::Border {
                color: app_colors::BUTTON_BORDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        }),
    )
    .on_enter(Message::CommandPreviewHovered(true))
    .on_exit(Message::CommandPreviewHovered(false))
    .into()
}

fn command_header(state: &Editor) -> iced::widget::Row<'_, Message> {
    let resolution_options = ResolutionOption::options_for(state.video_height);
    let selected = ResolutionOption::from_resolution(state.target_resolution);

    row![
        crorecut_checkbox(
            state.mute_audio,
            "Mute",
            Message::MuteAudioToggled(!state.mute_audio)
        ),
        space::horizontal().width(12),
        text("Resolution:").size(12).color(app_colors::TEXT),
        space::horizontal().width(4),
        pick_list(
            Some(selected),
            resolution_options,
            |opt: &ResolutionOption| opt.to_string()
        )
        .on_select(|opt| Message::ResolutionChanged(opt.to_resolution()))
        .text_size(12)
        .padding(4),
        space::horizontal().width(8),
        text("Compress to:").size(12).color(app_colors::TEXT),
        space::horizontal().width(4),
        text_input("", &state.target_size_mb)
            .on_input(Message::TargetSizeChanged)
            .style(compress_input_style)
            .width(Length::Fixed(56.0))
            .padding([6, 4])
            .size(12)
            .line_height(LineHeight::Absolute(20.0.into())),
        space::horizontal().width(4),
        text(format!("MB (source: {:.1} MB)", state.video_file_size_mb))
            .size(12)
            .color(app_colors::TEXT),
    ]
    .align_y(Alignment::Center)
    .spacing(0)
}

fn command_preview_text(state: &Editor) -> iced::widget::text::Rich<'_, (), Message> {
    let build_rich_text = |spans| -> iced::widget::text::Rich<'_, (), Message> {
        rich_text::<(), Message, _, _>(spans)
            .font(Font::MONOSPACE)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
            .line_height(LineHeight::Relative(1.5))
            .height(Length::Fixed(54.0))
    };

    if state.is_command_hovered {
        return build_rich_text(vec![
            span(&state.command_preview_full)
                .size(COMMAND_PREVIEW_FONT_SIZE)
                .color(app_colors::COMMAND_PREVIEW_TEXT),
        ]);
    }

    let cmd_masked = &state.command_preview_masked;
    if !cmd_masked.contains("{input}") || !cmd_masked.contains("{output}") {
        return build_rich_text(vec![
            span(cmd_masked)
                .size(COMMAND_PREVIEW_FONT_SIZE)
                .color(app_colors::COMMAND_PREVIEW_TEXT),
        ]);
    }

    // Highlight {input} and {output} placeholders
    let parts: Vec<&str> = cmd_masked.split("{input}").collect();
    let before_input = parts[0];
    let after_input_parts: Vec<&str> = parts[1].split("{output}").collect();
    let between = after_input_parts[0];
    let after_output = after_input_parts.get(1).unwrap_or(&"");

    build_rich_text(vec![
        span(before_input.to_string())
            .size(COMMAND_PREVIEW_FONT_SIZE)
            .color(app_colors::COMMAND_PREVIEW_TEXT),
        span("input")
            .size(COMMAND_PREVIEW_FONT_SIZE)
            .color(app_colors::FIR_GREEN),
        span(between.to_string())
            .size(COMMAND_PREVIEW_FONT_SIZE)
            .color(app_colors::COMMAND_PREVIEW_TEXT),
        span("output")
            .size(COMMAND_PREVIEW_FONT_SIZE)
            .color(app_colors::COMMAND_PREVIEW_OUTPUT),
        span(after_output.to_string())
            .size(COMMAND_PREVIEW_FONT_SIZE)
            .color(app_colors::COMMAND_PREVIEW_TEXT),
    ])
}
