use iced::{
    Element, Fill,
    alignment::Vertical,
    widget::{Space, button, column, container, row, text},
};

use crate::icons::Icon;

use super::{style, text::TextExt as _};

pub struct StatusBar<'a, Message> {
    architecture: &'a str,
    runner: &'a str,
    is_running: bool,
    log: &'a str,
    expanded: bool,
    toggle: Message,
}

impl<'a, Message> StatusBar<'a, Message> {
    pub fn new(architecture: &'a str, runner: &'a str, toggle: Message) -> Self {
        Self {
            architecture,
            runner,
            is_running: false,
            log: "",
            expanded: false,
            toggle,
        }
    }

    pub fn running(mut self, is_running: bool) -> Self {
        self.is_running = is_running;
        self
    }

    pub fn log(mut self, log: &'a str) -> Self {
        self.log = log;
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }
}

impl<'a, Message: Clone + 'a> From<StatusBar<'a, Message>> for Element<'a, Message> {
    fn from(status: StatusBar<'a, Message>) -> Self {
        let StatusBar {
            architecture,
            runner,
            is_running,
            log,
            expanded,
            toggle,
        } = status;
        let header = row![
            row![Icon::Chip.view(), text(architecture).supporting().muted(),]
                .spacing(12)
                .align_y(Vertical::Center),
            row![Icon::Run.view(), text(runner).supporting().muted(),]
                .spacing(12)
                .align_y(Vertical::Center),
            Space::new().width(Fill),
            row![
                if is_running {
                    Icon::Lightning.view()
                } else {
                    Icon::Power.view()
                },
                text(if is_running { "Running" } else { "Stopped" })
                    .supporting()
                    .muted(),
            ]
            .spacing(12)
            .align_y(Vertical::Center),
            button(Icon::Computer.view())
                .padding(0)
                .style(style::tab)
                .on_press(toggle),
        ]
        .spacing(42)
        .align_y(Vertical::Center)
        .padding([8, 32]);

        let mut content = column![header].width(Fill);

        if expanded {
            content = content.push(
                container(text(log).supporting())
                    .padding([16, 22])
                    .width(Fill)
                    .style(|_| container::background(crate::theme::HINT)),
            );
        }

        container(content)
            .width(Fill)
            .clip(true)
            .style(style::panel)
            .into()
    }
}
