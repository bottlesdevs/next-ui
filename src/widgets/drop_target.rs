use iced::{
    Element, Length, Padding, Point, Rectangle, Renderer, Theme, border,
    widget::{canvas, container, stack},
};

use super::{Control, style};

const BORDER_RADIUS: f32 = 6.0;
const BORDER_WIDTH: f32 = 2.0;
const DASH_PATTERN: &[f32] = &[6.0, 6.0];

/// An interactive surface with a dashed outline.
pub struct DropTarget<'a, Message> {
    content: Element<'a, Message>,
    on_activate: Message,
    width: Length,
    height: Length,
    padding: Padding,
}

impl<'a, Message> DropTarget<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>, on_activate: Message) -> Self {
        Self {
            content: content.into(),
            on_activate,
            width: Length::Shrink,
            height: Length::Shrink,
            padding: Padding::ZERO,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }
}

impl<'a, Message: Clone + 'a> From<DropTarget<'a, Message>> for Element<'a, Message> {
    fn from(target: DropTarget<'a, Message>) -> Self {
        let DropTarget {
            content,
            on_activate,
            width,
            height,
            padding,
        } = target;
        let content = stack![
            container(content)
                .width(width)
                .height(height)
                .padding(padding),
            canvas::Canvas::new(Outline)
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .width(width)
        .height(height);

        Control::new(content)
            .on_press(on_activate)
            .width(width)
            .height(height)
            .style(style::action)
            .into()
    }
}

struct Outline;

impl<Message> canvas::Program<Message> for Outline {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let inset = BORDER_WIDTH / 2.0;
        let outline = canvas::Path::rounded_rectangle(
            Point::new(inset, inset),
            iced::Size::new(
                (bounds.width - inset * 2.0).max(0.0),
                (bounds.height - inset * 2.0).max(0.0),
            ),
            border::Radius::from(BORDER_RADIUS),
        );

        frame.stroke(
            &outline,
            canvas::Stroke {
                line_dash: canvas::LineDash {
                    segments: DASH_PATTERN,
                    offset: 0,
                },
                ..canvas::Stroke::default()
                    .with_width(BORDER_WIDTH)
                    .with_color(theme.extended_palette().background.neutral.color)
            },
        );

        vec![frame.into_geometry()]
    }
}
