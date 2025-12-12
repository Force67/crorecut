use crate::app::Message;
use crate::video::VideoMetadata;
use iced::widget::{button, column, container, grid, image, row, scrollable, stack, text};
use iced::{Alignment, Border, Element, Length};

use crate::utils::format_duration_short;

#[derive(Debug, Clone)]
pub enum ThumbnailState {
    Pending,
    Generating,
    Ready(iced::advanced::image::Handle),
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct VideoItem {
    pub metadata: VideoMetadata,
    pub thumbnail_state: ThumbnailState,
    pub duration: Option<f64>,
}

pub fn video_grid_item(video: &VideoItem) -> Element<'_, Message> {
    let thumbnail_content = match &video.thumbnail_state {
        ThumbnailState::Pending => container(text("...").size(40))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.2, 0.2, 0.2,
                ))),
                ..Default::default()
            }),

        ThumbnailState::Generating => container(text("...").size(40))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.25, 0.25, 0.25,
                ))),
                ..Default::default()
            }),

        ThumbnailState::Ready(handle) => {
            let thumbnail_image = image(handle.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(iced::ContentFit::Cover);

            // Duration badge overlay (bottom-right corner)
            let duration_badge: Element<'_, Message> = if let Some(duration) = video.duration {
                container(
                    container(
                        text(format_duration_short(duration))
                            .size(11)
                            .color(iced::Color::WHITE),
                    )
                    .padding([2, 4])
                    .style(|_theme| container::Style {
                        background: Some(iced::Background::Color(iced::Color::from_rgba(
                            0.0, 0.0, 0.0, 0.7,
                        ))),
                        border: Border {
                            radius: 2.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::End)
                .align_y(Alignment::End)
                .padding(4)
                .into()
            } else {
                container(row![])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            };

            container(stack![thumbnail_image, duration_badge])
                .width(Length::Fill)
                .height(Length::Fill)
        }

        ThumbnailState::Failed(error) => container(
            column![text("X").size(40), text(error).size(10)]
                .align_x(Alignment::Center)
                .spacing(4),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.3, 0.1, 0.1,
            ))),
            ..Default::default()
        }),
    };

    let filename_label = text(&video.metadata.file_name)
        .size(12)
        .color(iced::Color::WHITE);
    let dir_label = text(&video.metadata.parent_dir_name)
        .size(12)
        .color(iced::Color::from_rgb(0.6, 0.6, 0.6));

    let content = column![thumbnail_content, filename_label, dir_label]
        .spacing(4)
        .width(Length::Fill)
        .height(Length::Fill);

    let video_path = video.metadata.path.clone();

    // Wrap in button for click handling and hover effects
    button(content)
        .on_press(Message::VideoSelected(video_path))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .style(|_theme, status| {
            let base_color = iced::Color::from_rgb(0.15, 0.15, 0.15);
            let hover_color = iced::Color::from_rgb(0.25, 0.25, 0.3);
            let pressed_color = iced::Color::from_rgb(0.2, 0.2, 0.25);

            button::Style {
                background: Some(iced::Background::Color(match status {
                    button::Status::Hovered => hover_color,
                    button::Status::Pressed => pressed_color,
                    _ => base_color,
                })),
                border: Border {
                    color: match status {
                        button::Status::Hovered => iced::Color::from_rgb(0.4, 0.5, 0.7),
                        button::Status::Pressed => iced::Color::from_rgb(0.3, 0.4, 0.6),
                        _ => iced::Color::from_rgb(0.2, 0.2, 0.2),
                    },
                    width: 2.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

pub fn grid_view(videos: &[VideoItem]) -> Element<'_, Message> {
    if videos.is_empty() {
        return container(
            column![
                text("No videos found").size(20),
                text("Place video files in your Videos folder").size(14)
            ]
            .spacing(10)
            .align_x(Alignment::Center),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into();
    }

    let video_grid = grid::Grid::with_children(videos.iter().map(video_grid_item))
        .columns(3)
        .spacing(12)
        .height(grid::Sizing::AspectRatio(1.35)); // Slightly taller than 16:9 to fit text

    scrollable(container(video_grid).padding(20).width(Length::Fill)).into()
}
