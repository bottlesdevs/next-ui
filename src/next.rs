//! The Next experience: standalone programs and their launch controls.

use iced::{Element, Fill, widget::text};
use next_ui::{
    Icon, chrome,
    layout::{self, PaneContext},
    widget::{ArtworkCard, CardAction, InfoCard, InfoCardKind, InfoRow, RowGroup, Search, spacing},
};
use uuid::Uuid;

use crate::{
    app::Request,
    model::{Action, Activity, Model, Program},
    parts,
};

pub struct State {
    query: String,
    selected: Option<Uuid>,
}

#[derive(Clone)]
pub enum Message {
    QueryChanged(String),
    Select(Uuid),
    Back,
    Act(Action),
    Window(chrome::Action),
}

impl State {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            selected: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Option<Request> {
        match message {
            Message::Window(action) => return Some(Request::Window(action)),
            Message::Act(action) => return Some(Request::Act(action)),
            Message::QueryChanged(query) => self.query = query,
            Message::Select(id) => self.selected = Some(id),
            Message::Back => self.selected = None,
        }
        None
    }

    pub fn view<'a>(&'a self, model: &'a Model) -> Element<'a, Message> {
        let selected = self.selected(model);
        layout::navigation_split(
            PaneContext::root(),
            selected.is_some(),
            move |pane| self.programs_view(model, pane, selected.is_some()),
            move |pane| self.detail_view(model, pane),
        )
    }

    fn programs_view<'a>(
        &'a self,
        model: &'a Model,
        pane: PaneContext,
        selected: bool,
    ) -> Element<'a, Message> {
        let header = pane.header(Message::Window(chrome::Action::Drag), |header| {
            let header = if selected {
                header.start(parts::header_button(
                    "Back to programs",
                    Icon::Arrow,
                    Message::Back,
                ))
            } else {
                header
            };
            header.middle(
                iced::widget::container(Search::new(
                    "Search programs",
                    &self.query,
                    Message::QueryChanged,
                ))
                .width(Fill)
                .max_width(550),
            )
        });
        let query = self.query.to_lowercase();
        let cards = model
            .programs()
            .into_iter()
            .filter(|program| program.name.to_lowercase().contains(&query))
            .map(|program| {
                let activity = model.activity(&program.target().subject());
                ArtworkCard::new(
                    program.name,
                    match activity {
                        Activity::Stopping => "Stopping…",
                        Activity::Launching => "Launching…",
                        Activity::Running => "Running",
                        Activity::Idle | Activity::Busy => "Installed program",
                    },
                )
                .menu(
                    CardAction::new("Program details", Icon::EllipsisVertical)
                        .on_press(Message::Select(program.prefix)),
                )
                .primary(if activity == Activity::Running {
                    CardAction::new("Stop", Icon::Stop)
                        .on_press(Message::Act(Action::Stop(program.prefix)))
                } else {
                    CardAction::new("Play", Icon::Play)
                        .loading(activity != Activity::Idle)
                        .on_press_maybe(
                            (activity == Activity::Idle)
                                .then(|| Message::Act(Action::Launch(program.target()))),
                        )
                })
                .into()
            })
            .collect::<Vec<_>>();
        let body: Element<'a, Message> = if cards.is_empty() {
            InfoCard::new(
                InfoCardKind::Hint,
                "Nothing here yet",
                if query.is_empty() {
                    "Installed programs will show up here."
                } else {
                    "No programs match your search."
                },
            )
            .into()
        } else {
            iced::widget::Grid::with_children(cards)
                .fluid(400.0)
                .spacing(spacing::SM)
                .height(iced::widget::grid::aspect_ratio(3.0, 2.0))
                .into()
        };
        parts::page(header, body)
    }

    fn detail_view<'a>(&'a self, model: &'a Model, pane: PaneContext) -> Element<'a, Message> {
        let Some(program) = self.selected(model) else {
            return parts::page(
                pane.header(Message::Window(chrome::Action::Drag), |header| header),
                text("No program selected"),
            );
        };
        let stop = Message::Act(Action::Stop(program.prefix));
        let running = model.activity(&program.target().subject()) == Activity::Running;
        let header = pane.header(Message::Window(chrome::Action::Drag), |header| {
            let header = if pane.is_standalone() {
                header.start(parts::header_button(
                    "Back to programs",
                    Icon::Arrow,
                    Message::Back,
                ))
            } else {
                header
            };
            header.middle(text(program.name)).end(
                parts::header_button("Stop program", Icon::Power, stop.clone())
                    .on_press_maybe(running.then_some(stop)),
            )
        });
        parts::page(
            header,
            RowGroup::new()
                .row(InfoRow::new("Executable").description(program.spec.executable()))
                .row(InfoRow::new("Runner").description(program.state.runner.name())),
        )
    }

    fn selected<'a>(&self, model: &'a Model) -> Option<Program<'a>> {
        model.program(self.selected?)
    }
}
