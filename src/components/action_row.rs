use iced::{
    Alignment, ContentFit, Element, Point, Radians, Rectangle, Renderer, Theme, mouse,
    widget::{canvas, column, container, row, stack, svg, text},
};

use crate::icons::Icon;

use super::{list_row::ListRow, text::TextExt as _};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    Determinate(u8),
    Indeterminate,
}

#[derive(Debug, Clone)]
pub enum ActionRowState<Message> {
    Ready(Message),
    Disabled,
    Progress(Progress),
}

pub struct ActionRow<'a, Message> {
    title: &'a str,
    description: &'a str,
    icon: Option<Icon>,
    state: ActionRowState<Message>,
}

impl<'a, Message> ActionRow<'a, Message> {
    pub fn new(title: &'a str, state: ActionRowState<Message>) -> Self {
        Self {
            title,
            description: "",
            icon: None,
            state,
        }
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = description;
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

impl<'a, Message: Clone + 'a> From<ActionRow<'a, Message>> for Element<'a, Message> {
    fn from(action: ActionRow<'a, Message>) -> Self {
        ListRow::from(action).into()
    }
}

impl<'a, Message: Clone + 'a> From<ActionRow<'a, Message>> for ListRow<'a, Message> {
    fn from(action: ActionRow<'a, Message>) -> Self {
        let mut description = row![].spacing(16).align_y(Alignment::Center);

        if let Some(icon) = action.icon {
            description = description.push(
                svg(icon.handle())
                    .width(24)
                    .height(24)
                    .content_fit(ContentFit::Contain),
            );
        }

        description = description.push(text(action.description).detail().muted());

        let labels = column![text(action.title).label(), description].spacing(4);

        match action.state {
            ActionRowState::Ready(message) => ListRow::new(labels)
                .trailing(Icon::Arrow.rotated(std::f32::consts::PI))
                .on_press(message),
            ActionRowState::Disabled => ListRow::new(labels)
                .trailing(Icon::Arrow.rotated(std::f32::consts::PI))
                .enabled(false),
            ActionRowState::Progress(progress) => {
                ListRow::new(labels).trailing(progress_indicator(progress))
            }
        }
    }
}

fn progress_indicator<'a, Message: 'a>(progress: Progress) -> Element<'a, Message> {
    let label: Element<'a, Message> = match progress {
        Progress::Determinate(value) => {
            column![text(value.min(100)).caption(), text("%").caption()]
                .align_x(Alignment::Center)
                .into()
        }
        Progress::Indeterminate => text("…").caption().into(),
    };

    stack![
        canvas::Canvas::new(ProgressRing { progress })
            .width(40)
            .height(40),
        container(label).center(40),
    ]
    .width(40)
    .height(40)
    .into()
}

#[derive(Debug, Clone, Copy)]
struct ProgressRing {
    progress: Progress,
}

impl<Message> canvas::Program<Message> for ProgressRing {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = bounds.width.min(bounds.height) / 2.0 - 2.0;
        let palette = theme.extended_palette();

        frame.stroke(
            &canvas::Path::circle(center, radius),
            canvas::Stroke::default()
                .with_width(4.0)
                .with_color(palette.background.stronger.color),
        );

        let fraction = progress_fraction(self.progress);
        let start = -std::f32::consts::FRAC_PI_2;
        let arc = canvas::Path::new(|path| {
            path.arc(canvas::path::Arc {
                center,
                radius,
                start_angle: Radians(start),
                end_angle: Radians(start + std::f32::consts::TAU * fraction),
            });
        });

        frame.stroke(
            &arc,
            canvas::Stroke::default()
                .with_width(4.0)
                .with_color(palette.secondary.base.text),
        );

        vec![frame.into_geometry()]
    }
}

fn progress_fraction(progress: Progress) -> f32 {
    match progress {
        Progress::Determinate(value) => f32::from(value.min(100)) / 100.0,
        Progress::Indeterminate => 0.7,
    }
}

#[cfg(test)]
mod tests {
    use super::{ActionRow, ActionRowState, Progress, progress_fraction};

    #[test]
    fn state_is_explicit_and_progress_is_bounded_at_render_time() {
        let ready = ActionRow::new("Open", ActionRowState::Ready(()));
        let progress = ActionRow::<()>::new(
            "Install",
            ActionRowState::Progress(Progress::Determinate(150)),
        );

        assert!(matches!(ready.state, ActionRowState::Ready(())));
        assert!(matches!(
            progress.state,
            ActionRowState::Progress(Progress::Determinate(150))
        ));
        assert_eq!(progress_fraction(Progress::Determinate(150)), 1.0);
        assert_eq!(progress_fraction(Progress::Determinate(25)), 0.25);
    }
}
