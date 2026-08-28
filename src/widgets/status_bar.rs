use iced::{
    Element, Fill, Theme,
    alignment::Vertical,
    widget::{Space, column, container, row, scrollable, text},
};

use crate::{icons::Icon, theme};

use super::{button::Button, spacing, text::TextExt as _};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

impl StatusState {
    const fn label(self) -> &'static str {
        match self {
            Self::Stopped => "Stopped",
            Self::Starting => "Starting",
            Self::Running => "Running",
            Self::Stopping => "Stopping",
            Self::Failed => "Failed",
        }
    }

    const fn icon(self) -> Icon {
        match self {
            Self::Stopped | Self::Stopping => Icon::Power,
            Self::Starting | Self::Running => Icon::Lightning,
            Self::Failed => Icon::Cross,
        }
    }
}

pub struct StatusBar<'a, Message> {
    architecture: &'a str,
    runner: &'a str,
    state: StatusState,
    log: Option<(&'a str, bool, Message)>,
}

impl<'a, Message> StatusBar<'a, Message> {
    pub fn new(architecture: &'a str, runner: &'a str, state: StatusState) -> Self {
        Self {
            architecture,
            runner,
            state,
            log: None,
        }
    }

    pub fn log(mut self, log: &'a str, expanded: bool, on_toggle: Message) -> Self {
        self.log = Some((log, expanded, on_toggle));
        self
    }
}

impl<'a, Message: Clone + 'a> From<StatusBar<'a, Message>> for Element<'a, Message> {
    fn from(status: StatusBar<'a, Message>) -> Self {
        let mut header = row![
            row![
                Icon::Chip.view(),
                text(status.architecture).supporting().muted(),
            ]
            .spacing(spacing::SM)
            .align_y(Vertical::Center),
            row![Icon::Run.view(), text(status.runner).supporting().muted(),]
                .spacing(spacing::SM)
                .align_y(Vertical::Center),
            Space::new().width(Fill),
            row![
                status.state.icon().view(),
                text(status.state.label())
                    .supporting()
                    .style(move |theme: &Theme| text::Style {
                        color: Some(if status.state == StatusState::Failed {
                            theme.palette().danger
                        } else {
                            theme.extended_palette().secondary.weak.text
                        }),
                    }),
            ]
            .spacing(spacing::SM)
            .align_y(Vertical::Center),
        ]
        .spacing(spacing::LG)
        .align_y(Vertical::Center);

        if let Some((_, _, on_toggle)) = &status.log {
            header = header.push(
                Button::icon_only("Toggle log", Icon::Computer)
                    .diameter(32.0)
                    .on_press(on_toggle.clone()),
            );
        }

        let mut content =
            column![container(header).padding([spacing::XS, spacing::LG])].width(Fill);

        if let Some((log, true, _)) = status.log {
            content = content.push(
                container(scrollable(text(log).supporting()).height(Fill))
                    .padding([spacing::MD, spacing::LG])
                    .width(Fill)
                    .max_height(180)
                    .style(|current_theme: &Theme| {
                        container::background(theme::BottlesTheme::from(current_theme).hint.color)
                    }),
            );
        }

        container(content)
            .width(Fill)
            .clip(true)
            .style(theme::panel)
            .into()
    }
}
