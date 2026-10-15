//! Classic view state. Snapshots and operations belong to the application context.

mod bottle;

use bottles_core::{BottleState, Operation, PrefixBackend};
use iced::{
    Element, Fill, Task,
    widget::{column, container},
};
use next_ui::{
    Icon,
    widget::{Popover, PopoverItem, Tab, Tabs, Title, header_button},
};
use uuid::Uuid;

use crate::{
    app, chrome,
    context::Context,
    jobs::Kind,
    layout::{PaneContext, Side, navigation_split, side_panel},
    theme, views,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabKind {
    Bottles,
    Library,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    Programs,
    Settings,
    #[cfg(feature = "fvs")]
    Snapshots,
}

struct Draft {
    name: String,
    runner: Option<Uuid>,
}

pub struct State {
    tab: TabKind,
    selected: Option<Uuid>,
    detail_tab: Detail,
    draft: Option<Draft>,
    creation_open: bool,
    rename: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tab(TabKind),
    Select(Uuid),
    Detail(Detail),
    Back,
    New,
    Cancel,
    Name(String),
    Runner(Uuid),
    Create,
    Rename(String),
    SaveName,
    LaunchProgram { bottle: Uuid, program: Uuid },
    LaunchLibrary(String, String),
    Stop(Uuid),
    Delete(Uuid),
    ToggleGamescope(Uuid, bool),
    ToggleMangoHud(Uuid, bool),
    NewProgram,
    Window(chrome::Action),
}

impl State {
    pub fn new() -> Self {
        Self {
            tab: TabKind::Bottles,
            selected: None,
            detail_tab: Detail::Programs,
            draft: None,
            creation_open: false,
            rename: None,
        }
    }

    pub fn update(&mut self, ctx: &mut Context, message: Message) -> Task<app::Message> {
        match message {
            Message::Tab(tab) => self.tab = tab,
            Message::Select(id) => {
                if self.selected != Some(id) {
                    self.selected = Some(id);
                    self.detail_tab = Detail::Programs;
                    self.rename = None;
                }
            }
            Message::Detail(detail) => self.detail_tab = detail,
            Message::Back => {
                self.selected = None;
                self.rename = None;
            }
            Message::New => {
                self.creation_open = true;
                self.draft = Some(Draft {
                    name: "Gaming paradise".into(),
                    runner: ctx.store.addons.runners().first().map(|runner| runner.id()),
                });
            }
            Message::Cancel => self.creation_open = false,
            Message::Name(name) => {
                if let Some(draft) = &mut self.draft {
                    draft.name = name;
                }
            }
            Message::Runner(runner) => {
                if let Some(draft) = &mut self.draft {
                    draft.runner = Some(runner);
                }
            }
            Message::Create => {
                let Some(draft) = self.draft.as_ref().filter(|_| self.creation_open) else {
                    return Task::none();
                };
                let runner = ctx
                    .store
                    .addons
                    .runners()
                    .into_iter()
                    .find(|runner| Some(runner.id()) == draft.runner);
                let Some(runner) = runner else {
                    return Task::none();
                };
                let Some(winebridge) = ctx.store.addons.winebridges().into_iter().next() else {
                    return Task::none();
                };
                let name = draft.name.clone();
                let operation = ctx.core.bottles().create(
                    name.clone(),
                    PrefixBackend::Standard,
                    runner.as_ref().clone(),
                    winebridge.as_ref().clone(),
                    ctx.store
                        .addons
                        .umus()
                        .first()
                        .map(|umu| umu.as_ref().clone()),
                );
                self.creation_open = false;
                return ctx.jobs.start(Kind::CreateBottle(name), operation);
            }
            Message::Rename(name) => self.rename = Some(name),
            Message::SaveName => {
                let (Some(id), Some(name)) = (self.selected, self.rename.take()) else {
                    return Task::none();
                };
                match ctx.core.bottles().open(id) {
                    Ok(bottle) => {
                        return ctx.jobs.start(
                            Kind::Edit(id),
                            bottle.edit(move |edit| {
                                edit.rename(name);
                                Ok(())
                            }),
                        );
                    }
                    Err(error) => ctx.notifications.error(error.to_string()),
                }
            }
            Message::LaunchProgram {
                bottle: id,
                program,
            } => return ctx.launch_entry("bottles".into(), format!("{id}/{program}")),
            Message::LaunchLibrary(provider, entry) => return ctx.launch_entry(provider, entry),
            Message::Stop(id) => match ctx.core.bottles().open(id) {
                Ok(bottle) => {
                    return ctx.jobs.start(
                        Kind::Stop(id),
                        Operation::new(move |_, _| async move { bottle.stop().await }),
                    );
                }
                Err(error) => ctx.notifications.error(error.to_string()),
            },
            Message::Delete(id) => {
                return ctx
                    .jobs
                    .start(Kind::Delete(id), ctx.core.bottles().delete(id));
            }
            Message::ToggleGamescope(id, enabled) | Message::ToggleMangoHud(id, enabled) => {
                let gamescope = matches!(message, Message::ToggleGamescope(..));
                match ctx.core.bottles().open(id) {
                    Ok(bottle) => {
                        return ctx.jobs.start(
                            Kind::Edit(id),
                            bottle.edit(move |edit| {
                                if gamescope {
                                    edit.wrappers().gamescope.enabled = enabled;
                                } else {
                                    edit.wrappers().mangohud.enabled = enabled;
                                }
                                Ok(())
                            }),
                        );
                    }
                    Err(error) => ctx.notifications.error(error.to_string()),
                }
            }
            Message::NewProgram => ctx
                .notifications
                .info("Program installation is not available yet"),
            Message::Window(action) => return Task::done(app::Message::Window(action)),
        }
        Task::none()
    }

    fn selected<'a>(&self, ctx: &'a Context) -> Option<&'a BottleState> {
        ctx.store
            .bottles
            .iter()
            .find(|bottle| Some(bottle.id()) == self.selected)
            .map(AsRef::as_ref)
    }

    pub fn view<'a>(&'a self, ctx: &'a Context) -> Element<'a, Message> {
        side_panel(
            Side::Start,
            self.creation_open,
            move |pane| {
                navigation_split(
                    pane,
                    self.tab == TabKind::Bottles && self.selected(ctx).is_some(),
                    move |pane| self.primary_page(ctx, pane),
                    move |pane| self.detail_page(ctx, pane),
                )
            },
            move |pane| self.creation_page(ctx, pane),
        )
    }

    fn primary_page<'a>(&'a self, ctx: &'a Context, pane: PaneContext) -> Element<'a, Message> {
        let tabs = Tabs::new(
            [
                Tab::new(TabKind::Bottles, "Bottles"),
                Tab::new(TabKind::Library, "Library"),
            ],
            Some(self.tab),
            Message::Tab,
        )
        .height(Fill);
        let header = pane.header(Message::Window(chrome::Action::Drag), |header| {
            header
                .start(views::header_button("Add bottle", Icon::Plus, Message::New))
                .middle(tabs)
        });
        let body = match self.tab {
            TabKind::Bottles => bottle::rows(
                &ctx.store,
                &ctx.jobs,
                self.selected(ctx).map(BottleState::id),
            ),
            TabKind::Library => views::library_grid(ctx, "", Message::LaunchLibrary),
        };
        views::page(header, body)
    }

    fn creation_page<'a>(&'a self, ctx: &'a Context, pane: PaneContext) -> Element<'a, Message> {
        let Some(draft) = &self.draft else {
            return column![].into();
        };
        let can_create = self.creation_open
            && !draft.name.trim().is_empty()
            && ctx
                .store
                .addons
                .runners()
                .iter()
                .any(|runner| Some(runner.id()) == draft.runner)
            && !ctx.store.addons.winebridges().is_empty();
        let header = pane.header(Message::Window(chrome::Action::Drag), |header| {
            header
                .start(views::header_button(
                    "Cancel creation",
                    Icon::Arrow,
                    Message::Cancel,
                ))
                .middle(Title::new("New Bottle").subtitle("Creating a new bottle."))
                .end(
                    views::header_button("Create bottle", Icon::Checkmark, Message::Create)
                        .on_press_maybe(can_create.then_some(Message::Create)),
                )
        });
        views::page(header, bottle::creation(draft, &ctx.store, can_create))
    }

    fn detail_page<'a>(&'a self, ctx: &'a Context, pane: PaneContext) -> Element<'a, Message> {
        let Some(bottle) = self.selected(ctx) else {
            return column![].into();
        };
        let id = bottle.id();
        let busy = bottle::busy(&ctx.jobs, id);
        let tabs = [
            Tab::new(Detail::Programs, "Programs"),
            Tab::new(Detail::Settings, "Settings"),
        ];
        #[cfg(feature = "fvs")]
        let tabs = tabs
            .into_iter()
            .chain([Tab::new(Detail::Snapshots, "Snapshots")]);
        let tabs = Tabs::new(tabs, Some(self.detail_tab), Message::Detail).height(Fill);
        let header = pane.header(Message::Window(chrome::Action::Drag), |mut header| {
            if pane.is_standalone() {
                header = header.start(views::header_button(
                    "Back to bottles",
                    Icon::Arrow,
                    Message::Back,
                ));
            }
            let mut stop = PopoverItem::new("Stop bottle").icon(Icon::Stop);
            let mut delete = PopoverItem::new("Delete bottle").icon(Icon::Cross);
            if !busy {
                stop = stop.on_select(Message::Stop(id));
                delete = delete.on_select(Message::Delete(id));
            }
            header
                .start(
                    Popover::new(header_button("Bottle menu", Icon::EllipsisVertical))
                        .item(stop)
                        .item(delete),
                )
                .middle(tabs)
        });
        let content = match self.detail_tab {
            Detail::Programs => bottle::programs(bottle, &ctx.jobs),
            Detail::Settings => bottle::settings(bottle, self.rename.as_deref(), &ctx.jobs),
            #[cfg(feature = "fvs")]
            Detail::Snapshots => Title::new("Snapshots")
                .subtitle("Snapshot management is coming later.")
                .into(),
        };
        let body = container(
            column![views::scroll_content(content), bottle::status(bottle)]
                .width(Fill)
                .height(Fill),
        )
        .style(theme::panel)
        .clip(true);
        column![header, body].width(Fill).height(Fill).into()
    }
}
