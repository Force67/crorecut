//! Material-style ripple effect wrapper component

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree};
use iced::advanced::{Shell, Widget};
use iced::{Color, Event, Length, Point, Rectangle, Size, Vector, mouse};
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

pub struct Ripple<W> {
    content: W,
    ripple_color: Color,
    shape: RippleShape,
    enabled: bool,
}

impl<W> Ripple<W> {
    pub fn new(content: W) -> Self {
        Self {
            content,
            ripple_color: Color::WHITE,
            shape: RippleShape::default(),
            enabled: true,
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

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

pub fn ripple<W>(content: W) -> Ripple<W> {
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

impl<W> widget::Meta for Ripple<W> {}

impl<Message, W, Theme, Renderer> Widget<Message, Theme, Renderer> for Ripple<W>
where
    Renderer: renderer::Renderer,
    W: Widget<Message, Theme, Renderer>,
{
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<RippleState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(RippleState::default())
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) {
        self.content.layout(&mut tree.children[0], renderer, limits);

        tree.size = tree.children[0].size;
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout,
        viewport: &Rectangle,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .operate(&mut tree.children[0], layout, viewport, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout,
        cursor: mouse::Cursor,
        renderer: &Renderer,
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
                if self.enabled && cursor.is_over(bounds) {
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
        self.content.update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        // Draw child widget first
        self.content.draw(
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
        layout: Layout,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
        window: Size,
    ) -> Vec<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
            window,
        )
    }
}
