//! The Next experience: standalone programs and their launch controls.

use iced::{Element, Fill, Task, widget::text};
use uuid::Uuid;

use crate::{
    Icon,
    jobs::Kind,
    layout::{self, PaneContext},
    widget::{ArtworkCard, CardAction, InfoRow, RowGroup, Search, spacing},
};
use crate::{
    app, chrome,
    context::Context,
    views,
    widget::{InfoCard, InfoCardKind},
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
    Launch(Uuid),
    Stop(Uuid),
    Window(chrome::Action),
}

impl State {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            selected: None,
        }
    }

    pub fn update(&mut self, ctx: &mut Context, message: Message) -> Task<app::Message> {
        match message {
            Message::Window(action) => return Task::done(app::Message::Window(action)),
            Message::QueryChanged(query) => self.query = query,
            Message::Select(id) => self.selected = Some(id),
            Message::Back => self.selected = None,
            Message::Launch(id) => return ctx.launch_entry("programs".into(), id.to_string()),
            Message::Stop(id) => match ctx.core.programs().open(id) {
                Ok(program) => {
                    return ctx.jobs.start(
                        Kind::Stop(id),
                        bottles_core::Operation::new(
                            move |_, _| async move { program.stop().await },
                        ),
                    );
                }
                Err(error) => ctx.notifications.error(error.to_string()),
            },
        }
        Task::none()
    }

    pub fn view<'a>(&'a self, ctx: &'a Context) -> Element<'a, Message> {
        let selected = self
            .selected
            .and_then(|id| ctx.store.programs.iter().find(|program| program.id() == id));
        layout::navigation_split(
            PaneContext::root(),
            selected.is_some(),
            move |pane| self.programs_view(ctx, pane, selected.is_some()),
            move |pane| self.detail_view(ctx, pane),
        )
    }

    fn programs_view<'a>(
        &'a self,
        ctx: &'a Context,
        pane: PaneContext,
        selected: bool,
    ) -> Element<'a, Message> {
        let header = pane.header(Message::Window(chrome::Action::Drag), |header| {
            let header = if selected {
                header.start(views::header_button(
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
        let cards = ctx
            .store
            .programs
            .iter()
            .filter(|program| program.name().to_lowercase().contains(&query))
            .map(|program| {
                let id = program.id();
                let busy = busy(ctx, id);
                ArtworkCard::new(
                    program.name(),
                    if ctx.jobs.running(&Kind::Stop(id)) {
                        "Stopping…"
                    } else if busy {
                        "Launching…"
                    } else {
                        "Installed program"
                    },
                )
                .menu(
                    CardAction::new("Program details", Icon::EllipsisVertical)
                        .on_press(Message::Select(id)),
                )
                .primary(
                    CardAction::new("Play", Icon::Play)
                        .loading(busy)
                        .on_press_maybe((!busy).then_some(Message::Launch(id))),
                )
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
        views::page(header, body, f32::INFINITY)
    }

    fn detail_view<'a>(&'a self, ctx: &'a Context, pane: PaneContext) -> Element<'a, Message> {
        let Some(program) = self
            .selected
            .and_then(|id| ctx.store.programs.iter().find(|program| program.id() == id))
        else {
            return views::page(
                pane.header(Message::Window(chrome::Action::Drag), |header| header),
                text("No program selected"),
                f32::INFINITY,
            );
        };
        let id = program.id();
        let header = pane.header(Message::Window(chrome::Action::Drag), |header| {
            let header = if pane.is_standalone() {
                header.start(views::header_button(
                    "Back to programs",
                    Icon::Arrow,
                    Message::Back,
                ))
            } else {
                header
            };
            header.middle(text(program.name())).end(
                views::header_button("Stop program", Icon::Power, Message::Stop(id))
                    .on_press_maybe((!busy(ctx, id)).then_some(Message::Stop(id))),
            )
        });
        views::page(
            header,
            RowGroup::new()
                .row(InfoRow::new("Executable").description(program.launch().executable()))
                .row(InfoRow::new("Runner").description(program.config().runner.name())),
            f32::INFINITY,
        )
    }
}

fn busy(ctx: &Context, id: Uuid) -> bool {
    ctx.jobs
        .running(&Kind::Launch("programs".into(), id.to_string()))
        || ctx.jobs.running(&Kind::Stop(id))
}
