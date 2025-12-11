use iced::widget::canvas::{Frame, Path, Stroke};
use iced::{Event, Rectangle, mouse};

use crate::app::Message;
use crate::ui::editor::ImageTransform;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CropSelection {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Canvas for rendering crop selection overlay
/// Note: selection is in image space (preview frame coordinates), not canvas space
#[derive(Debug)]
pub struct SelectionCanvas {
    pub selection: Option<CropSelection>,
    pub preview_width: f32,
    pub preview_height: f32,
}

/// State that holds the canvas bounds (updated during draw)
#[derive(Debug, Default, Clone)]
pub struct CanvasState {
    pub bounds: (f32, f32),
}

impl iced::widget::canvas::Program<Message> for SelectionCanvas {
    type State = CanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<iced::widget::canvas::Action<Message>> {
        use iced::widget::canvas::Action;

        // Update canvas bounds in state
        state.bounds = (bounds.width, bounds.height);

        if let Some(position) = cursor.position_in(bounds)
            && let Event::Mouse(mouse_event) = event
        {
            match mouse_event {
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    return Some(Action::publish(Message::CropMousePressed));
                }
                mouse::Event::ButtonReleased(mouse::Button::Left) => {
                    return Some(Action::publish(Message::CropMouseReleased));
                }
                mouse::Event::CursorMoved { .. } => {
                    return Some(Action::publish(Message::CropMouseMoved {
                        x: position.x,
                        y: position.y,
                        canvas_width: bounds.width,
                        canvas_height: bounds.height,
                    }));
                }
                _ => {}
            }
        }

        None
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::advanced::mouse::Cursor,
    ) -> Vec<iced::widget::canvas::Geometry> {
        if let Some(img_selection) = self.selection {
            let mut frame = Frame::new(renderer, bounds.size());

            // Calculate transform from image space to canvas space
            let transform = ImageTransform::from_contain(
                bounds.width,
                bounds.height,
                self.preview_width,
                self.preview_height,
            );

            // Transform selection from image space to canvas space
            let (sel_x, sel_y) = transform.image_to_canvas(img_selection.x, img_selection.y);
            let (sel_right, sel_bottom) = transform.image_to_canvas(
                img_selection.x + img_selection.width,
                img_selection.y + img_selection.height,
            );

            let selection = CropSelection {
                x: sel_x,
                y: sel_y,
                width: sel_right - sel_x,
                height: sel_bottom - sel_y,
            };

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
