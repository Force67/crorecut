use iced::mouse;
use iced::widget::canvas::{self, Action, Frame, Path};
use iced::{Element, Event, Length, Point, Rectangle, Size};

use super::Message;

const HANDLE_WIDTH: f32 = 12.0;
const HANDLE_HEIGHT: f32 = 24.0;
const TRACK_HEIGHT: f32 = 6.0;
const PLAYHEAD_WIDTH: f32 = 3.0;

/// A dual-handle range slider for trim selection with playhead scrubbing
pub struct RangeSlider {
    pub min: f64,
    pub max: f64,
    pub start_value: f64,
    pub end_value: f64,
    pub playhead: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RangeSliderState {
    is_dragging_start: bool,
    is_dragging_end: bool,
    is_scrubbing: bool,
}

impl canvas::Program<Message> for RangeSlider {
    type State = RangeSliderState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let range = self.max - self.min;
        if range <= 0.0 {
            return None;
        }

        let track_width = bounds.width - HANDLE_WIDTH * 2.0;
        let start_x = HANDLE_WIDTH + ((self.start_value - self.min) / range) as f32 * track_width;
        let end_x = HANDLE_WIDTH + ((self.end_value - self.min) / range) as f32 * track_width;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(cursor_position) = cursor.position_in(bounds) {
                    // Check if clicking on start handle (prioritize if handles overlap)
                    let start_dist = (cursor_position.x - start_x).abs();
                    let end_dist = (cursor_position.x - end_x).abs();

                    if start_dist < HANDLE_WIDTH && start_dist <= end_dist {
                        state.is_dragging_start = true;
                        return Some(Action::capture());
                    }
                    // Check if clicking on end handle
                    if end_dist < HANDLE_WIDTH {
                        state.is_dragging_end = true;
                        return Some(Action::capture());
                    }

                    // Clicking on track - start scrubbing
                    state.is_scrubbing = true;
                    let normalized = ((cursor_position.x - HANDLE_WIDTH) / track_width) as f64;
                    let new_value = self.min + normalized.clamp(0.0, 1.0) * range;
                    return Some(Action::publish(Message::SliderChanged(new_value)).and_capture());
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.is_dragging_start || state.is_dragging_end || state.is_scrubbing {
                    state.is_dragging_start = false;
                    state.is_dragging_end = false;
                    state.is_scrubbing = false;
                    return Some(Action::capture());
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let calc_value = |x: f32| -> f64 {
                    let normalized = ((x - HANDLE_WIDTH) / track_width) as f64;
                    self.min + normalized.clamp(0.0, 1.0) * range
                };

                if let Some(cursor_position) = cursor.position_in(bounds) {
                    if state.is_dragging_start {
                        return Some(
                            Action::publish(Message::TrimStartChanged(calc_value(
                                cursor_position.x,
                            )))
                            .and_capture(),
                        );
                    }
                    if state.is_dragging_end {
                        return Some(
                            Action::publish(Message::TrimEndChanged(calc_value(cursor_position.x)))
                                .and_capture(),
                        );
                    }
                    if state.is_scrubbing {
                        return Some(
                            Action::publish(Message::SliderChanged(calc_value(cursor_position.x)))
                                .and_capture(),
                        );
                    }
                } else if state.is_dragging_start || state.is_dragging_end || state.is_scrubbing {
                    // Cursor moved outside bounds while dragging - keep tracking
                    if let Some(cursor_position) = cursor.position() {
                        let relative_x = cursor_position.x - bounds.x;
                        if state.is_dragging_start {
                            return Some(
                                Action::publish(Message::TrimStartChanged(calc_value(relative_x)))
                                    .and_capture(),
                            );
                        }
                        if state.is_dragging_end {
                            return Some(
                                Action::publish(Message::TrimEndChanged(calc_value(relative_x)))
                                    .and_capture(),
                            );
                        }
                        if state.is_scrubbing {
                            return Some(
                                Action::publish(Message::SliderChanged(calc_value(relative_x)))
                                    .and_capture(),
                            );
                        }
                    }
                }
            }
            _ => {}
        }

        None
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let range = self.max - self.min;
        if range <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let track_width = bounds.width - HANDLE_WIDTH * 2.0;
        let track_y = (bounds.height - TRACK_HEIGHT) / 2.0;

        // Draw background track (outside trim range - darker)
        let track_bg = Path::rectangle(
            Point::new(HANDLE_WIDTH, track_y),
            Size::new(track_width, TRACK_HEIGHT),
        );
        frame.fill(&track_bg, iced::Color::from_rgb(0.2, 0.2, 0.2));

        // Calculate handle positions
        let start_x = HANDLE_WIDTH + ((self.start_value - self.min) / range) as f32 * track_width;
        let end_x = HANDLE_WIDTH + ((self.end_value - self.min) / range) as f32 * track_width;

        // Draw selected range (highlighted track between handles)
        let selected_range = Path::rectangle(
            Point::new(start_x, track_y),
            Size::new((end_x - start_x).max(0.0), TRACK_HEIGHT),
        );
        frame.fill(&selected_range, iced::Color::from_rgb(0.35, 0.55, 0.75));

        // Draw playhead
        let playhead_x = HANDLE_WIDTH + ((self.playhead - self.min) / range) as f32 * track_width;
        let playhead_height = bounds.height * 0.85;
        let playhead_y = (bounds.height - playhead_height) / 2.0;
        let playhead = Path::rectangle(
            Point::new(playhead_x - PLAYHEAD_WIDTH / 2.0, playhead_y),
            Size::new(PLAYHEAD_WIDTH, playhead_height),
        );
        frame.fill(&playhead, iced::color!(0xFFD144));

        let handle_y = (bounds.height - HANDLE_HEIGHT) / 2.0;
        let handle_color = iced::Color::WHITE;
        let handle_border = iced::Color::from_rgb(0.4, 0.4, 0.4);

        let start_handle = Path::rectangle(
            Point::new(start_x - HANDLE_WIDTH / 2.0, handle_y),
            Size::new(HANDLE_WIDTH, HANDLE_HEIGHT),
        );
        frame.fill(&start_handle, handle_color);
        frame.stroke(
            &start_handle,
            canvas::Stroke::default()
                .with_color(handle_border)
                .with_width(1.0),
        );

        let end_handle = Path::rectangle(
            Point::new(end_x - HANDLE_WIDTH / 2.0, handle_y),
            Size::new(HANDLE_WIDTH, HANDLE_HEIGHT),
        );
        frame.fill(&end_handle, handle_color);
        frame.stroke(
            &end_handle,
            canvas::Stroke::default()
                .with_color(handle_border)
                .with_width(1.0),
        );

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.is_dragging_start || state.is_dragging_end {
            return mouse::Interaction::Grabbing;
        }
        if state.is_scrubbing {
            return mouse::Interaction::Grabbing;
        }

        if let Some(cursor_position) = cursor.position_in(bounds) {
            let range = self.max - self.min;
            if range > 0.0 {
                let track_width = bounds.width - HANDLE_WIDTH * 2.0;
                let start_x =
                    HANDLE_WIDTH + ((self.start_value - self.min) / range) as f32 * track_width;
                let end_x =
                    HANDLE_WIDTH + ((self.end_value - self.min) / range) as f32 * track_width;

                if (cursor_position.x - start_x).abs() < HANDLE_WIDTH
                    || (cursor_position.x - end_x).abs() < HANDLE_WIDTH
                {
                    return mouse::Interaction::Grab;
                }

                return mouse::Interaction::Pointer;
            }
        }

        mouse::Interaction::default()
    }
}

/// Create a range slider widget with playhead
pub fn range_slider(
    min: f64,
    max: f64,
    start: f64,
    end: f64,
    playhead: f64,
) -> Element<'static, Message> {
    iced::widget::canvas(RangeSlider {
        min,
        max,
        start_value: start,
        end_value: end,
        playhead,
    })
    .width(Length::Fill)
    .height(Length::Fixed(40.0))
    .into()
}
