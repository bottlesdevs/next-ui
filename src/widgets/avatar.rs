use iced::{
    Border, Color, Element, Length,
    widget::{container, text},
};

use super::text::TextExt as _;

/// The outline of an [`Avatar`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AvatarShape {
    /// A circle, for people and profiles.
    #[default]
    Circle,
    /// A rounded square, for apps and plugins.
    Rounded,
}

/// A coloured shape showing an initial, standing in for a picture.
pub struct Avatar<'a> {
    initial: &'a str,
    size: f32,
    shape: AvatarShape,
    active: bool,
    color: Option<Color>,
}

impl<'a> Avatar<'a> {
    /// Creates a 44px circle showing `initial`.
    pub fn new(initial: &'a str) -> Self {
        Self {
            initial,
            size: 44.0,
            shape: AvatarShape::Circle,
            active: false,
            color: None,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(1.0);
        self
    }

    pub fn shape(mut self, shape: AvatarShape) -> Self {
        self.shape = shape;
        self
    }

    /// Draws the avatar in the accent colour, e.g. for the current profile.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Overrides the background, e.g. with a plugin's own colour.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl<'a, Message: 'a> From<Avatar<'a>> for Element<'a, Message> {
    fn from(avatar: Avatar<'a>) -> Self {
        let Avatar {
            initial,
            size,
            shape,
            active,
            color,
        } = avatar;
        let radius = match shape {
            AvatarShape::Circle => size / 2.0,
            AvatarShape::Rounded => size * 0.25,
        };

        container(text(initial).size(size * 0.4).medium())
            .center(Length::Fixed(size))
            .style(move |theme| {
                let colors = crate::theme::colors(theme);
                let (background, foreground) = match (color, active) {
                    (Some(color), _) => (color, colors.text),
                    (None, true) => (colors.accent, colors.window),
                    (None, false) => (colors.selection, colors.text),
                };

                container::Style::default()
                    .background(background)
                    .color(foreground)
                    .border(Border::default().rounded(radius))
            })
            .into()
    }
}
