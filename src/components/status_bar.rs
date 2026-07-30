use iced::{
    Element, Fill, Theme,
    alignment::Vertical,
    widget::{Space, button, column, container, row, text},
};

use super::style;

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
            row![
                text("▣").size(30).style(muted),
                text(architecture).size(24).style(muted),
            ]
            .spacing(12)
            .align_y(Vertical::Center),
            row![
                text("♟").size(30).style(muted),
                text(runner).size(24).style(muted),
            ]
            .spacing(12)
            .align_y(Vertical::Center),
            Space::new().width(Fill),
            row![
                text(if is_running { "ϟ" } else { "○" })
                    .size(32)
                    .style(muted),
                text(if is_running { "Running" } else { "Stopped" })
                    .size(24)
                    .style(muted),
            ]
            .spacing(12)
            .align_y(Vertical::Center),
            button(text("▣").size(30))
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
                container(text(log).size(24).style(text::base))
                    .padding([16, 22])
                    .width(Fill)
                    .style(|theme| {
                        container::Style::default()
                            .color(theme.palette().text)
                            .background(theme.palette().background)
                    }),
            );
        }

        container(content)
            .width(Fill)
            .clip(true)
            .style(style::surface)
            .into()
    }
}

fn muted(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().secondary.weak.text),
    }
}
