//! Preferences: a full-window page with General and About tabs.

use iced::{
    Alignment, Element, Fill,
    widget::{column, container, text},
};
use next_ui::{
    Icon, chrome,
    layout::PaneContext,
    widget::{ActionRow, Avatar, AvatarShape, RowGroup, Tab, Tabs, TextExt as _, spacing},
};

use crate::{
    app::Request,
    model::{Action, Experience, Model},
    parts,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    General,
    About,
}

#[derive(Default)]
pub struct State {
    page: Page,
}

#[derive(Debug, Clone)]
pub enum Message {
    Page(Page),
    Back,
    Act(Action),
    Window(chrome::Action),
}

impl State {
    pub fn update(&mut self, message: Message) -> Option<Request> {
        match message {
            Message::Page(page) => self.page = page,
            Message::Back => return Some(Request::ClosePreferences),
            Message::Act(action) => return Some(Request::Act(action)),
            Message::Window(action) => return Some(Request::Window(action)),
        }
        None
    }

    pub fn view<'a>(&'a self, model: &'a Model) -> Element<'a, Message> {
        let tabs = Tabs::new(
            [
                Tab::new(Page::General, "General"),
                Tab::new(Page::About, "About"),
            ],
            Some(self.page),
            Message::Page,
        )
        .height(Fill);
        let header = PaneContext::root().header(Message::Window(chrome::Action::Drag), |header| {
            header
                .start(parts::header_button("Back", Icon::Arrow, Message::Back))
                .middle(tabs)
        });
        let body = match self.page {
            Page::General => general(model),
            Page::About => about(),
        };
        parts::page(
            header,
            container(container(body).max_width(880)).center_x(Fill),
        )
    }
}

fn general(model: &Model) -> Element<'_, Message> {
    let current = model.settings.experience;
    let choice = |experience: Experience, label, description| {
        let available = experience.is_available();
        let selected = current == Some(experience);
        let mut row = ActionRow::new(
            label,
            available.then_some(Message::Act(Action::SwitchExperience(experience))),
        )
        .description(if available {
            description
        } else {
            "Not available in this build"
        })
        .selected(selected);
        if selected {
            row = row.trailing(Icon::Checkmark.view());
        }
        row
    };

    RowGroup::new()
        .title("Experience")
        .columns(2)
        .row(choice(
            Experience::Next,
            "Next Mode",
            "The easiest way to use Bottles.",
        ))
        .row(choice(
            Experience::Classic,
            "Classic Mode",
            "The experience for advanced users.",
        ))
        .into()
}

fn about<'a>() -> Element<'a, Message> {
    let hero = column![
        Avatar::new("B")
            .size(104.0)
            .shape(AvatarShape::Rounded)
            .active(true),
        text("Bottles Next").size(30).medium(),
        text("Run Windows software and games, each in a place of its own.")
            .size(17)
            .muted(),
        text(concat!("Version ", env!("CARGO_PKG_VERSION")))
            .size(14)
            .muted(),
    ]
    .spacing(spacing::SM)
    .align_x(Alignment::Center);

    // Opening links needs a URL opener, which the app doesn't have yet.
    let link = |title, description, icon| {
        ActionRow::new(title, None)
            .description(description)
            .leading(icon_tile(icon))
            .trailing(Icon::ExternalLink.view())
    };
    let page = |title, description, icon| {
        ActionRow::new(title, None)
            .description(description)
            .leading(icon_tile(icon))
            .trailing(Icon::ChevronRight.view())
    };

    let content = column![
        container(hero).center_x(Fill).padding([spacing::LG, 0.0]),
        RowGroup::new()
            .row(link(
                "Report a Problem",
                "Opens the issue tracker",
                Icon::Bug
            ))
            .row(link(
                "Source Code",
                "Bottles is free software. Read it, fork it, help out.",
                Icon::Code,
            )),
        RowGroup::new()
            .row(page(
                "Credits",
                "The people who make and translate Bottles",
                Icon::People,
            ))
            .row(page(
                "Legal",
                "License and third-party notices",
                Icon::Document
            )),
    ]
    .spacing(spacing::MD)
    .max_width(640);

    container(content).center_x(Fill).into()
}

fn icon_tile<'a>(icon: Icon) -> Element<'a, Message> {
    container(icon.view().width(22).height(22))
        .center(40)
        .style(|theme| {
            let colors = next_ui::theme::colors(theme);
            container::Style::default()
                .background(colors.border)
                .border(iced::Border::default().rounded(10))
        })
        .into()
}
