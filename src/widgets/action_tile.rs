use iced::{
    Alignment, Border, Length, Point, Rectangle, Renderer, border,
    widget::{
        button, canvas, column, container, row, stack, text,
        text::{Fragment, IntoFragment},
    },
};

use super::focusable_action::{ActionState, FocusableAction};
use super::text::TextExt as _;
use super::{radius, spacing, text_size};
use crate::{Element, Theme, icons::Icon};

const BORDER_WIDTH: f32 = 2.0;
const DASH_PATTERN: &[f32] = &[6.0, 6.0];

const DISC: f32 = 44.0;
const DISC_ICON: f32 = 16.0;

/// A dashed tile that starts something new, e.g. adding a program: an icon
/// in a disc beside a title and subtitle, centered in whatever space it is
/// given. Without [`on_press`](Self::on_press) it is disabled.
pub struct ActionTile<'a, Message> {
    icon: Icon,
    title: Fragment<'a>,
    subtitle: Fragment<'a>,
    on_press: Option<Message>,
}

impl<'a, Message> ActionTile<'a, Message> {
    pub fn new(icon: Icon, title: impl IntoFragment<'a>, subtitle: impl IntoFragment<'a>) -> Self {
        Self {
            icon,
            title: title.into_fragment(),
            subtitle: subtitle.into_fragment(),
            on_press: None,
        }
    }

    pub fn on_press(self, message: Message) -> Self {
        self.on_press_maybe(Some(message))
    }

    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }
}

impl<'a, Message: Clone + 'a> From<ActionTile<'a, Message>> for Element<'a, Message> {
    fn from(tile: ActionTile<'a, Message>) -> Self {
        let disc = container(tile.icon.view().width(DISC_ICON).height(DISC_ICON))
            .center(DISC)
            .style(|theme: &Theme| {
                container::Style::default()
                    .background(theme.colors().card)
                    .border(Border::default().rounded(DISC / 2.0))
            });
        let labels = column![
            text(tile.title).size(text_size::HEADING).medium(),
            text(tile.subtitle).size(text_size::SMALL),
        ]
        .spacing(spacing::XS);
        let content = stack![
            container(row![disc, labels].spacing(16).align_y(Alignment::Center))
                .center(Length::Fill)
                .padding(spacing::LG),
            canvas::Canvas::new(Outline)
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        FocusableAction::new(content)
            .on_press_maybe(tile.on_press)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(style)
            .into()
    }
}

fn style(theme: &Theme, _: ActionState) -> button::Style {
    let colors = theme.colors();

    button::Style {
        text_color: colors.text,
        border: Border::default().rounded(radius::ROW),
        ..button::Style::default()
    }
}

struct Outline;

impl<Message> canvas::Program<Message, Theme> for Outline {
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
            border::Radius::from(radius::ROW),
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
                    .with_color(theme.colors().raised)
            },
        );

        vec![frame.into_geometry()]
    }
}
