use iced::{
    Background, Border, Element, Length, Padding, Point, Rectangle, Renderer, Theme, border,
    widget::{button, canvas, container, stack},
};

use super::focusable_action::{ActionState, FocusableAction};

const BORDER_RADIUS: f32 = 6.0;
const BORDER_WIDTH: f32 = 2.0;
const DASH_PATTERN: &[f32] = &[6.0, 6.0];

/// A dashed activation surface.
pub struct ActionTile<'a, Message> {
    content: Element<'a, Message>,
    on_activate: Option<Message>,
    width: Length,
    height: Length,
    padding: Padding,
}

impl<'a, Message> ActionTile<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>, message: Message) -> Self {
        Self {
            content: content.into(),
            on_activate: Some(message),
            width: Length::Shrink,
            height: Length::Shrink,
            padding: Padding::ZERO,
        }
    }

    pub fn disabled(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            on_activate: None,
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

impl<'a, Message: Clone + 'a> From<ActionTile<'a, Message>> for Element<'a, Message> {
    fn from(target: ActionTile<'a, Message>) -> Self {
        let ActionTile {
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

        FocusableAction::new(content)
            .on_press_maybe(on_activate)
            .width(width)
            .height(height)
            .style(style)
            .into()
    }
}

fn style(theme: &Theme, state: ActionState) -> button::Style {
    let colors = crate::theme::colors(theme);

    button::Style {
        background: if state.pressed {
            Some(Background::Color(colors.selection))
        } else if state.hovered || state.focused {
            Some(Background::Color(colors.hover))
        } else {
            None
        },
        text_color: colors.text,
        border: Border::default().rounded(BORDER_RADIUS),
        ..button::Style::default()
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
                    .with_color(crate::theme::colors(theme).border)
            },
        );

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::Space;

    use super::*;

    #[test]
    fn constructors_make_activation_explicit() {
        let disabled: ActionTile<'_, ()> = ActionTile::disabled(Space::new());
        assert!(disabled.on_activate.is_none());

        let active = ActionTile::new(Space::new(), ());
        assert_eq!(active.on_activate, Some(()));
    }
}
