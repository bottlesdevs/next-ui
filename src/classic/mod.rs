//! The complete Classic experience: routes, dialogs, read models, and workflows.
//!
//! A future Next experience is a sibling of this module and cannot access these
//! private workflow modules.

use std::sync::Arc;

mod accounts;
mod bottles;
mod layout;
mod library;
mod profiles;
mod settings;
#[cfg(feature = "fvs")]
mod snapshots;

use crate::{
    Experience,
    icons::Icon,
    theme,
    ui::chrome,
    widgets::{
        action_row::{ActionRow, State as ActionRowState},
        button::{Button, ButtonKind},
        row_group::RowGroup,
        tabs::{Tab, Tabs},
        text_row::TextRow,
        title::Title,
    },
};
use bottles_core::{Bottle, BottleManager, BottleState, Bottles, Profiles, ProfilesConfig};
use iced::{
    Element, Fill, Subscription, Task,
    keyboard::{self, key},
    widget::{column, container, scrollable},
};
use layout::{PaneContext, Side, navigation_split, side_panel};
use uuid::Uuid;

const CONTENT_MAX_WIDTH: f32 = 1150.0;
const CONTENT_GRID_BREAKPOINT: f32 = 720.0;

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
enum Panel {
    NewBottle,
    Profiles,
}

enum Modal {
    NewProfile(profiles::NewProfileDialog),
    AccountLogin(accounts::LoginDialog),
}

struct ReadModel {
    bottles: Vec<Bottle>,
    bottle_states: Vec<Arc<BottleState>>,
    profiles: Arc<ProfilesConfig>,
}

impl ReadModel {
    fn new(bottle_manager: &BottleManager, profiles: &Profiles) -> Self {
        let bottles = bottle_manager.list();
        let bottle_states = bottles
            .iter()
            .filter_map(|bottle| bottle.state().ok())
            .collect();

        Self {
            bottles,
            bottle_states,
            profiles: profiles.snapshot(),
        }
    }

    fn set_bottles(&mut self, bottles: Vec<Bottle>) -> bool {
        if self.bottles.len() == bottles.len()
            && self
                .bottles
                .iter()
                .zip(&bottles)
                .all(|(current, next)| current.id() == next.id())
        {
            return false;
        }
        self.bottle_states = bottles
            .iter()
            .filter_map(|bottle| bottle.state().ok())
            .collect();
        self.bottles = bottles;
        true
    }

    fn set_bottle_state(&mut self, state: Arc<BottleState>) -> bool {
        if let Some(current) = self
            .bottle_states
            .iter_mut()
            .find(|current| current.id() == state.id())
        {
            if current.as_ref() == state.as_ref() {
                return false;
            }
            *current = state;
        } else {
            self.bottle_states.push(state);
        }
        true
    }

    fn bottle(&self, id: Uuid) -> Option<Bottle> {
        self.bottles
            .iter()
            .find(|bottle| bottle.id() == id)
            .cloned()
    }

    fn bottle_state(&self, id: Uuid) -> Option<&Arc<BottleState>> {
        self.bottle_states.iter().find(|state| state.id() == id)
    }
}

pub struct State {
    route: Route,
    panel: Panel,
    panel_open: bool,
    modal: Option<Modal>,
    read_model: ReadModel,
    bottles: bottles::State,
    #[cfg(feature = "fvs")]
    snapshots: snapshots::State,
    profiles: profiles::State,
    library: library::State,
    accounts: accounts::State,
    settings: settings::State,
    draining: bool,
}

#[derive(Clone)]
pub enum Message {
    PrimaryTabSelected(PrimaryTab),
    DetailTabSelected(DetailTab),
    BottleSelected(Uuid),
    Back,
    OpenNewProfile,
    NewProfileDialog(profiles::NewProfileMessage),
    AccountLoginDialog(accounts::LoginMessage),
    AddBottle,
    CancelBottle,
    OpenMenu,
    TogglePower,
    #[allow(dead_code)] // Constructed when the disabled Next settings row is enabled.
    RequestExperience(Experience),
    Window(chrome::Action),
    MoveFocus(bool),
    Bottles(bottles::Message),
    Settings(settings::Message),
    #[cfg(feature = "fvs")]
    Snapshots(snapshots::Message),
    Library(library::Message),
    Profiles(profiles::Message),
    Accounts(accounts::Message),
    BottleListChanged(Vec<Bottle>),
    BottleStateChanged(Arc<BottleState>),
    ProfilesChanged(Arc<ProfilesConfig>),
}

impl Message {
    fn allowed_while_draining(&self) -> bool {
        match self {
            Self::Bottles(
                bottles::Message::BottleCreation(_) | bottles::Message::ProgramLaunched(_),
            )
            | Self::Library(
                library::Message::Entry { .. }
                | library::Message::Loaded(_)
                | library::Message::Launched(_),
            )
            | Self::Profiles(
                profiles::Message::ProfileUpdated { .. } | profiles::Message::ProfileDeleted { .. },
            )
            | Self::Accounts(accounts::Message::ProfileUpdated(_))
            | Self::BottleListChanged(_)
            | Self::BottleStateChanged(_)
            | Self::ProfilesChanged(_) => true,
            #[cfg(target_os = "linux")]
            Self::Settings(settings::Message::WrapperUpdated { .. }) => true,
            #[cfg(feature = "fvs")]
            Self::Snapshots(snapshots::Message::Loaded { .. }) => true,
            _ => false,
        }
    }
}

impl Modal {
    fn cancel_message(&self) -> Message {
        match self {
            Self::NewProfile(_) => Message::NewProfileDialog(profiles::NewProfileMessage::Cancel),
            Self::AccountLogin(_) => Message::AccountLoginDialog(accounts::LoginMessage::Cancel),
        }
    }
}

impl State {
    pub fn new(core: &Bottles) -> (Self, Task<Message>) {
        let bottle_manager = core.bottles().clone();
        let profiles_manager = core.profiles().clone();
        let read_model = ReadModel::new(&bottle_manager, &profiles_manager);
        let profiles = profiles::State::new(profiles_manager, read_model.profiles.selected());
        let mut library = library::State::new(core.library().clone());
        let library_boot = library.reload().map(Message::Library);

        let state = Self {
            route: Route::Bottles,
            panel: Panel::NewBottle,
            panel_open: false,
            modal: None,
            read_model,
            bottles: bottles::State::new(bottle_manager, core.addons()),
            #[cfg(feature = "fvs")]
            snapshots: snapshots::State::new(),
            profiles,
            library,
            accounts: accounts::State::new(),
            settings: settings::State::new(),
            draining: false,
        };

        (state, library_boot)
    }

    fn reload_library(&mut self) -> Task<Message> {
        if self.draining {
            return Task::none();
        }
        self.library.reload().map(Message::Library)
    }

    fn primary_tab(&self) -> PrimaryTab {
        match self.route {
            Route::Library => PrimaryTab::Library,
            _ => PrimaryTab::Bottles,
        }
    }

    pub fn experience(&self) -> Experience {
        Experience::Classic
    }

    pub fn has_active_operations(&self) -> bool {
        let active = self.bottles.has_active_operation()
            || self.profiles.has_active_operation()
            || self.accounts.has_active_operation()
            || self.settings.has_active_operation()
            || self.library.has_active_operation();
        #[cfg(feature = "fvs")]
        let active = active || self.snapshots.has_active_operation();
        active
    }

    pub(crate) fn has_modal(&self) -> bool {
        self.modal.is_some()
    }

    pub(crate) fn dismiss_modal(&mut self) -> Task<Message> {
        let Some(modal) = &self.modal else {
            return Task::none();
        };

        self.update(modal.cancel_message())
    }

    pub fn cancel_active_operations(&mut self) {
        self.draining = true;
        self.modal = None;
        self.bottles.cancel_creation();
        self.accounts.cancel_active_operation();
        self.library.cancel_active_operations();
        #[cfg(feature = "fvs")]
        self.snapshots.cancel_active_operations();
    }

    pub fn resume_after_failed_switch(&mut self) -> Task<Message> {
        self.draining = false;
        let library = self.reload_library();
        #[cfg(feature = "fvs")]
        if let Some(bottle) = self.selected_bottle_handle() {
            let snapshots = self.snapshots.load(bottle).map(Message::Snapshots);
            return Task::batch([library, snapshots]);
        }
        library
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        if self.draining && !message.allowed_while_draining() {
            return Task::none();
        }
        match message {
            Message::PrimaryTabSelected(tab) => {
                self.route = match tab {
                    PrimaryTab::Bottles => Route::Bottles,
                    PrimaryTab::Library => Route::Library,
                };
                if tab == PrimaryTab::Library {
                    return self.reload_library();
                }
            }
            Message::DetailTabSelected(tab) => {
                if let Route::Bottle { id, .. } = self.route {
                    self.route = Route::Bottle { id, tab };
                }
            }
            Message::BottleSelected(id) => {
                self.route = Route::Bottle {
                    id,
                    tab: DetailTab::Programs,
                };
                #[cfg(feature = "fvs")]
                self.snapshots.clear();

                #[cfg(feature = "fvs")]
                if let Some(bottle) = self.selected_bottle_handle() {
                    return self.snapshots.load(bottle).map(Message::Snapshots);
                }
            }
            Message::Back => {
                self.modal = None;
                self.accounts.cancel_active_operation();
                if self.panel_open && self.panel == Panel::Profiles {
                    self.panel_open = false;
                } else {
                    self.route = Route::Bottles;
                }
            }
            Message::OpenNewProfile => {
                if !self.profiles.has_active_operation() {
                    self.accounts.cancel_active_operation();
                    self.modal = Some(Modal::NewProfile(profiles::NewProfileDialog::new()));
                }
            }
            Message::NewProfileDialog(profiles::NewProfileMessage::NameChanged(name)) => {
                if let Some(Modal::NewProfile(dialog)) = &mut self.modal {
                    dialog.set_name(name);
                }
            }
            Message::NewProfileDialog(profiles::NewProfileMessage::Submit) => {
                if self.profiles.has_active_operation() {
                    return Task::none();
                }
                let Some(Modal::NewProfile(dialog)) = &mut self.modal else {
                    return Task::none();
                };
                dialog.clear_error();
                let submission = dialog.submission();
                return self
                    .profiles
                    .update(profiles::Message::Create(submission))
                    .0
                    .map(Message::Profiles);
            }
            Message::NewProfileDialog(profiles::NewProfileMessage::Cancel) => {
                if matches!(self.modal, Some(Modal::NewProfile(_))) {
                    self.modal = None;
                }
            }
            Message::AccountLoginDialog(accounts::LoginMessage::Cancel) => {
                if matches!(self.modal, Some(Modal::AccountLogin(_))) {
                    self.modal = None;
                    self.accounts.cancel_active_operation();
                }
            }
            Message::AccountLoginDialog(message) => {
                let Some(Modal::AccountLogin(dialog)) = &mut self.modal else {
                    return Task::none();
                };
                match message {
                    accounts::LoginMessage::CodeChanged(code) => dialog.set_code(code),
                    accounts::LoginMessage::OpenUrl => accounts::open_url(dialog.url()),
                    accounts::LoginMessage::CopyUrl => {
                        return iced::clipboard::write(dialog.url().to_owned());
                    }
                    accounts::LoginMessage::Submit => dialog.submit(),
                    accounts::LoginMessage::Cancel => {}
                }
            }
            Message::AddBottle => {
                self.panel = Panel::NewBottle;
                self.panel_open = true;
                self.bottles.reset_creation();
            }
            Message::CancelBottle => {
                self.bottles.cancel_creation();
                self.panel_open = false;
            }
            Message::Window(action) => {
                if self.modal.is_some() {
                    return Task::none();
                }
                return action.task().unwrap_or_else(Task::none);
            }
            Message::MoveFocus(previous) => {
                return if previous {
                    iced::widget::operation::focus_previous()
                } else {
                    iced::widget::operation::focus_next()
                };
            }
            Message::Bottles(message) => {
                let (task, output) = self.bottles.update(message);
                let task = task.map(Message::Bottles);
                return match output {
                    Some(bottles::Output::Created) => {
                        if self.panel_open && self.panel == Panel::NewBottle {
                            self.panel_open = false;
                        }
                        task
                    }
                    None => task,
                };
            }
            #[cfg(feature = "fvs")]
            Message::Snapshots(message) => {
                return self.snapshots.update(message).map(Message::Snapshots);
            }
            Message::Settings(message) => {
                let selected_id = match self.route {
                    Route::Bottle { id, .. } => Some(id),
                    _ => None,
                };
                #[cfg(target_os = "linux")]
                let bottle = selected_id.and_then(|id| self.read_model.bottle(id));
                let bottle_state = selected_id.and_then(|id| self.read_model.bottle_state(id));
                let ctx = settings::Context {
                    #[cfg(target_os = "linux")]
                    bottle,
                    bottle_state,
                };
                return self.settings.update(message, &ctx).map(Message::Settings);
            }
            Message::Profiles(message) => {
                let (task, output) = self.profiles.update(message);
                let task = task.map(Message::Profiles);
                return match output {
                    Some(profiles::Output::ToggleSettings) => {
                        if self.panel_open && self.panel == Panel::Profiles {
                            self.panel_open = false;
                        } else {
                            self.panel = Panel::Profiles;
                            self.panel_open = true;
                        }
                        task
                    }
                    Some(profiles::Output::CreateFinished(Ok(()))) => {
                        if matches!(self.modal, Some(Modal::NewProfile(_))) {
                            self.modal = None;
                        }
                        task
                    }
                    Some(profiles::Output::CreateFinished(Err(error))) => {
                        if let Some(Modal::NewProfile(dialog)) = &mut self.modal {
                            dialog.set_error(error.to_string());
                        }
                        task
                    }
                    None => task,
                };
            }
            Message::Accounts(message) => {
                let ctx = accounts::Context {
                    active_profile: self.read_model.profiles.selected(),
                    profiles: self.profiles.manager(),
                };
                let (task, output) = self.accounts.update(message, &ctx);
                let task = task.map(Message::Accounts);
                return match output {
                    Some(accounts::Output::LoginRequested(dialog)) => {
                        self.modal = Some(Modal::AccountLogin(dialog));
                        task
                    }
                    Some(accounts::Output::LinkFinished) => {
                        if matches!(self.modal, Some(Modal::AccountLogin(_))) {
                            self.modal = None;
                        }
                        task
                    }
                    None => task,
                };
            }
            Message::Library(message) => {
                let (task, output) = self.library.update(message);
                return if matches!(output, Some(library::Output::Reload)) {
                    Task::batch([task.map(Message::Library), self.reload_library()])
                } else {
                    task.map(Message::Library)
                };
            }
            Message::BottleListChanged(bottles) => {
                return if self.read_model.set_bottles(bottles) {
                    self.reload_library()
                } else {
                    Task::none()
                };
            }
            Message::BottleStateChanged(state) => {
                return if self.read_model.set_bottle_state(state) {
                    self.reload_library()
                } else {
                    Task::none()
                };
            }
            Message::ProfilesChanged(snapshot) => {
                if self.read_model.profiles == snapshot {
                    return Task::none();
                }
                let selected_changed = {
                    let current = self.read_model.profiles.selected();
                    let next = snapshot.selected();
                    current.id() != next.id() || current.name() != next.name()
                };
                if selected_changed {
                    self.profiles.sync_selected(snapshot.selected());
                }
                self.read_model.profiles = snapshot;
                return self.reload_library();
            }
            Message::OpenMenu | Message::TogglePower | Message::RequestExperience(_) => {}
        }

        Task::none()
    }

    fn selected_bottle_handle(&self) -> Option<Bottle> {
        let Route::Bottle { id, .. } = self.route else {
            return None;
        };

        self.read_model.bottle(id)
    }

    fn selected_bottle_state(&self) -> Option<&Arc<BottleState>> {
        let Route::Bottle { id, .. } = self.route else {
            return None;
        };

        self.read_model.bottle_state(id)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let keys = keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Tab),
                modifiers,
                repeat: false,
                ..
            } => Some(Message::MoveFocus(modifiers.shift())),
            _ => None,
        });

        let mut subscriptions = vec![keys];

        subscriptions.push(
            Subscription::run_with(self.bottles.manager().clone(), bottles::bottle_events)
                .map(Message::BottleListChanged),
        );

        for bottle in &self.read_model.bottles {
            subscriptions.push(
                Subscription::run_with(bottle.clone(), bottles::bottle_state_events)
                    .map(Message::BottleStateChanged),
            );
        }

        subscriptions.push(
            Subscription::run_with(self.profiles.manager().clone(), profiles::profile_events)
                .map(Message::ProfilesChanged),
        );

        Subscription::batch(subscriptions)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let split = side_panel(
            match self.panel {
                Panel::Profiles => Side::End,
                Panel::NewBottle => Side::Start,
            },
            self.panel_open,
            |base_context| {
                navigation_split(
                    base_context,
                    matches!(self.route, Route::Bottle { .. }),
                    |context| self.primary_page(context),
                    |context| self.detail_page(context),
                )
            },
            |context| {
                if self.panel == Panel::Profiles {
                    self.profile_settings_page(context)
                } else {
                    self.new_bottle_page(context)
                }
            },
        );

        container(split).width(Fill).height(Fill).into()
    }

    pub(crate) fn modal_view(&self) -> Option<(Element<'_, Message>, Message)> {
        let modal = self.modal.as_ref()?;
        let content = match modal {
            Modal::AccountLogin(dialog) => {
                Element::from(dialog.view()).map(Message::AccountLoginDialog)
            }
            Modal::NewProfile(dialog) => {
                Element::from(dialog.view(self.profiles.creating_profile()))
                    .map(Message::NewProfileDialog)
            }
        };

        Some((content, modal.cancel_message()))
    }

    fn primary_page(&self, context: PaneContext) -> Element<'_, Message> {
        let tabs = Tabs::new(
            [
                Tab::new(PrimaryTab::Bottles, "Bottles"),
                Tab::new(PrimaryTab::Library, "Library"),
            ],
            Some(self.primary_tab()),
            Message::PrimaryTabSelected,
        );
        let header = context
            .header(Message::Window(chrome::Action::Drag))
            .start(header_button("Add bottle", Icon::Plus, Message::AddBottle))
            .middle(tabs)
            .end(
                self.profiles
                    .view_switcher(&self.read_model.profiles)
                    .map(Message::Profiles),
            );
        let content: Element<'_, Message> = match self.primary_tab() {
            PrimaryTab::Bottles => self
                .bottles
                .rows_view(&self.read_model.bottle_states, Message::BottleSelected),
            PrimaryTab::Library => self.library.view().map(Message::Library),
        };

        column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn profile_settings_page(&self, context: PaneContext) -> Element<'_, Message> {
        let header = context
            .header(Message::Window(chrome::Action::Drag))
            .start(header_button("Cancel", Icon::Arrow, Message::Back))
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
                Message::OpenNewProfile,
            ));

        let content: Element<'_, Message> = {
            let active = self.read_model.profiles.selected();
            let accounts_ctx = accounts::Context {
                active_profile: active,
                profiles: self.profiles.manager(),
            };
            let links = self
                .accounts
                .view_links(&accounts_ctx)
                .map(Message::Accounts);

            column![
                RowGroup::new()
                    .title("Experience")
                    .row(
                        ActionRow::new("Classic", ActionRowState::Disabled)
                            .description("Current experience")
                            .icon(Icon::Checkmark),
                    )
                    .row(
                        ActionRow::new("Next", ActionRowState::Disabled)
                            .description("Not available yet")
                            .icon(Icon::Wand),
                    ),
                RowGroup::new()
                    .title("Profile")
                    .row(
                        TextRow::new("Profile name", self.profiles.name_draft())
                            .icon(Icon::Person)
                            .on_input(|name| {
                                Message::Profiles(profiles::Message::NameChanged(name))
                            })
                            .on_submit(Message::Profiles(profiles::Message::RenameSubmit,)),
                    )
                    .row(accounts::action_button_row(
                        Icon::Cross,
                        "Delete profile",
                        "Removes this profile and its linked accounts from this device",
                        "Delete",
                        Message::Profiles(profiles::Message::DeleteProfile(active.id(),)),
                    )),
                links,
            ]
            .spacing(18)
            .into()
        };
        let content: Element<'_, Message> = if let Some(error) = self.profiles.last_error() {
            column![
                crate::widgets::info_card::InfoCard::new(
                    crate::widgets::info_card::Kind::Error,
                    "Could not update profile",
                    error,
                )
                .width(Fill),
                content,
            ]
            .spacing(18)
            .into()
        } else {
            content
        };

        let page: Element<'_, Message> = column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into();

        page
    }

    fn new_bottle_page(&self, context: PaneContext) -> Element<'_, Message> {
        let header = context
            .header(Message::Window(chrome::Action::Drag))
            .start(header_button(
                "Cancel bottle creation",
                Icon::Arrow,
                Message::CancelBottle,
            ))
            .middle(
                container(Title::new("New Bottle").subtitle("Creating a new bottle."))
                    .padding(iced::padding::bottom(12)),
            )
            .end(header_button(
                "Create bottle",
                Icon::Checkmark,
                Message::Bottles(bottles::Message::CreateBottle),
            ));
        let content = self.bottles.creation_view().map(Message::Bottles);

        column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn detail_page(&self, context: PaneContext) -> Element<'_, Message> {
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
            Message::DetailTabSelected,
        );
        let mut header = context.header(Message::Window(chrome::Action::Drag));

        if context.is_standalone() {
            header = header.start(
                Button::icon_only("Back to bottles", Icon::Arrow)
                    .diameter(32.0)
                    .icon_size(16.0)
                    .kind(ButtonKind::Transparent)
                    .on_press(Message::Back),
            );
        }

        let header = header
            .start(header_button(
                "More actions",
                Icon::EllipsisVertical,
                Message::OpenMenu,
            ))
            .start(header_button(
                "Toggle power",
                Icon::Power,
                Message::TogglePower,
            ))
            .middle(tabs);
        let settings_ctx = settings::Context {
            #[cfg(target_os = "linux")]
            bottle: self.selected_bottle_handle(),
            bottle_state: self.selected_bottle_state(),
        };
        let detail_tab = match self.route {
            Route::Bottle { tab, .. } => tab,
            _ => DetailTab::Programs,
        };
        let content = match detail_tab {
            DetailTab::Programs => {
                if let (Some(bottle), Some(state)) =
                    (self.selected_bottle_handle(), self.selected_bottle_state())
                {
                    self.bottles
                        .program_grid(bottle, state)
                        .map(Message::Bottles)
                } else {
                    column![].into()
                }
            }
            DetailTab::Settings => self.settings.view(&settings_ctx).map(Message::Settings),
            #[cfg(feature = "fvs")]
            DetailTab::Snapshots => self.snapshots.view().map(Message::Snapshots),
        };

        column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into()
    }
}

fn scroll_panel<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
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
    .style(theme::panel)
    .clip(true)
    .into()
}

fn header_button(label: &str, icon: Icon, message: Message) -> Button<'_, Message> {
    Button::icon_only(label, icon)
        .diameter(32.0)
        .icon_size(16.0)
        .kind(ButtonKind::Transparent)
        .on_press(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draining_accepts_task_messages_but_rejects_new_actions() {
        assert!(Message::Library(library::Message::Loaded(1)).allowed_while_draining());
        assert!(
            !Message::Library(library::Message::QueryChanged("new search".into()))
                .allowed_while_draining()
        );
        assert!(!Message::AddBottle.allowed_while_draining());
    }
}
