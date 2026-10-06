//! Mouse area wrapper that reports cursor positions with click messages.

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree};
use iced::advanced::{Shell, Widget};
use iced::{Event, Length, Point, Rectangle, Size, Vector, mouse};

pub struct PositionedMouseArea<'a, Message, W> {
    content: W,
    on_click: Box<dyn Fn(Point) -> Message + 'a>,
}

impl<'a, Message, W> PositionedMouseArea<'a, Message, W> {
    pub fn new(content: W, on_click: impl Fn(Point) -> Message + 'a) -> Self {
        Self {
            content,
            on_click: Box::new(on_click),
        }
    }
}

pub fn positioned_mouse_area<'a, Message, W>(
    content: W,
    on_click: impl Fn(Point) -> Message + 'a,
) -> PositionedMouseArea<'a, Message, W> {
    PositionedMouseArea::new(content, on_click)
}

#[derive(Debug, Default)]
struct State {
    is_pressed: bool,
}

impl<Message, W> widget::Meta for PositionedMouseArea<'_, Message, W> {}

impl<Message, W, Theme, Renderer> Widget<Message, Theme, Renderer>
    for PositionedMouseArea<'_, Message, W>
where
    Renderer: renderer::Renderer,
    W: Widget<Message, Theme, Renderer>,
{
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
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
        let state = tree.state.downcast_mut::<State>();
        let is_over = cursor.is_over(layout.bounds());

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if is_over => {
                state.is_pressed = true;
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.is_pressed
                    && is_over
                    && let Some(position) = cursor.position()
                {
                    shell.publish((self.on_click)(position));
                }

                state.is_pressed = false;
            }
            _ => {}
        }

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
        self.content.draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
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
