use crate::app::{Message, PlayerViewState, ResizeHandle, Screen};
use crate::ui::range_slider::range_slider;
use crate::ui::selection_canvas::{CropSelection, SelectionCanvas};
use crate::video::build_ffmpeg_command;
use iced::widget::{
    button, canvas, column, container, image, mouse_area, rich_text, row, span, stack, text,
};
use iced::{Alignment, Element, Font, Length};

/// Helper function to format timestamps as HH:MM:SS
fn format_timestamp(seconds: f64) -> String {
    let hours = (seconds / 3600.0).floor() as u32;
    let minutes = ((seconds % 3600.0) / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;
    format!("{:02}:{:02}:{:02}", hours, minutes, secs)
}

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
    let back_button = button(text("← Back"))
        .on_press(Message::NavigateTo(Screen::Home))
        .padding(10);

    let video_name = player_state
        .video_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown");

    let header = row![back_button, text(video_name).size(20)]
        .spacing(20)
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
        })
        .width(Length::Fill)
        .height(Length::Fill);

        // Stack image and overlay
        let content = stack![image_widget, selection_canvas];

        // Wrap in mouse area for interaction
        let interactive_content = mouse_area(content)
            .on_press(Message::CropMousePressed)
            .on_move(|point: iced::Point| Message::CropMouseMoved {
                x: point.x,
                y: point.y,
            })
            .on_release(Message::CropMouseReleased)
            .interaction(iced::mouse::Interaction::Crosshair);

        container(interactive_content)
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
        row![current_time, total_time]
            .spacing(12)
            .align_y(Alignment::Center),
    ]
    .spacing(8)
    .padding(20)
    .width(Length::Fill);

    // FFmpeg command display
    let ffmpeg_cmd = build_ffmpeg_command(
        &player_state.video_path,
        player_state.crop_selection.as_ref(),
        player_state.trim_start,
        player_state.trim_end,
        player_state.duration,
    );

    // Label with inline copy link
    let copy_link = button(
        text("Copy")
            .size(12)
            .color(iced::Color::from_rgb(0.4, 0.7, 1.0)),
    )
    .on_press(Message::CopyCommand(ffmpeg_cmd.full))
    .padding(0)
    .style(|_theme, _status| button::Style {
        background: None,
        text_color: iced::Color::from_rgb(0.4, 0.7, 1.0),
        ..Default::default()
    });

    let command_header = row![
        text("FFmpeg Command (")
            .size(12)
            .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
        copy_link,
        text("):")
            .size(12)
            .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
    ]
    .spacing(0)
    .align_y(Alignment::Center);

    // Build rich text with colored placeholders
    // Split by {input} and {output}, which always appear in that order
    let parts: Vec<&str> = ffmpeg_cmd.masked.split("{input}").collect();
    let before_input = parts[0];
    let after_input_parts: Vec<&str> = parts[1].split("{output}").collect();
    let between = after_input_parts[0];
    let after_output = after_input_parts.get(1).unwrap_or(&"");

    let gray = iced::Color::from_rgb(0.7, 0.7, 0.7);
    let italic = Font {
        style: iced::font::Style::Italic,
        ..Font::default()
    };

    let spans: Vec<iced::widget::text::Span<'_, Font>> = vec![
        span(before_input.to_string()).size(12).color(gray),
        span("input")
            .size(12)
            .color(iced::Color::from_rgb(0.4, 0.9, 0.5))
            .font(italic),
        span(between.to_string()).size(12).color(gray),
        span("output")
            .size(12)
            .color(iced::Color::from_rgb(0.5, 0.7, 1.0))
            .font(italic),
        span(after_output.to_string()).size(12).color(gray),
    ];

    let command_text = rich_text(spans).wrapping(iced::widget::text::Wrapping::WordOrGlyph);

    let command_display = container(command_text)
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

    let command_section = column![command_header, command_display]
        .spacing(4)
        .padding(20)
        .width(Length::Fill);

    // Main layout
    column![header, frame_display, timeline_controls, command_section]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
