//! Material-style ripple effect wrapper component

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::{Color, Element, Event, Length, Point, Rectangle, Size, Vector, mouse};
use std::time::Instant;

const RIPPLE_DURATION: f32 = 0.5;
const FADE_DURATION: f32 = 0.25;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum RippleShape {
    #[default]
    Rectangular,
    Circular,
    // TODO: Rounded (not supported due to `iced` limitation at the moment)
}

pub struct Ripple<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    ripple_color: Color,
    shape: RippleShape,
}

impl<'a, Message, Theme, Renderer> Ripple<'a, Message, Theme, Renderer> {
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            content: content.into(),
            ripple_color: Color::WHITE,
            shape: RippleShape::default(),
        }
    }

    #[must_use]
    pub fn ripple_color(mut self, color: Color) -> Self {
        self.ripple_color = color;
        self
    }

    #[must_use]
    pub fn shape(mut self, shape: RippleShape) -> Self {
        self.shape = shape;
        self
    }
}

pub fn ripple<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Ripple<'a, Message, Theme, Renderer> {
    Ripple::new(content)
}

#[derive(Debug, Clone)]
struct RippleAnimation {
    origin: Point,
    start_time: Instant,
    release_time: Option<Instant>,
    expansion_at_release: f32,
    max_radius: f32,
}

impl RippleAnimation {
    fn expansion_progress(&self) -> f32 {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        (elapsed / RIPPLE_DURATION).min(1.0)
    }

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
struct RippleState {
    is_pressed: bool,
    ripples: Vec<RippleAnimation>,
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Ripple<'a, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<RippleState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(RippleState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let state = tree.state.downcast_mut::<RippleState>();

        // Clean up finished ripples
        state.ripples.retain(|r| !r.is_finished());

        // Handle ripple animation events
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor.is_over(bounds) {
                    state.is_pressed = true;

                    if let Some(cursor_pos) = cursor.position_in(bounds) {
                        // For circular shape, always start from center
                        let origin = if self.shape == RippleShape::Circular {
                            Point::new(bounds.width / 2.0, bounds.height / 2.0)
                        } else {
                            cursor_pos
                        };

                        let corners = [
                            Point::new(0.0, 0.0),
                            Point::new(bounds.width, 0.0),
                            Point::new(0.0, bounds.height),
                            Point::new(bounds.width, bounds.height),
                        ];
                        let max_radius = corners
                            .iter()
                            .map(|c| {
                                let dx = c.x - origin.x;
                                let dy = c.y - origin.y;
                                (dx * dx + dy * dy).sqrt()
                            })
                            .fold(0.0_f32, |a, b| a.max(b));

                        state.ripples.push(RippleAnimation {
                            origin,
                            start_time: Instant::now(),
                            release_time: None,
                            expansion_at_release: 0.0,
                            max_radius,
                        });
                    }
                }
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
                }
            }

            _ => {}
        }

        if !state.ripples.is_empty() {
            shell.request_redraw();
        }

        // Pass event to child widget
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        // Draw child widget first
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );

        // Draw ripple overlay with clipping
        let bounds = layout.bounds();
        let state = tree.state.downcast_ref::<RippleState>();

        if state.ripples.is_empty() {
            return;
        }

        let (clip_center, clip_radius) = if self.shape == RippleShape::Circular {
            let radius = bounds.width.min(bounds.height) / 2.0;
            let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
            (Some(center), radius)
        } else {
            (None, 0.0)
        };

        renderer.with_layer(bounds, |renderer| {
            for ripple in &state.ripples {
                let expansion = ripple.expansion_progress();
                let eased = 1.0 - (1.0 - expansion).powi(3);
                let current_radius = ripple.max_radius * eased;
                let ripple_alpha = self.ripple_color.a;

                let alpha = if ripple.is_held() {
                    ripple_alpha
                } else {
                    let held_alpha = ripple_alpha * (1.0 - ripple.expansion_at_release * 0.2);
                    let fade = ripple.fade_progress();
                    held_alpha * (1.0 - fade)
                };

                if alpha > 0.001 && current_radius > 0.0 {
                    let ripple_color = Color {
                        a: alpha,
                        ..self.ripple_color
                    };

                    // For circular clipping, limit the ripple radius so it doesn't
                    // extend beyond the widget's circular boundary
                    let effective_radius = if let Some(center) = clip_center {
                        let dx = ripple.origin.x - center.x;
                        let dy = ripple.origin.y - center.y;
                        let dist_to_center = (dx * dx + dy * dy).sqrt();
                        let max_allowed = clip_radius + dist_to_center;
                        current_radius.min(max_allowed)
                    } else {
                        current_radius
                    };

                    // Draw ripple circle using a quad with fully rounded corners
                    let ripple_bounds = Rectangle {
                        x: bounds.x + ripple.origin.x - effective_radius,
                        y: bounds.y + ripple.origin.y - effective_radius,
                        width: effective_radius * 2.0,
                        height: effective_radius * 2.0,
                    };

                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: ripple_bounds,
                            border: iced::Border {
                                radius: effective_radius.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        ripple_color,
                    );
                }
            }
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Ripple<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(ripple: Ripple<'a, Message, Theme, Renderer>) -> Self {
        Self::new(ripple)
    }
}
