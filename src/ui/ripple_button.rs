//! Material-style ripple button component

use iced::widget::canvas::{self, Frame, Path};
use iced::{Element, Length, Point, Rectangle, mouse};
use std::time::Instant;

// android look-and-feel
const RIPPLE_DURATION: f32 = 0.45;
const FADE_DURATION: f32 = 0.25;
const RIPPLE_ALPHA: f32 = 0.12;

pub struct RippleButton<'a, Message> {
    label: String,
    width: Length,
    height: Length,
    padding: f32,
    on_press: Option<Message>,
    background_color: iced::Color,
    _phantom: std::marker::PhantomData<&'a Message>,
}

impl<'a, Message> RippleButton<'a, Message>
where
    Message: Clone,
{
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            width: Length::Shrink,
            height: Length::Shrink,
            padding: 12.0,
            on_press: None,
            background_color: iced::Color::from_rgb(0.2, 0.2, 0.2),
            _phantom: std::marker::PhantomData,
        }
    }

    #[must_use]
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    #[must_use]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    #[must_use]
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    #[must_use]
    pub fn background(mut self, color: iced::Color) -> Self {
        self.background_color = color;
        self
    }
}

#[derive(Debug, Clone)]
struct Ripple {
    origin: Point,
    start_time: Instant,
    release_time: Option<Instant>,
    expansion_at_release: f32,
    max_radius: f32,
}

impl Ripple {
    /// Progress of the expansion animation (0.0 to 1.0)
    fn expansion_progress(&self) -> f32 {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        (elapsed / RIPPLE_DURATION).min(1.0)
    }

    /// Progress of the fade-out animation (0.0 to 1.0), only valid after release
    fn fade_progress(&self) -> f32 {
        match self.release_time {
            Some(release) => {
                let elapsed = release.elapsed().as_secs_f32();
                (elapsed / FADE_DURATION).min(1.0)
            }
            None => 0.0,
        }
    }

    fn is_held(&self) -> bool {
        self.release_time.is_none()
    }

    fn is_finished(&self) -> bool {
        self.release_time.is_some() && self.fade_progress() >= 1.0
    }
}

#[derive(Debug, Default)]
pub struct RippleState {
    is_hovered: bool,
    is_pressed: bool,
    ripples: Vec<Ripple>,
    press_position: Option<Point>,
}

struct RippleProgram<Message> {
    label: String,
    on_press: Option<Message>,
    background_color: iced::Color,
}

impl<Message> canvas::Program<Message> for RippleProgram<Message>
where
    Message: Clone,
{
    type State = RippleState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        use iced::Event;

        state.ripples.retain(|r| !r.is_finished());

        let needs_animation = !state.ripples.is_empty();
        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let was_hovered = state.is_hovered;
                state.is_hovered = cursor.is_over(bounds);

                if was_hovered != state.is_hovered {
                    return Some(canvas::Action::request_redraw());
                }

                if needs_animation {
                    return Some(canvas::Action::request_redraw());
                }

                None
            }

            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor.is_over(bounds) && self.on_press.is_some() {
                    state.is_pressed = true;

                    // Start ripple animation from click position
                    if let Some(position) = cursor.position_in(bounds) {
                        // Calculate max radius to cover the entire button
                        let corners = [
                            Point::new(0.0, 0.0),
                            Point::new(bounds.width, 0.0),
                            Point::new(0.0, bounds.height),
                            Point::new(bounds.width, bounds.height),
                        ];
                        let max_radius = corners
                            .iter()
                            .map(|c| {
                                let dx = c.x - position.x;
                                let dy = c.y - position.y;
                                (dx * dx + dy * dy).sqrt()
                            })
                            .fold(0.0_f32, |a, b| a.max(b));

                        state.ripples.push(Ripple {
                            origin: position,
                            start_time: Instant::now(),
                            release_time: None,
                            expansion_at_release: 0.0,
                            max_radius,
                        });
                        state.press_position = Some(position);
                    }

                    return Some(canvas::Action::request_redraw().and_capture());
                }
                None
            }

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.is_pressed {
                    state.is_pressed = false;

                    let now = Instant::now();
                    for ripple in &mut state.ripples {
                        if ripple.is_held() {
                            ripple.expansion_at_release = ripple.expansion_progress();
                            ripple.release_time = Some(now);
                        }
                    }

                    return Some(canvas::Action::request_redraw());
                }
                None
            }

            Event::Mouse(mouse::Event::CursorLeft) => {
                state.is_hovered = false;
                Some(canvas::Action::request_redraw())
            }

            Event::Window(iced::window::Event::RedrawRequested(_)) => {
                // Continue animation if there are active ripples
                if !state.ripples.is_empty() {
                    // Clean up finished ripples
                    state.ripples.retain(|r| !r.is_finished());

                    if !state.ripples.is_empty() {
                        return Some(canvas::Action::request_redraw());
                    }
                }
                None
            }

            _ => {
                if needs_animation {
                    Some(canvas::Action::request_redraw())
                } else {
                    None
                }
            }
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // Determine background color based on state
        let bg_color = self.background_color;

        let bg_path = Path::rectangle(Point::ORIGIN, bounds.size());
        frame.fill(&bg_path, bg_color);

        for ripple in &state.ripples {
            let expansion = ripple.expansion_progress();
            let eased = 1.0 - (1.0 - expansion).powi(3);
            let current_radius = ripple.max_radius * eased;

            let alpha = if ripple.is_held() {
                RIPPLE_ALPHA * (1.0 - expansion * 0.2) // Slight fade during expansion while held
            } else {
                // Start fade from the alpha value at release time
                let held_alpha = RIPPLE_ALPHA * (1.0 - ripple.expansion_at_release * 0.2);
                let fade = ripple.fade_progress();
                held_alpha * (1.0 - fade)
            };

            if alpha > 0.001 && current_radius > 0.0 {
                let ripple_color = iced::Color::from_rgba(1.0, 1.0, 1.0, alpha);
                frame.fill(&Path::circle(ripple.origin, current_radius), ripple_color);
            }
        }

        // Draw text label centered
        frame.fill_text(canvas::Text {
            content: self.label.clone(),
            position: Point::new(bounds.width / 2.0, bounds.height / 2.0),
            color: iced::Color::WHITE,
            size: iced::Pixels(14.0),
            font: iced::Font::default(),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Center,
            shaping: iced::widget::text::Shaping::Basic,
            line_height: iced::widget::text::LineHeight::default(),
            max_width: f32::INFINITY,
        });

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) && self.on_press.is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

impl<'a, Message> From<RippleButton<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(button: RippleButton<'a, Message>) -> Self {
        let program = RippleProgram {
            label: button.label,
            on_press: button.on_press,
            background_color: button.background_color,
        };

        iced::widget::canvas(program)
            .width(button.width)
            .height(button.height)
            .into()
    }
}

pub fn ripple_button<'a, Message>(label: impl Into<String>) -> RippleButton<'a, Message>
where
    Message: Clone + 'a,
{
    RippleButton::new(label)
}
