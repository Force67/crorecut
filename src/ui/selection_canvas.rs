use iced::widget::canvas::{Frame, Path, Stroke};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CropSelection {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Canvas for rendering crop selection overlay
#[derive(Debug)]
pub struct SelectionCanvas {
    pub selection: Option<CropSelection>,
}

impl<Message> iced::widget::canvas::Program<Message> for SelectionCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::advanced::mouse::Cursor,
    ) -> Vec<iced::widget::canvas::Geometry> {
        if let Some(selection) = self.selection {
            let mut frame = Frame::new(renderer, bounds.size());

            // Draw semi-transparent overlay outside selection
            let overlay_color = iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5);

            // Top overlay
            if selection.y > 0.0 {
                frame.fill_rectangle(
                    iced::Point::new(0.0, 0.0),
                    iced::Size::new(bounds.width, selection.y),
                    overlay_color,
                );
            }

            // Bottom overlay
            let bottom_y = selection.y + selection.height;
            if bottom_y < bounds.height {
                frame.fill_rectangle(
                    iced::Point::new(0.0, bottom_y),
                    iced::Size::new(bounds.width, bounds.height - bottom_y),
                    overlay_color,
                );
            }

            // Left overlay
            if selection.x > 0.0 {
                frame.fill_rectangle(
                    iced::Point::new(0.0, selection.y),
                    iced::Size::new(selection.x, selection.height),
                    overlay_color,
                );
            }

            // Right overlay
            let right_x = selection.x + selection.width;
            if right_x < bounds.width {
                frame.fill_rectangle(
                    iced::Point::new(right_x, selection.y),
                    iced::Size::new(bounds.width - right_x, selection.height),
                    overlay_color,
                );
            }

            // Draw selection border
            let selection_rect = Path::rectangle(
                iced::Point::new(selection.x, selection.y),
                iced::Size::new(selection.width, selection.height),
            );
            frame.stroke(
                &selection_rect,
                Stroke::default()
                    .with_color(iced::Color::WHITE)
                    .with_width(2.0),
            );

            // Draw corner handles
            const HANDLE_SIZE: f32 = 8.0;
            let handle_color = iced::Color::from_rgb(0.3, 0.7, 1.0);

            let corners = [
                (selection.x, selection.y),                    // Top-left
                (selection.x + selection.width, selection.y),  // Top-right
                (selection.x, selection.y + selection.height), // Bottom-left
                (
                    selection.x + selection.width,
                    selection.y + selection.height,
                ), // Bottom-right
            ];

            for (cx, cy) in corners.iter() {
                frame.fill_rectangle(
                    iced::Point::new(cx - HANDLE_SIZE / 2.0, cy - HANDLE_SIZE / 2.0),
                    iced::Size::new(HANDLE_SIZE, HANDLE_SIZE),
                    handle_color,
                );

                // Draw white border around handle
                let handle_rect = Path::rectangle(
                    iced::Point::new(cx - HANDLE_SIZE / 2.0, cy - HANDLE_SIZE / 2.0),
                    iced::Size::new(HANDLE_SIZE, HANDLE_SIZE),
                );
                frame.stroke(
                    &handle_rect,
                    Stroke::default()
                        .with_color(iced::Color::WHITE)
                        .with_width(1.0),
                );
            }

            vec![frame.into_geometry()]
        } else {
            vec![]
        }
    }
}
