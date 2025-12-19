use crate::app::{Message, PlayerViewState, RenderStatus, Screen};
use crate::ui::range_slider::range_slider;
use crate::ui::selection_canvas::{CropSelection, SelectionCanvas};
use crate::utils::format_timestamp;
use crate::video::{
    CompressionParams, VideoDimensions, build_compression_command, build_ffmpeg_command,
};
use iced::widget::text::LineHeight;
use iced::widget::{
    button, canvas, column, container, image, mouse_area, rich_text, row, space, span, stack, svg,
    text, text_input,
};
use iced::{Alignment, Element, Font, Length};

pub fn editor_screen<'a>(state: &'a PlayerViewState) -> Element<'a, Message> {
    column![
        editor_header(&state.video_path),
        frame_display(
            state.frame_handle.as_ref(),
            state.crop_selection,
            state.preview_width,
            state.preview_height,
        ),
        timeline(
            state.timestamp,
            state.duration,
            state.trim_start,
            state.trim_end,
            state.is_streaming,
        ),
        command_section(state),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
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

fn white_svg(data: &'static [u8], size: u32) -> iced::widget::Svg<'static> {
    svg(svg::Handle::from_memory(data))
        .width(size)
        .height(size)
        .style(|_, _| svg::Style {
            color: Some(iced::Color::WHITE),
        })
}

fn editor_header(video_path: &std::path::Path) -> Element<'_, Message> {
    row![
        button(white_svg(crate::icons::ARROW_SVG, 20))
            .on_press(Message::NavigateTo(Screen::Home))
            .padding(10)
            .style(icon_button_style),
        button(white_svg(crate::icons::FOLDER_SVG, 20))
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
    frame_handle: Option<&'a iced::advanced::image::Handle>,
    crop_selection: Option<CropSelection>,
    preview_width: f32,
    preview_height: f32,
) -> iced::widget::Container<'a, Message> {
    let style = |_theme: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(iced::Color::BLACK)),
        ..Default::default()
    };

    if let Some(handle) = frame_handle {
        container(stack![
            image(handle.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(iced::ContentFit::Contain),
            canvas(SelectionCanvas {
                selection: crop_selection,
                preview_width,
                preview_height,
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

    column![
        if duration > 0.0 {
            range_slider(0.0, duration, trim_start, trim_end, timestamp)
        } else {
            range_slider(0.0, 1.0, 0.0, 1.0, 0.0)
        },
        row![
            button(white_svg(icon, 16))
                .on_press(msg)
                .padding(8)
                .style(|_theme, status| {
                    let bg = match status {
                        button::Status::Hovered => iced::color!(0x414449),
                        button::Status::Pressed => iced::color!(0x292B2D),
                        _ => iced::color!(0x303030),
                    };
                    button::Style {
                        background: Some(iced::Background::Color(bg)),
                        border: iced::Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }),
            text(format_timestamp(timestamp))
                .size(14)
                .color(iced::Color::WHITE),
            text("/").size(14),
            text(format_timestamp(duration))
                .size(14)
                .color(iced::Color::from_rgb(0.7, 0.7, 0.7))
        ]
        .spacing(12)
        .align_y(Alignment::Center)
    ]
    .spacing(8)
    .padding(20)
    .width(Length::Fill)
    .into()
}

fn render_status_row<'a>(status: &'a RenderStatus, target_size_mb: &str) -> Element<'a, Message> {
    match status {
        RenderStatus::Idle => row![].into(),
        RenderStatus::Rendering => {
            let msg = if target_size_mb.is_empty() {
                "Rendering..."
            } else {
                "Rendering and compressing..."
            };
            row![text(msg).size(12).color(iced::Color::WHITE)].into()
        }
        RenderStatus::Completed(output_path) => {
            let green = iced::Color::from_rgb(0.4, 0.9, 0.4);
            row![
                text("Done! (").size(12).color(green),
                link_button("Open", Message::OpenFile(output_path.clone())),
                text(", ").size(12).color(green),
                link_button("Open Folder", Message::OpenParentDir(output_path.clone())),
                text(" or ").size(12).color(green),
                link_button("Delete", Message::DeleteFile(output_path.clone())),
                text(")").size(12).color(green),
            ]
            .into()
        }
        RenderStatus::Failed(err) => row![
            text(format!("Failed: {}", err))
                .size(12)
                .color(iced::Color::from_rgb(1.0, 0.4, 0.4))
        ]
        .into(),
    }
}

fn command_section(state: &PlayerViewState) -> Element<'_, Message> {
    let gray = iced::Color::from_rgb(0.6, 0.6, 0.6);
    let cmd_gray = iced::Color::from_rgb(0.7, 0.7, 0.7);

    let dimen = VideoDimensions {
        video_width: state.video_width,
        video_height: state.video_height,
        preview_width: state.preview_width,
        preview_height: state.preview_height,
    };

    let target_size: Option<f64> = state
        .target_size_mb
        .trim()
        .parse()
        .ok()
        .filter(|&s: &f64| s > 0.0);

    let (cmd_full, cmd_masked) = if let Some(target_size_mb) = target_size {
        let params = CompressionParams {
            target_size_mb,
            duration_sec: state.duration,
            audio_bitrate_bps: state.audio_bitrate_bps,
        };
        match build_compression_command(
            &state.video_path,
            &params,
            state.crop_selection.as_ref(),
            state.trim_start,
            state.trim_end,
            &dimen,
        ) {
            Ok(cmd) => (cmd.preview_full, cmd.preview_masked),
            Err(e) => {
                let err_msg = format!("Error: {}", e);
                (err_msg.clone(), err_msg)
            }
        }
    } else {
        let cmd = build_ffmpeg_command(
            &state.video_path,
            state.crop_selection.as_ref(),
            state.trim_start,
            state.trim_end,
            state.duration,
            &dimen,
        );
        (cmd.full, cmd.masked)
    };

    let command_header = row![
        text("FFmpeg Command (").size(12).color(gray),
        link_button("Copy", Message::CopyCommand(cmd_full.clone())),
        text(" or ").size(12).color(gray),
        link_button("Run", Message::RunCommand),
        text(")").size(12).color(gray),
        space::horizontal(),
        text("Compress to: ").size(12).color(gray),
        text_input("", &state.target_size_mb)
            .on_input(Message::TargetSizeChanged)
            .width(Length::Fixed(50.0))
            .padding(4)
            .size(12),
        text(format!(" MB (source: {:.1} MB)", state.video_file_size_mb))
            .size(12)
            .color(gray),
    ]
    .align_y(Alignment::Center)
    .spacing(0);

    let italic = Font {
        style: iced::font::Style::Italic,
        ..Font::default()
    };

    let command_text: iced::widget::text::Rich<'_, (), Message> = if state.is_command_hovered {
        rich_text::<(), Message, _, _>(vec![span(cmd_full).size(12).color(cmd_gray)])
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
            .line_height(LineHeight::Relative(1.5))
            .height(Length::Fixed(54.0))
    } else if cmd_masked.contains("{input}") && cmd_masked.contains("{output}") {
        let parts: Vec<&str> = cmd_masked.split("{input}").collect();
        let before_input = parts[0];
        let after_input_parts: Vec<&str> = parts[1].split("{output}").collect();
        let between = after_input_parts[0];
        let after_output = after_input_parts.get(1).unwrap_or(&"");

        rich_text::<(), Message, _, _>(vec![
            span(before_input.to_string()).size(12).color(cmd_gray),
            span("input")
                .size(12)
                .color(iced::Color::from_rgb(0.4, 0.9, 0.5))
                .font(italic),
            span(between.to_string()).size(12).color(cmd_gray),
            span("output")
                .size(12)
                .color(iced::Color::from_rgb(0.5, 0.7, 1.0))
                .font(italic),
            span(after_output.to_string()).size(12).color(cmd_gray),
        ])
        .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
        .line_height(LineHeight::Relative(1.5))
        .height(Length::Fixed(54.0))
    } else {
        rich_text::<(), Message, _, _>(vec![span(cmd_masked).size(12).color(cmd_gray)])
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
            .line_height(LineHeight::Relative(1.5))
            .height(Length::Fixed(54.0))
    };

    column![
        command_header,
        mouse_area(
            container(command_text)
                .padding(10)
                .width(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgb(
                        0.1, 0.1, 0.1,
                    ))),
                    border: iced::Border {
                        color: iced::Color::from_rgb(0.3, 0.3, 0.3),
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                }),
        )
        .on_enter(Message::CommandPreviewHovered(true))
        .on_exit(Message::CommandPreviewHovered(false)),
        render_status_row(&state.render_status, &state.target_size_mb)
    ]
    .spacing(8)
    .padding(20)
    .width(Length::Fill)
    .into()
}
