use crate::app::{Message, PlayerViewState, RenderStatus, ResizeHandle, Screen};
use crate::ui::range_slider::range_slider;
use crate::ui::selection_canvas::{CropSelection, SelectionCanvas};
use crate::video::{VideoDimensions, build_ffmpeg_command};
use iced::widget::text::LineHeight;
use iced::widget::{
    button, canvas, column, container, image, mouse_area, rich_text, row, span, stack, svg, text,
    text_input,
};
use iced::{Alignment, Element, Font, Length};

/// Represents the transform from canvas space to image space (ContentFit::Contain)
#[derive(Debug, Clone, Copy)]
pub struct ImageTransform {
    pub scale: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub _displayed_width: f32,
    pub _displayed_height: f32,
}

impl ImageTransform {
    /// Calculate the transform for an image displayed with ContentFit::Contain
    pub fn from_contain(
        container_width: f32,
        container_height: f32,
        image_width: f32,
        image_height: f32,
    ) -> Self {
        if image_width <= 0.0 || image_height <= 0.0 {
            return Self {
                scale: 1.0,
                offset_x: 0.0,
                offset_y: 0.0,
                _displayed_width: 0.0,
                _displayed_height: 0.0,
            };
        }

        // Calculate scale to fit within container while maintaining aspect ratio
        let scale = (container_width / image_width).min(container_height / image_height);

        let displayed_width = image_width * scale;
        let displayed_height = image_height * scale;

        // Center the image
        let offset_x = (container_width - displayed_width) / 2.0;
        let offset_y = (container_height - displayed_height) / 2.0;

        Self {
            scale,
            offset_x,
            offset_y,
            _displayed_width: displayed_width,
            _displayed_height: displayed_height,
        }
    }

    /// Transform from canvas space to image space
    pub fn canvas_to_image(&self, canvas_x: f32, canvas_y: f32) -> (f32, f32) {
        let image_x = (canvas_x - self.offset_x) / self.scale;
        let image_y = (canvas_y - self.offset_y) / self.scale;
        (image_x, image_y)
    }

    /// Transform from image space to canvas space
    pub fn image_to_canvas(&self, image_x: f32, image_y: f32) -> (f32, f32) {
        let canvas_x = image_x * self.scale + self.offset_x;
        let canvas_y = image_y * self.scale + self.offset_y;
        (canvas_x, canvas_y)
    }

    /// Clamp coordinates to image bounds (kept for potential future use)
    #[allow(dead_code)]
    pub fn clamp_to_image(
        &self,
        x: f32,
        y: f32,
        image_width: f32,
        image_height: f32,
    ) -> (f32, f32) {
        (x.clamp(0.0, image_width), y.clamp(0.0, image_height))
    }
}

use crate::utils::format_timestamp;

/// Check if coordinates are on a resize handle of the selection
pub fn get_resize_handle(x: f32, y: f32, selection: &CropSelection) -> Option<ResizeHandle> {
    const HANDLE_SIZE: f32 = 10.0;

    let left = selection.x;
    let right = selection.x + selection.width;
    let top = selection.y;
    let bottom = selection.y + selection.height;

    // Check corner handles
    if (x - left).abs() < HANDLE_SIZE && (y - top).abs() < HANDLE_SIZE {
        return Some(ResizeHandle::TopLeft);
    }
    if (x - right).abs() < HANDLE_SIZE && (y - top).abs() < HANDLE_SIZE {
        return Some(ResizeHandle::TopRight);
    }
    if (x - left).abs() < HANDLE_SIZE && (y - bottom).abs() < HANDLE_SIZE {
        return Some(ResizeHandle::BottomLeft);
    }
    if (x - right).abs() < HANDLE_SIZE && (y - bottom).abs() < HANDLE_SIZE {
        return Some(ResizeHandle::BottomRight);
    }

    None
}

/// Check if a point is inside the selection rectangle
pub fn point_in_selection(x: f32, y: f32, selection: &CropSelection) -> bool {
    x >= selection.x
        && x <= selection.x + selection.width
        && y >= selection.y
        && y <= selection.y + selection.height
}

/// Resize a selection based on the handle being dragged
pub fn resize_selection(
    selection: &mut CropSelection,
    handle: ResizeHandle,
    x: f32,
    y: f32,
    original: &CropSelection,
) {
    match handle {
        ResizeHandle::TopLeft => {
            selection.x = x;
            selection.y = y;
            selection.width = (original.x + original.width) - x;
            selection.height = (original.y + original.height) - y;
        }
        ResizeHandle::TopRight => {
            selection.y = y;
            selection.width = x - original.x;
            selection.height = (original.y + original.height) - y;
        }
        ResizeHandle::BottomLeft => {
            selection.x = x;
            selection.width = (original.x + original.width) - x;
            selection.height = y - original.y;
        }
        ResizeHandle::BottomRight => {
            selection.width = x - original.x;
            selection.height = y - original.y;
        }
    }

    // Ensure minimum size and prevent negative dimensions
    if selection.width < 10.0 {
        selection.width = 10.0;
    }
    if selection.height < 10.0 {
        selection.height = 10.0;
    }
}

/// Main editor view for the video player screen
pub fn editor_view<'a>(player_state: &'a PlayerViewState) -> Element<'a, Message> {
    // Back button and header

    let arrow_svg = svg::Handle::from_memory(crate::icons::ARROW_SVG);
    let arrow_icon = svg(arrow_svg)
        .width(20)
        .height(20)
        .style(|_theme, _status| svg::Style {
            color: Some(iced::Color::WHITE),
        });
    let folder_svg = svg::Handle::from_memory(crate::icons::FOLDER_SVG);
    let folder_icon = svg(folder_svg)
        .width(20)
        .height(20)
        .style(|_theme, _status| svg::Style {
            color: Some(iced::Color::WHITE),
        });

    let back_button = button(arrow_icon)
        .on_press(Message::NavigateTo(Screen::Home))
        .padding(10)
        .style(|_theme, status| {
            let background = match status {
                button::Status::Hovered => iced::color!(0x414449),
                button::Status::Pressed => iced::color!(0x292B2D),
                _ => iced::color!(0x202020),
            };
            button::Style {
                background: Some(iced::Background::Color(background)),
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    let folder_button = button(folder_icon)
        .on_press(Message::OpenParentDir(player_state.video_path.clone()))
        .padding(10)
        .style(|_theme, status| {
            let background = match status {
                button::Status::Hovered => iced::color!(0x414449),
                button::Status::Pressed => iced::color!(0x292B2D),
                _ => iced::color!(0x202020),
            };
            button::Style {
                background: Some(iced::Background::Color(background)),
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    let video_name = player_state
        .video_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown");

    let header = row![back_button, folder_button, text(video_name).size(20)]
        .spacing(10)
        .padding(10)
        .align_y(Alignment::Center);

    // Frame display area
    let frame_display = if let Some(ref handle) = player_state.frame_handle {
        let image_widget = image(handle.clone())
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(iced::ContentFit::Contain);

        // Create canvas for selection overlay
        let selection_canvas = canvas(SelectionCanvas {
            selection: player_state.crop_selection,
            preview_width: player_state.preview_width,
            preview_height: player_state.preview_height,
        })
        .width(Length::Fill)
        .height(Length::Fill);

        // Stack image and canvas (canvas handles mouse events)
        let content = stack![image_widget, selection_canvas];

        container(content)
            .width(Length::Fill)
            .height(Length::FillPortion(4))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::BLACK)),
                ..Default::default()
            })
    } else {
        // Placeholder while loading
        container(
            text("Loading video...")
                .size(20)
                .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
        )
        .width(Length::Fill)
        .height(Length::FillPortion(4))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::BLACK)),
            ..Default::default()
        })
    };

    // Timeline controls
    let current_time = text(format_timestamp(player_state.timestamp))
        .size(14)
        .color(iced::Color::WHITE);

    let total_time = text(format_timestamp(player_state.duration))
        .size(14)
        .color(iced::Color::from_rgb(0.7, 0.7, 0.7));

    // Combined range slider with playhead
    let timeline_slider = if player_state.duration > 0.0 {
        range_slider(
            0.0,
            player_state.duration,
            player_state.trim_start,
            player_state.trim_end,
            player_state.timestamp,
        )
    } else {
        range_slider(0.0, 1.0, 0.0, 1.0, 0.0)
    };

    let timeline_controls = column![
        // Combined range slider with trim handles and playhead
        timeline_slider,
        // Time labels
        row![current_time, text("/").size(14), total_time]
            .spacing(12)
            .align_y(Alignment::Center),
    ]
    .spacing(8)
    .padding(20)
    .width(Length::Fill);

    // FFmpeg command display
    let dimen = VideoDimensions {
        video_width: player_state.video_width,
        video_height: player_state.video_height,
        preview_width: player_state.preview_width,
        preview_height: player_state.preview_height,
    };
    let ffmpeg_cmd = build_ffmpeg_command(
        &player_state.video_path,
        player_state.crop_selection.as_ref(),
        player_state.trim_start,
        player_state.trim_end,
        player_state.duration,
        &dimen,
    );

    // Label with inline copy and run links
    let link_color = iced::Color::from_rgb(0.4, 0.7, 1.0);
    let gray = iced::Color::from_rgb(0.6, 0.6, 0.6);

    let copy_link = button(text("Copy").size(12).color(link_color))
        .on_press(Message::CopyCommand(ffmpeg_cmd.full.clone()))
        .padding(0)
        .style(|_theme, _status| button::Style {
            background: None,
            text_color: iced::Color::from_rgb(0.4, 0.7, 1.0),
            ..Default::default()
        });

    let run_link = button(text("Run").size(12).color(link_color))
        .on_press(Message::RunCommand)
        .padding(0)
        .style(|_theme, _status| button::Style {
            background: None,
            text_color: iced::Color::from_rgb(0.4, 0.7, 1.0),
            ..Default::default()
        });

    // Target size input for compression (optional)
    let size_input = text_input("", &player_state.target_size_mb)
        .on_input(Message::TargetSizeChanged)
        .width(Length::Fixed(50.0))
        .padding(4)
        .size(12);

    let command_header = row![
        text("FFmpeg Command (").size(12).color(gray),
        copy_link,
        text(" or ").size(12).color(gray),
        run_link,
        text(")").size(12).color(gray),
        container(row![].width(Length::Fill)), // Spacer
        text("Compress to: ").size(12).color(gray),
        size_input,
        text(format!(
            " MB (source: {:.1} MB)",
            player_state.video_file_size_mb
        ))
        .size(12)
        .color(gray),
    ]
    // .spacing(4)
    .align_y(Alignment::Center);

    // Build rich text - show full command when hovered, masked when not
    let cmd_gray = iced::Color::from_rgb(0.7, 0.7, 0.7);
    let italic = Font {
        style: iced::font::Style::Italic,
        ..Font::default()
    };

    let command_text = if player_state.is_command_hovered {
        // Show full command with actual paths
        rich_text(vec![span(ffmpeg_cmd.full.clone()).size(12).color(cmd_gray)])
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
            .line_height(LineHeight::Relative(1.5))
            .height(Length::Fixed(54.0))
    } else {
        // Show masked command with placeholders
        // Split by {input} and {output}, which always appear in that order
        let parts: Vec<&str> = ffmpeg_cmd.masked.split("{input}").collect();
        let before_input = parts[0];
        let after_input_parts: Vec<&str> = parts[1].split("{output}").collect();
        let between = after_input_parts[0];
        let after_output = after_input_parts.get(1).unwrap_or(&"");

        let spans: Vec<iced::widget::text::Span<'_, Font>> = vec![
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
        ];

        rich_text(spans)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
            .line_height(LineHeight::Relative(1.5))
            .height(Length::Fixed(54.0))
    };

    let command_container = container(command_text)
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
        });

    // Wrap in mouse_area to detect hover
    let command_display = mouse_area(command_container)
        .on_enter(Message::CommandPreviewHovered(true))
        .on_exit(Message::CommandPreviewHovered(false));

    // Render status indicator
    let status_row: Element<'_, Message> = match &player_state.render_status {
        RenderStatus::Idle => row![].into(),
        RenderStatus::Rendering => {
            let msg = if player_state.target_size_mb.is_empty() {
                "Rendering..."
            } else {
                "Rendering and compressing..."
            };
            row![text(msg).size(12).color(iced::Color::WHITE),].into()
        }
        RenderStatus::Completed(output_path) => {
            let open_link = button(text("Open").size(12).color(link_color))
                .on_press(Message::OpenFile(output_path.clone()))
                .padding(0)
                .style(|_theme, _status| button::Style {
                    background: None,
                    text_color: iced::Color::from_rgb(0.4, 0.7, 1.0),
                    ..Default::default()
                });
            let delete_link = button(text("Delete").size(12).color(link_color))
                .on_press(Message::DeleteFile(output_path.clone()))
                .padding(0)
                .style(|_theme, _status| button::Style {
                    background: None,
                    text_color: iced::Color::from_rgb(0.4, 0.7, 1.0),
                    ..Default::default()
                });
            let green = iced::Color::from_rgb(0.4, 0.9, 0.4);
            row![
                text("Done! (").size(12).color(green),
                open_link,
                text(" or ").size(12).color(green),
                delete_link,
                text(")").size(12).color(green),
            ]
            .spacing(0)
            .into()
        }
        RenderStatus::Failed(err) => row![
            text(format!("Failed: {}", err))
                .size(12)
                .color(iced::Color::from_rgb(1.0, 0.4, 0.4)),
        ]
        .into(),
    };

    let command_section = column![command_header, command_display, status_row]
        .spacing(8)
        .padding(20)
        .width(Length::Fill);

    // Main layout
    column![header, frame_display, timeline_controls, command_section]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
