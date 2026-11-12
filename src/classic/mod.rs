//! Classic view state. Everything else is read from the model.

mod bottle;

use bottles_core::{PrefixBackend, WineArch};
use iced::{
    Element, Fill,
    widget::{column, container},
};
use next_ui::{
    Icon, chrome,
    layout::{PaneContext, Side, navigation_split, side_panel},
    theme,
    widget::{Popover, PopoverItem, Tab, Tabs, Title, header_button},
};
use uuid::Uuid;

use crate::{
    app::Request,
    manage_profiles,
    model::{Action, Activity, Bottle, Experience, Model, Subject},
    parts,
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
    Snapshots,
}

struct Draft {
    name: String,
    runner: Option<Uuid>,
    arch: WineArch,
}

pub struct State {
    tab: TabKind,
    selected: Option<Uuid>,
    detail_tab: Detail,
    draft: Option<Draft>,
    creation_open: bool,
    rename: Option<String>,
    profiles: manage_profiles::State,
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
    Arch(WineArch),
    Create,
    Rename(String),
    SaveName,
    Act(Action),
    Window(chrome::Action),
    Profiles(manage_profiles::Message),
    OpenPreferences,
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
            profiles: manage_profiles::State::default(),
        }
    }

    pub fn update(&mut self, model: &Model, message: Message) -> Option<Request> {
        match message {
            Message::Profiles(message) => return self.profiles.update(model, message),
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
                    runner: model.default_runner(),
                    arch: WineArch::default(),
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
            Message::Arch(arch) => {
                if let Some(draft) = &mut self.draft {
                    draft.arch = arch;
                }
            }
            Message::Create => {
                let draft = self.draft.as_ref().filter(|_| self.can_create(model))?;
                let action = Action::CreateBottle {
                    name: draft.name.clone(),
                    runner: draft.runner?,
                    arch: draft.arch,
                };
                self.creation_open = false;
                return Some(Request::Act(action));
            }
            Message::Rename(name) => self.rename = Some(name),
            Message::SaveName => {
                let name = self.rename.take()?;
                return Some(Request::Act(Action::Rename(self.selected?, name)));
            }
            Message::Act(action) => return Some(Request::Act(action)),
            Message::Window(action) => return Some(Request::Window(action)),
            Message::OpenPreferences => return Some(Request::OpenPreferences),
        }
        None
    }

    /// Returns whether the open draft names a bottle that can be created.
    fn can_create(&self, model: &Model) -> bool {
        self.creation_open
            && self.draft.as_ref().is_some_and(|draft| {
                !draft.name.trim().is_empty()
                    && draft
                        .runner
                        .is_some_and(|runner| model.can_create_with(runner, draft.arch))
            })
    }

    fn selected<'a>(&self, model: &'a Model) -> Option<Bottle<'a>> {
        model.bottle(self.selected?)
    }

    pub fn view<'a>(&'a self, model: &'a Model) -> Element<'a, Message> {
        side_panel(
            Side::End,
            self.profiles.is_open(),
            move |_| self.screen(model),
            move |pane| self.profiles.pane(model, pane).map(Message::Profiles),
        )
    }

    fn screen<'a>(&'a self, model: &'a Model) -> Element<'a, Message> {
        side_panel(
            Side::Start,
            self.creation_open,
            move |pane| match self.tab {
                TabKind::Bottles => navigation_split(
                    pane,
                    self.selected(model).is_some(),
                    move |pane| self.primary_page(model, pane),
                    move |pane| self.detail_page(model, pane),
                ),
                TabKind::Library => self.primary_page(model, pane),
            },
            move |pane| self.creation_page(model, pane),
        )
    }

    fn primary_page<'a>(&'a self, model: &'a Model, pane: PaneContext) -> Element<'a, Message> {
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
            let header = parts::app_controls(header, model, pane, Message::Profiles, Message::Act);
            header
                .start(parts::app_menu(
                    Experience::Classic,
                    Message::OpenPreferences,
                    |experience| Message::Act(Action::SwitchExperience(experience)),
                ))
                .start(parts::header_button("Add bottle", Icon::Plus, Message::New))
                .middle(tabs)
        });
        let body = match self.tab {
            TabKind::Bottles => bottle::rows(model, self.selected),
            TabKind::Library => parts::library_grid(
                model,
                "",
                |target| Message::Act(Action::Launch(target)),
                |_| None,
            ),
        };
        parts::page(header, body)
    }

    fn creation_page<'a>(&'a self, model: &'a Model, pane: PaneContext) -> Element<'a, Message> {
        let Some(draft) = &self.draft else {
            return column![].into();
        };
        let can_create = self.can_create(model);
        let header = pane.header(Message::Window(chrome::Action::Drag), |header| {
            header
                .start(parts::header_button(
                    "Cancel creation",
                    Icon::Arrow,
                    Message::Cancel,
                ))
                .middle(Title::new("New Bottle").subtitle("Creating a new bottle."))
                .end(
                    parts::header_button("Create bottle", Icon::Checkmark, Message::Create)
                        .on_press_maybe(can_create.then_some(Message::Create)),
                )
        });
        parts::page(header, bottle::creation(draft, model, can_create))
    }

    fn detail_page<'a>(&'a self, model: &'a Model, pane: PaneContext) -> Element<'a, Message> {
        let Some(bottle) = self.selected(model) else {
            return column![].into();
        };
        let id = bottle.id;
        let activity = model.activity(&Subject::Prefix(id));
        // Snapshots are taken through FVS, which Virgo's availability reports.
        let snapshots = PrefixBackend::Virgo
            .is_available()
            .then(|| Tab::new(Detail::Snapshots, "Snapshots"));
        let tabs = [
            Tab::new(Detail::Programs, "Programs"),
            Tab::new(Detail::Settings, "Settings"),
        ]
        .into_iter()
        .chain(snapshots);
        let tabs = Tabs::new(tabs, Some(self.detail_tab), Message::Detail).height(Fill);
        let header = pane.header(Message::Window(chrome::Action::Drag), |mut header| {
            if pane.is_standalone() {
                header = header.start(parts::header_button(
                    "Back to bottles",
                    Icon::Arrow,
                    Message::Back,
                ));
            }
            let mut stop = PopoverItem::new("Stop bottle").icon(Icon::Stop);
            let mut delete = PopoverItem::new("Delete bottle").icon(Icon::Cross);
            if activity == Activity::Running {
                stop = stop.on_select(Message::Act(Action::Stop(id)));
            }
            if activity.is_settled() {
                delete = delete.on_select(Message::Act(Action::Delete(id)));
            }
            let header = header
                .start(
                    Popover::new(header_button("Bottle menu", Icon::EllipsisVertical))
                        .item(stop)
                        .item(delete),
                )
                .middle(tabs);
            parts::app_controls(header, model, pane, Message::Profiles, Message::Act)
        });
        let status = bottle::status(&bottle, activity);
        let content = match self.detail_tab {
            Detail::Programs => bottle::programs(bottle, model),
            Detail::Settings => {
                bottle::settings(bottle, self.rename.as_deref(), activity.is_settled())
            }
            Detail::Snapshots => Title::new("Snapshots")
                .subtitle("Snapshot management is coming later.")
                .into(),
        };
        let body = container(
            column![parts::scroll_content(content), status]
                .width(Fill)
                .height(Fill),
        )
        .style(theme::panel)
        .clip(true);
        column![header, body].width(Fill).height(Fill).into()
    }
}
