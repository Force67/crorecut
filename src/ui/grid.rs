use crate::app::Message;
use crate::video::VideoMetadata;
use iced::widget::{column, container, image, row, scrollable, text};
use iced::{Alignment, Element, Length};

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
}

pub fn video_grid_item(video: &VideoItem) -> Element<'_, Message> {
    let thumbnail_content = match &video.thumbnail_state {
        ThumbnailState::Pending => container(text("...").size(40))
            .center_x(Length::Fill)
            .center_y(Length::Fixed(180.0))
            .width(Length::Fill)
            .height(Length::Fixed(180.0))
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.2, 0.2, 0.2,
                ))),
                ..Default::default()
            }),

        ThumbnailState::Generating => container(text("...").size(40))
            .center_x(Length::Fill)
            .center_y(Length::Fixed(180.0))
            .width(Length::Fill)
            .height(Length::Fixed(180.0))
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.25, 0.25, 0.25,
                ))),
                ..Default::default()
            }),

        ThumbnailState::Ready(handle) => container(
            image(handle.clone())
                .width(Length::Fill)
                .height(Length::Fixed(180.0))
                .content_fit(iced::ContentFit::Cover),
        )
        .width(Length::Fill)
        .height(Length::Fixed(180.0)),

        ThumbnailState::Failed(_) => container(text("X").size(40))
            .center_x(Length::Fill)
            .center_y(Length::Fixed(180.0))
            .width(Length::Fill)
            .height(Length::Fixed(180.0))
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

    column![thumbnail_content, filename_label, dir_label]
        .spacing(8)
        .width(Length::Fill)
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

    let rows_content: Vec<Element<_>> = videos
        .chunks(3)
        .map(|chunk| {
            let row_items: Vec<Element<_>> = chunk
                .iter()
                .map(|video| {
                    container(video_grid_item(video))
                        .width(Length::FillPortion(1))
                        .into()
                })
                .chain(std::iter::repeat_with(|| {
                    container("").width(Length::FillPortion(1)).into()
                }))
                .take(3)
                .collect();

            row(row_items).spacing(20).width(Length::Fill).into()
        })
        .collect();

    let grid_column = column(rows_content)
        .spacing(20)
        .padding(20)
        .width(Length::Fill);

    scrollable(grid_column).into()
}
