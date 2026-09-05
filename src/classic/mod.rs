//! The complete Classic experience: routes, dialogs, read models, and workflows.
//!
//! A future Next experience is a sibling of this module and cannot access these
//! private workflow modules.

mod accounts_view;
mod bottles_view;
mod layout;
mod library_view;
mod profiles_view;
mod settings_view;
#[cfg(feature = "fvs")]
mod snapshots_view;

use crate::{
    Icon,
    domain::{BottleSnapshot, DomainSnapshots},
    features::{bottle_settings, bottles, library, profiles},
    theme,
    widget::{
        ActionRow, Button, ButtonKind, Dialog, InfoCard, InfoCardKind, RowGroup, Tab, Tabs,
        TextRow, Title,
    },
};
use bottles_core::Bottles;
use iced::{
    Element, Fill, Task,
    widget::{column, container, keyed_column, scrollable},
};
use layout::{PaneContext, Side, navigation_split, side_panel};
use uuid::Uuid;

#[cfg(feature = "fvs")]
use crate::features::snapshots;

const CONTENT_MAX_WIDTH: f32 = 1150.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryTab {
    Bottles,
    Library,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Programs,
    Settings,
    #[cfg(feature = "fvs")]
    Snapshots,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Bottles,
    Bottle { id: Uuid, tab: DetailTab },
    Library,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelKind {
    NewBottle,
    Profiles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PanelState {
    kind: PanelKind,
    open: bool,
}

impl PanelState {
    fn open(kind: PanelKind) -> Self {
        Self { kind, open: true }
    }

    fn close(&mut self) {
        self.open = false;
    }
}

pub struct State {
    route: Route,
    panel: Option<PanelState>,
    bottles: bottles::State,
    #[cfg(feature = "fvs")]
    snapshots: snapshots::State,
    profiles: profiles::State,
    library: library::State,
    settings: bottle_settings::State,
    effects_suspended: bool,
}

#[derive(Clone)]
pub enum Action {
    PrimaryTabSelected(PrimaryTab),
    DetailTabSelected(DetailTab),
    BottleSelected(Uuid),
    Back,
    AddBottle,
    CancelBottle,
    OpenMenu,
    TogglePower,
    DragWindow,
    Bottles(bottles::Message),
    Settings(bottle_settings::Message),
    Library(library::Message),
    Profiles(profiles::Message),
}

#[derive(Clone)]
pub enum Event {
    Bottles(bottles::Message),
    Settings(bottle_settings::Message),
    #[cfg(feature = "fvs")]
    Snapshots(snapshots::Message),
    Library(library::Message),
    Profiles(profiles::Message),
}

impl State {
    pub fn new(core: &Bottles, domain: &DomainSnapshots) -> (Self, Task<Event>) {
        let bottle_manager = core.bottles().clone();
        let profiles_manager = core.profiles().clone();
        let profiles = profiles::State::new(profiles_manager, domain.profiles().selected());
        let mut library = library::State::new(core.library().clone());
        let library_boot = library.reload().map(Event::Library);

        let state = Self {
            route: Route::Bottles,
            panel: None,
            bottles: bottles::State::new(bottle_manager, core.addons()),
            #[cfg(feature = "fvs")]
            snapshots: snapshots::State::default(),
            profiles,
            library,
            settings: bottle_settings::State::default(),
            effects_suspended: false,
        };

        (state, library_boot)
    }

    fn reload_library(&mut self) -> Task<Event> {
        if self.effects_suspended {
            return Task::none();
        }
        self.library.reload().map(Event::Library)
    }

    fn primary_tab(&self) -> PrimaryTab {
        match self.route {
            Route::Library => PrimaryTab::Library,
            _ => PrimaryTab::Bottles,
        }
    }

    pub fn has_active_operations(&self) -> bool {
        let active = self.bottles.has_active_operation()
            || self.profiles.has_active_operation()
            || self.settings.has_active_operation()
            || self.library.has_active_operation();
        #[cfg(feature = "fvs")]
        let active = active || self.snapshots.has_active_operation();
        active
    }

    pub fn cancel_active_operations(&mut self) {
        self.effects_suspended = true;
        self.profiles.dismiss_dialogs();
        self.bottles.cancel_creation();
        self.library.cancel_active_operations();
        #[cfg(feature = "fvs")]
        self.snapshots.cancel_active_operations();
    }

    pub fn resume_after_failed_switch(&mut self, _domain: &DomainSnapshots) -> Task<Event> {
        self.effects_suspended = false;
        let library = self.reload_library();
        #[cfg(feature = "fvs")]
        if let Some(snapshot) = self.selected_bottle(_domain) {
            let snapshots = self
                .snapshots
                .load(snapshot.bottle.clone())
                .map(Event::Snapshots);
            return Task::batch([library, snapshots]);
        }
        library
    }

    pub fn update_action(&mut self, action: Action, domain: &DomainSnapshots) -> Task<Event> {
        match action {
            Action::PrimaryTabSelected(tab) => {
                self.leave_bottle_route();
                self.route = match tab {
                    PrimaryTab::Bottles => Route::Bottles,
                    PrimaryTab::Library => Route::Library,
                };
                if tab == PrimaryTab::Library {
                    return self.reload_library();
                }
            }
            Action::DetailTabSelected(tab) => {
                if let Route::Bottle { id, .. } = self.route {
                    self.route = Route::Bottle { id, tab };
                }
            }
            Action::BottleSelected(id) => {
                if domain.bottle(id).is_none() {
                    return Task::none();
                }
                self.route = Route::Bottle {
                    id,
                    tab: DetailTab::Programs,
                };
                #[cfg(feature = "fvs")]
                self.snapshots.clear();
                #[cfg(feature = "fvs")]
                if let Some(snapshot) = self.selected_bottle(domain) {
                    return self
                        .snapshots
                        .load(snapshot.bottle.clone())
                        .map(Event::Snapshots);
                }
            }
            Action::Back => {
                self.profiles.dismiss_dialogs();
                if self.panel_is_open(PanelKind::Profiles) {
                    self.close_panel();
                } else {
                    self.leave_bottle_route();
                    self.route = Route::Bottles;
                }
            }
            Action::AddBottle => {
                self.profiles.dismiss_dialogs();
                self.panel = Some(PanelState::open(PanelKind::NewBottle));
                self.bottles.reset_creation();
            }
            Action::CancelBottle => {
                self.bottles.cancel_creation();
                if self.panel_is_open(PanelKind::NewBottle) {
                    self.close_panel();
                }
            }
            Action::OpenMenu | Action::TogglePower | Action::DragWindow => {}
            Action::Bottles(message) => {
                let (task, output) = self.bottles.update(message);
                let task = task.map(Event::Bottles);
                if matches!(output, Some(bottles::Output::Created))
                    && self.panel_is_open(PanelKind::NewBottle)
                {
                    self.close_panel();
                }
                return task;
            }
            Action::Settings(message) => return self.update_settings(message, domain),
            Action::Profiles(message) => {
                let (task, output) = self.profiles.update(message, domain.profiles().selected());
                if matches!(output, Some(profiles::Output::ToggleSettings)) {
                    if self.panel_is_open(PanelKind::Profiles) {
                        self.close_panel();
                        self.profiles.dismiss_dialogs();
                    } else {
                        self.panel = Some(PanelState::open(PanelKind::Profiles));
                    }
                }
                return task.map(Event::Profiles);
            }
            Action::Library(message) => return self.library.update(message).map(Event::Library),
        }

        Task::none()
    }

    pub fn update_event(&mut self, event: Event, domain: &DomainSnapshots) -> Task<Event> {
        match event {
            Event::Bottles(message) => {
                let (task, output) = self.bottles.update(message);
                if matches!(output, Some(bottles::Output::Created))
                    && self.panel_is_open(PanelKind::NewBottle)
                {
                    self.close_panel();
                }
                task.map(Event::Bottles)
            }
            Event::Settings(message) => self.update_settings(message, domain),
            #[cfg(feature = "fvs")]
            Event::Snapshots(message) => self.snapshots.update(message).map(Event::Snapshots),
            Event::Library(message) => self.library.update(message).map(Event::Library),
            Event::Profiles(message) => {
                let (task, _) = self.profiles.update(message, domain.profiles().selected());
                task.map(Event::Profiles)
            }
        }
    }

    pub fn bottles_changed(&mut self, domain: &DomainSnapshots) -> Task<Event> {
        let selected_exists = match self.route {
            Route::Bottle { id, .. } => domain.bottle(id).is_some(),
            Route::Bottles | Route::Library => true,
        };
        if reconcile_route(&mut self.route, selected_exists) {
            self.leave_bottle_route();
        }
        self.reload_library()
    }

    pub fn profiles_changed(
        &mut self,
        domain: &DomainSnapshots,
        selected_changed: bool,
    ) -> Task<Event> {
        if selected_changed {
            self.profiles.sync_selected(domain.profiles().selected());
        }
        self.reload_library()
    }

    fn update_settings(
        &mut self,
        message: bottle_settings::Message,
        domain: &DomainSnapshots,
    ) -> Task<Event> {
        let selected = self
            .selected_bottle(domain)
            .map(|snapshot| (snapshot.bottle.clone(), snapshot.state.as_ref()));
        self.settings.update(message, selected).map(Event::Settings)
    }

    fn panel_is_open(&self, kind: PanelKind) -> bool {
        self.panel
            .is_some_and(|panel| panel.open && panel.kind == kind)
    }

    fn close_panel(&mut self) {
        if let Some(panel) = &mut self.panel {
            panel.close();
        }
    }

    fn leave_bottle_route(&mut self) {
        #[cfg(feature = "fvs")]
        self.snapshots.clear();
    }

    fn selected_bottle<'a>(&self, domain: &'a DomainSnapshots) -> Option<&'a BottleSnapshot> {
        let Route::Bottle { id, .. } = self.route else {
            return None;
        };
        domain.bottle(id)
    }

    pub fn view<'a>(&'a self, domain: &'a DomainSnapshots) -> Element<'a, Action> {
        let panel = self.panel.unwrap_or(PanelState {
            kind: PanelKind::NewBottle,
            open: false,
        });
        let split = side_panel(
            match panel.kind {
                PanelKind::Profiles => Side::End,
                PanelKind::NewBottle => Side::Start,
            },
            panel.open,
            |base_context| {
                navigation_split(
                    base_context,
                    matches!(self.route, Route::Bottle { .. }),
                    |context| self.primary_page(context, domain),
                    |context| self.detail_page(context, domain),
                )
            },
            move |context| {
                if panel.kind == PanelKind::Profiles {
                    self.profile_settings_page(context, domain)
                } else {
                    self.new_bottle_page(context)
                }
            },
        );

        container(split).width(Fill).height(Fill).into()
    }

    pub(crate) fn dialog(&self) -> Option<Dialog<'_, Action>> {
        if !self.panel_is_open(PanelKind::Profiles) {
            return None;
        }

        profiles_view::dialog(&self.profiles)
            .map(|dialog| dialog.map(Action::Profiles))
            .or_else(|| {
                accounts_view::dialog(&self.profiles).map(|dialog| dialog.map(Action::Profiles))
            })
    }

    fn primary_page<'a>(
        &'a self,
        context: PaneContext,
        domain: &'a DomainSnapshots,
    ) -> Element<'a, Action> {
        let tabs = Tabs::new(
            [
                Tab::new(PrimaryTab::Bottles, "Bottles"),
                Tab::new(PrimaryTab::Library, "Library"),
            ],
            Some(self.primary_tab()),
            Action::PrimaryTabSelected,
        );
        let header = context.header(Action::DragWindow, |header| {
            header
                .start(header_button("Add bottle", Icon::Plus, Action::AddBottle))
                .middle(tabs)
                .end(profiles_view::switcher(domain.profiles()).map(Action::Profiles))
        });
        let content: Element<'_, Action> = match self.primary_tab() {
            PrimaryTab::Bottles => bottles_view::rows(
                domain.bottles(),
                match self.route {
                    Route::Bottle { id, .. } => Some(id),
                    _ => None,
                },
                Action::BottleSelected,
            ),
            PrimaryTab::Library => library_view::view(&self.library).map(Action::Library),
        };

        column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn profile_settings_page<'a>(
        &'a self,
        context: PaneContext,
        domain: &'a DomainSnapshots,
    ) -> Element<'a, Action> {
        let header = context.header(Action::DragWindow, |header| {
            header
                .start(header_button("Cancel", Icon::Arrow, Action::Back))
                .middle(
                    container(
                        Title::new("Profile Settings")
                            .subtitle("Manage your profiles and linked accounts."),
                    )
                    .padding(iced::padding::bottom(12)),
                )
                .end(header_button(
                    "New profile",
                    Icon::Plus,
                    Action::Profiles(profiles::Message::OpenCreate),
                ))
        });

        let content: Element<'_, Action> = {
            let active = domain.profiles().selected();
            let links = accounts_view::links(&self.profiles, active).map(Action::Profiles);

            column![
                RowGroup::new()
                    .title("Experience")
                    .row(
                        ActionRow::new("Classic", None)
                            .description("Current experience")
                            .icon(Icon::Checkmark),
                    )
                    .row(
                        ActionRow::new("Next", None)
                            .description("Not available yet")
                            .icon(Icon::Wand),
                    ),
                RowGroup::new()
                    .title("Profile")
                    .row(
                        TextRow::new("Profile name", self.profiles.name_draft())
                            .icon(Icon::Person)
                            .on_input(|name| {
                                Action::Profiles(profiles::Message::NameChanged(name))
                            })
                            .on_submit(Action::Profiles(profiles::Message::RenameSubmit,)),
                    )
                    .row(accounts_view::action_button_row(
                        Icon::Cross,
                        "Delete profile",
                        "Removes this profile and its linked accounts from this device",
                        "Delete",
                        Action::Profiles(profiles::Message::DeleteProfile(active.id(),)),
                    )),
                links,
            ]
            .spacing(18)
            .into()
        };
        let content: Element<'_, Action> = if let Some(error) = self.profiles.last_error() {
            column![
                InfoCard::new(InfoCardKind::Error, "Could not update profile", error,).width(Fill),
                content,
            ]
            .spacing(18)
            .into()
        } else {
            content
        };

        let page: Element<'_, Action> = column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into();

        page
    }

    fn new_bottle_page(&self, context: PaneContext) -> Element<'_, Action> {
        let header = context.header(Action::DragWindow, |header| {
            header
                .start(header_button(
                    "Cancel bottle creation",
                    Icon::Arrow,
                    Action::CancelBottle,
                ))
                .middle(
                    container(Title::new("New Bottle").subtitle("Creating a new bottle."))
                        .padding(iced::padding::bottom(12)),
                )
                .end(
                    header_button(
                        "Create bottle",
                        Icon::Checkmark,
                        Action::Bottles(bottles::Message::CreateBottle),
                    )
                    .on_press_maybe(
                        (!self.bottles.is_creating())
                            .then_some(Action::Bottles(bottles::Message::CreateBottle)),
                    ),
                )
        });
        let content = bottles_view::creation(&self.bottles).map(Action::Bottles);

        column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn detail_page<'a>(
        &'a self,
        context: PaneContext,
        domain: &'a DomainSnapshots,
    ) -> Element<'a, Action> {
        #[cfg(feature = "fvs")]
        let detail_tabs = [
            Tab::new(DetailTab::Programs, "Programs"),
            Tab::new(DetailTab::Settings, "Settings"),
            Tab::new(DetailTab::Snapshots, "Snapshots"),
        ];
        #[cfg(not(feature = "fvs"))]
        let detail_tabs = [
            Tab::new(DetailTab::Programs, "Programs"),
            Tab::new(DetailTab::Settings, "Settings"),
        ];
        let tabs = Tabs::new(
            detail_tabs,
            Some(match self.route {
                Route::Bottle { tab, .. } => tab,
                _ => DetailTab::Programs,
            }),
            Action::DetailTabSelected,
        );
        let header = context.header(Action::DragWindow, |mut header| {
            if context.is_standalone() {
                header = header.start(
                    Button::icon_only("Back to bottles", Icon::Arrow)
                        .diameter(32.0)
                        .icon_size(16.0)
                        .kind(ButtonKind::Transparent)
                        .on_press(Action::Back),
                );
            }

            header
                .start(header_button(
                    "More actions",
                    Icon::EllipsisVertical,
                    Action::OpenMenu,
                ))
                .start(header_button(
                    "Toggle power",
                    Icon::Power,
                    Action::TogglePower,
                ))
                .middle(tabs)
        });
        let detail_tab = match self.route {
            Route::Bottle { tab, .. } => tab,
            _ => DetailTab::Programs,
        };
        let content = match detail_tab {
            DetailTab::Programs => {
                if let Some(snapshot) = self.selected_bottle(domain) {
                    bottles_view::programs(
                        &self.bottles,
                        snapshot.bottle.clone(),
                        snapshot.state.as_ref(),
                    )
                    .map(Action::Bottles)
                } else {
                    column![].into()
                }
            }
            DetailTab::Settings => settings_view::view(
                &self.settings,
                self.selected_bottle(domain)
                    .map(|snapshot| snapshot.state.as_ref()),
            )
            .map(Action::Settings),
            #[cfg(feature = "fvs")]
            DetailTab::Snapshots => snapshots_view::view(&self.snapshots),
        };

        let mut body = column![scroll_content(content)].width(Fill).height(Fill);

        if let Some(snapshot) = self.selected_bottle(domain) {
            let state = snapshot.state.as_ref();
            body = body.push(
                keyed_column([(state.id(), bottles_view::status(state).map(Action::Bottles))])
                    .width(Fill),
            );
        }

        let body = container(body)
            .width(Fill)
            .height(Fill)
            .style(theme::panel)
            .clip(true);

        column![header, body].width(Fill).height(Fill).into()
    }
}

fn reconcile_route(route: &mut Route, selected_exists: bool) -> bool {
    if matches!(route, Route::Bottle { .. }) && !selected_exists {
        *route = Route::Bottles;
        true
    } else {
        false
    }
}

fn scroll_panel<'a>(content: impl Into<Element<'a, Action>>) -> Element<'a, Action> {
    container(scroll_content(content))
        .width(Fill)
        .height(Fill)
        .style(theme::panel)
        .clip(true)
        .into()
}

fn scroll_content<'a>(content: impl Into<Element<'a, Action>>) -> Element<'a, Action> {
    let content = container(content).width(Fill).max_width(CONTENT_MAX_WIDTH);
    let content = container(content).padding(24).center_x(Fill);

    container(
        scrollable(content)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new()
                    .width(4)
                    .scroller_width(4)
                    .margin(12),
            ))
            .style(theme::scrollbar)
            .width(Fill)
            .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .into()
}

fn header_button(label: &str, icon: Icon, message: Action) -> Button<'_, Action> {
    Button::icon_only(label, icon)
        .diameter(32.0)
        .icon_size(16.0)
        .kind(ButtonKind::Transparent)
        .on_press(message)
}
