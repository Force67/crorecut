use iced::mouse;
use iced::widget::canvas::{self, Action, Frame, LineCap, Path};
use iced::{Event, Length, Point, Radians, Rectangle, Size, Widget};

use super::Message;
use crate::ui::color as app_colors;

const HANDLE_WIDTH: f32 = 10.0;
const HANDLE_HEIGHT: f32 = 26.0;
const TRACK_HEIGHT: f32 = 4.0;
const PLAYHEAD_WIDTH: f32 = 4.0;
const PLAYHEAD_HEIGHT: f32 = 28.0;
const TIMELINE_HEIGHT: f32 = 40.0;

/// A trim timeline with dual handles and playhead scrubbing.
struct TrimTimeline {
    min: f64,
    max: f64,
    start_value: f64,
    end_value: f64,
    playhead: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct TrimTimelineState {
    drag_mode: DragMode,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum DragMode {
    #[default]
    None,
    StartHandle,
    EndHandle,
    Scrub,
}

impl DragMode {
    fn is_active(self) -> bool {
        !matches!(self, Self::None)
    }
}

#[derive(Debug, Clone, Copy)]
struct TimelineGeometry {
    range: f64,
    track_x: f32,
    track_width: f32,
    track_y: f32,
    handle_y: f32,
    playhead_y: f32,
    start_x: f32,
    end_x: f32,
    playhead_x: f32,
}

impl TimelineGeometry {
    fn new(timeline: &TrimTimeline, bounds: Rectangle) -> Option<Self> {
        let range = timeline.max - timeline.min;
        let track_width = bounds.width - HANDLE_WIDTH;

        if range <= 0.0 || track_width <= 0.0 {
            return None;
        }

        let track_x = HANDLE_WIDTH / 2.0;
        let point_x = |value: f64| track_x + ((value - timeline.min) / range) as f32 * track_width;

        Some(Self {
            range,
            track_x,
            track_width,
            track_y: (bounds.height - TRACK_HEIGHT) / 2.0,
            handle_y: (bounds.height - HANDLE_HEIGHT) / 2.0,
            playhead_y: (bounds.height - PLAYHEAD_HEIGHT) / 2.0,
            start_x: point_x(timeline.start_value),
            end_x: point_x(timeline.end_value),
            playhead_x: point_x(timeline.playhead),
        })
    }

    fn value_at(self, min: f64, x: f32) -> f64 {
        let normalized = ((x - self.track_x) / self.track_width) as f64;
        min + normalized.clamp(0.0, 1.0) * self.range
    }

    fn drag_mode_at(self, x: f32) -> DragMode {
        let start_dist = (x - self.start_x).abs();
        let end_dist = (x - self.end_x).abs();

        if start_dist < HANDLE_WIDTH && start_dist <= end_dist {
            DragMode::StartHandle
        } else if end_dist < HANDLE_WIDTH {
            DragMode::EndHandle
        } else {
            DragMode::Scrub
        }
    }
}

impl TrimTimeline {
    fn drag_action(
        &self,
        drag_mode: DragMode,
        geometry: TimelineGeometry,
        x: f32,
    ) -> Action<Message> {
        let value = geometry.value_at(self.min, x);
        let message = match drag_mode {
            DragMode::StartHandle => Message::TrimStartChanged(value),
            DragMode::EndHandle => Message::TrimEndChanged(value),
            DragMode::Scrub | DragMode::None => Message::SliderChanged(value),
        };

        Action::publish(message).and_capture()
    }

    fn drag_x(bounds: Rectangle, cursor: mouse::Cursor) -> Option<f32> {
        cursor
            .position_in(bounds)
            .map(|position| position.x)
            .or_else(|| cursor.position().map(|position| position.x - bounds.x))
    }
}

impl canvas::Program<Message> for TrimTimeline {
    type State = TrimTimelineState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let geometry = TimelineGeometry::new(self, bounds)?;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let cursor_x = cursor.position_in(bounds)?.x;

                state.drag_mode = geometry.drag_mode_at(cursor_x);
                Some(self.drag_action(state.drag_mode, geometry, cursor_x))
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.drag_mode.is_active() {
                    state.drag_mode = DragMode::None;
                    Some(Action::capture())
                } else {
                    None
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if !state.drag_mode.is_active() {
                    return None;
                }

                Self::drag_x(bounds, cursor).map(|x| self.drag_action(state.drag_mode, geometry, x))
            }
            _ => None,
        }
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

        if let Some(geometry) = TimelineGeometry::new(self, bounds) {
            draw_track(&mut frame, geometry);
            draw_selected_range(&mut frame, geometry);
            draw_playhead(&mut frame, geometry);
            draw_handles(&mut frame, geometry);
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.drag_mode.is_active() {
            return mouse::Interaction::Grabbing;
        }

        let Some(cursor_position) = cursor.position_in(bounds) else {
            return mouse::Interaction::default();
        };

        let Some(geometry) = TimelineGeometry::new(self, bounds) else {
            return mouse::Interaction::default();
        };

        match geometry.drag_mode_at(cursor_position.x) {
            DragMode::StartHandle | DragMode::EndHandle => mouse::Interaction::Grab,
            DragMode::Scrub => mouse::Interaction::Pointer,
            DragMode::None => mouse::Interaction::default(),
        }
    }
}

fn draw_track(frame: &mut Frame, geometry: TimelineGeometry) {
    let track = Path::rectangle(
        Point::new(geometry.track_x, geometry.track_y),
        Size::new(geometry.track_width, TRACK_HEIGHT),
    );

    frame.fill(
        &track,
        iced::Color {
            a: 0.4,
            ..app_colors::CONTROL_ACCENT
        },
    );
}

fn draw_selected_range(frame: &mut Frame, geometry: TimelineGeometry) {
    let range = Path::rectangle(
        Point::new(geometry.start_x, geometry.track_y),
        Size::new((geometry.end_x - geometry.start_x).max(0.0), TRACK_HEIGHT),
    );

    frame.fill(&range, iced::color!(0x232326));
}

fn draw_playhead(frame: &mut Frame, geometry: TimelineGeometry) {
    let playhead = Path::line(
        Point::new(geometry.playhead_x, geometry.playhead_y),
        Point::new(geometry.playhead_x, geometry.playhead_y + PLAYHEAD_HEIGHT),
    );

    frame.stroke(
        &playhead,
        canvas::Stroke::default()
            .with_color(iced::color!(0x585860))
            .with_width(PLAYHEAD_WIDTH)
            .with_line_cap(LineCap::Round),
    );
}

fn draw_handles(frame: &mut Frame, geometry: TimelineGeometry) {
    let trim_handle = trim_handle_svg();

    frame.draw_svg(
        Rectangle::new(
            Point::new(geometry.start_x - HANDLE_WIDTH / 2.0, geometry.handle_y),
            Size::new(HANDLE_WIDTH, HANDLE_HEIGHT),
        ),
        trim_handle.clone(),
    );
    frame.draw_svg(
        Rectangle::new(
            Point::new(geometry.end_x - HANDLE_WIDTH / 2.0, geometry.handle_y),
            Size::new(HANDLE_WIDTH, HANDLE_HEIGHT),
        ),
        trim_handle.rotation(Radians(std::f32::consts::PI)),
    );
}

fn trim_handle_svg() -> iced::advanced::svg::Svg {
    iced::advanced::svg::Svg::new(iced::advanced::svg::Handle::from_memory(
        crate::icons::TRIM_HANDLE_SVG,
    ))
}

/// Create a trim timeline widget with playhead scrubbing.
pub fn trim_timeline(
    min: f64,
    max: f64,
    start: f64,
    end: f64,
    playhead: f64,
) -> impl Widget<Message> {
    iced::widget::canvas(TrimTimeline {
        min,
        max,
        start_value: start,
        end_value: end,
        playhead,
    })
    .width(Length::Fill)
    .height(Length::Fixed(TIMELINE_HEIGHT))
}
