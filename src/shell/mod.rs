use std::sync::Arc;

use crate::{
    icons::Icon,
    theme,
    widgets::{
        button::{Button, ButtonKind},
        header_bar::HeaderBar,
        row_group::RowGroup,
        split_view::{PaneMode, PaneSide, SplitView},
        tabs::{Tab, Tabs},
        text_row::TextRow,
        title::Title,
        window_frame,
    },
};
use bottles_core::{Bottle, BottleState, Bottles};
use iced::{
    Background, Element, Fill, Padding, Subscription, Task, Theme,
    keyboard::{self, key},
    theme::Mode as ThemeMode,
    widget::{center, column, container, mouse_area, opaque, scrollable, stack},
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryTab {
    Bottles,
    Library,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Programs,
    Settings,
    Snapshots,
}

pub struct State {
    primary_tab: PrimaryTab,
    detail_tab: DetailTab,
    bottles: crate::features::bottles::State,
    split_view_state: SplitViewState,
    snapshots: crate::features::snapshots::State,
    profiles: crate::features::profiles::State,
    library: crate::features::library::State,
    accounts: crate::features::accounts::State,
    settings: crate::features::settings::State,
    system_theme: ThemeMode,
}

#[derive(Clone, PartialEq, Eq)]
pub enum SplitViewState {
    Bottle(Uuid),
    NewBottle,
    Profiles,
    None,
}

#[derive(Clone)]
pub enum Message {
    PrimaryTabSelected(PrimaryTab),
    DetailTabSelected(DetailTab),
    BottleSelected(Uuid),
    Back,
    AddBottle,
    CancelBottle,
    OpenMenu,
    TogglePower,
    Window(window_frame::Action),
    MoveFocus(bool),
    Bottles(crate::features::bottles::Message),
    Settings(crate::features::settings::Message),
    Snapshots(crate::features::snapshots::Message),
    Library(crate::features::library::Message),
    Profiles(crate::features::profiles::Message),
    Accounts(crate::features::accounts::Message),
    SystemThemeChanged(ThemeMode),
}

impl State {
    fn empty() -> Self {
        Self {
            primary_tab: PrimaryTab::Bottles,
            detail_tab: DetailTab::Programs,
            bottles: crate::features::bottles::State::new(),
            split_view_state: SplitViewState::None,
            snapshots: crate::features::snapshots::State::new(),
            profiles: crate::features::profiles::State::new(),
            library: crate::features::library::State::new(),
            accounts: crate::features::accounts::State::new(),
            settings: crate::features::settings::State::new(),
            system_theme: ThemeMode::default(),
        }
    }

    /// Resets the library for whatever profile is active now, if the
    /// active profile's identity actually changed since `previous`. Called
    /// after every `profiles` update (and after the account/login flows
    /// still living in the shell hand back a fresh profile) since only
    /// `features::profiles` knows the new active profile, but only the
    /// shell may reach into `library` — cross-feature effects are always
    /// intercepted here rather than let features call each other.
    fn sync_library_to_active_profile(&mut self, previous: Option<&str>) {
        let current = self
            .profiles
            .active_profile()
            .map(|profile| profile.id.as_str());

        if current == previous {
            return;
        }

        match self.profiles.active_profile() {
            Some(profile) => {
                let has_watchable_accounts =
                    !(profile.accounts.is_empty() && profile.steam_link.is_none());
                self.library.reset_for_profile(Some(has_watchable_accounts));
            }
            None => self.library.reset_for_profile(None),
        }
    }

    pub fn new() -> (Self, Task<Message>) {
        let bottles_boot = crate::features::bottles::State::boot().map(Message::Bottles);
        let profiles_boot = crate::features::profiles::State::boot().map(Message::Profiles);
        let theme_boot = iced::system::theme().map(Message::SystemThemeChanged);

        (
            Self::empty(),
            Task::batch([bottles_boot, profiles_boot, theme_boot]),
        )
    }

    pub fn new_with_bottles(bottles: Bottles) -> (Self, Task<Message>) {
        let mut state = Self::empty();
        state.bottles = crate::features::bottles::State::new_with_bottles(bottles);
        let theme_boot = iced::system::theme().map(Message::SystemThemeChanged);

        (
            state,
            Task::batch([
                crate::features::profiles::State::boot().map(Message::Profiles),
                theme_boot,
            ]),
        )
    }

    pub fn theme(&self) -> Theme {
        theme::BottlesTheme::for_mode(self.system_theme).theme
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PrimaryTabSelected(tab) => {
                self.primary_tab = tab;

                if tab == PrimaryTab::Library {
                    self.split_view_state = SplitViewState::None;
                }
            }
            Message::DetailTabSelected(tab) => self.detail_tab = tab,
            Message::BottleSelected(id) => {
                self.primary_tab = PrimaryTab::Bottles;
                self.split_view_state = SplitViewState::Bottle(id);
                self.snapshots.clear();

                if let Some(bottle) = self.selected_bottle_handle() {
                    return self.snapshots.load(bottle).map(Message::Snapshots);
                }
            }
            Message::Back => self.split_view_state = SplitViewState::None,
            Message::AddBottle => {
                self.split_view_state = SplitViewState::NewBottle;
                self.bottles.reset_creation();
            }
            Message::CancelBottle => self.split_view_state = SplitViewState::None,
            Message::Window(action) => return action.task(),
            Message::MoveFocus(previous) => {
                return if previous {
                    iced::widget::operation::focus_previous()
                } else {
                    iced::widget::operation::focus_next()
                };
            }
            Message::Bottles(message) => {
                let close_panel = matches!(
                    &message,
                    crate::features::bottles::Message::BottleCreated(Ok(_))
                );

                let task = if let crate::features::bottles::Message::LaunchProgram(id) = message {
                    self.selected_bottle_handle()
                        .map(|bottle| self.bottles.launch_program(bottle, id))
                        .unwrap_or_else(Task::none)
                } else {
                    self.bottles.update(message)
                };

                if close_panel {
                    self.split_view_state = SplitViewState::None;
                }

                return task.map(Message::Bottles);
            }
            Message::Snapshots(message) => {
                return self.snapshots.update(message).map(Message::Snapshots);
            }
            Message::Settings(message) => {
                let is_wrapper_updated_ok = matches!(
                    message,
                    crate::features::settings::Message::WrapperUpdated(Ok(()))
                );
                let selected_id = match self.split_view_state {
                    SplitViewState::Bottle(id) => Some(id),
                    _ => None,
                };
                let bottle = selected_id.and_then(|id| self.bottles.bottle_handle(id));
                let bottle_state = selected_id.and_then(|id| self.bottles.bottle_state(id));
                let ctx = crate::features::settings::Context {
                    bottle,
                    bottle_state,
                };
                let task = self.settings.update(message, &ctx).map(Message::Settings);

                if is_wrapper_updated_ok {
                    self.bottles.refresh_states();
                }

                return task;
            }
            Message::Profiles(crate::features::profiles::Message::ToggleProfileSettings) => {
                self.split_view_state = if self.split_view_state == SplitViewState::Profiles {
                    SplitViewState::None
                } else {
                    SplitViewState::Profiles
                };

                return self
                    .profiles
                    .update(crate::features::profiles::Message::ToggleProfileSettings)
                    .map(Message::Profiles);
            }
            Message::Profiles(message) => {
                let previous_active_id = self
                    .profiles
                    .active_profile()
                    .map(|profile| profile.id.clone());
                // Opening the new-profile dialog, submitting it, or a
                // successful profile update all also close the accounts
                // feature's login modal, since only one of the two
                // profile-pane overlays can be open at a time.
                let closes_login_modal = matches!(
                    message,
                    crate::features::profiles::Message::ToggleNewProfile
                        | crate::features::profiles::Message::SubmitNewProfile
                        | crate::features::profiles::Message::ProfileUpdated(Ok(_))
                );

                let task = self.profiles.update(message).map(Message::Profiles);

                if closes_login_modal {
                    self.accounts.close_login();
                }

                self.sync_library_to_active_profile(previous_active_id.as_deref());

                return task;
            }
            Message::Accounts(message) => {
                // `BeginLogin` opens the accounts feature's sign-in modal,
                // which is mutually exclusive with `profiles`' new-profile
                // dialog (previously enforced by both sharing one field).
                let opens_login_modal = matches!(
                    message,
                    crate::features::accounts::Message::LoginChallengeReceived(Ok(_))
                );
                // A successful account/Steam mutation hands back a fresh
                // profile — only `features::profiles` owns `active_profile`,
                // so the shell relays it in as a cross-feature effect
                // rather than letting `accounts` reach into `profiles`.
                let updated_profile = match &message {
                    crate::features::accounts::Message::ProfileUpdated(Ok(profile)) => {
                        Some(profile.clone())
                    }
                    _ => None,
                };

                let previous_active_id = self
                    .profiles
                    .active_profile()
                    .map(|profile| profile.id.clone());
                let ctx = crate::features::accounts::Context {
                    active_profile: self.profiles.active_profile(),
                    profiles: self.profiles.profiles(),
                    profile_manager: self.profiles.manager_handle(),
                };
                let mut tasks = vec![self.accounts.update(message, &ctx).map(Message::Accounts)];

                if opens_login_modal {
                    tasks.push(
                        self.profiles
                            .update(crate::features::profiles::Message::CancelNewProfile)
                            .map(Message::Profiles),
                    );
                }

                if let Some(profile) = updated_profile {
                    self.profiles.set_active_profile(profile);
                }

                self.sync_library_to_active_profile(previous_active_id.as_deref());

                return Task::batch(tasks);
            }
            Message::Library(message) => {
                return self.library.update(message).map(Message::Library);
            }
            Message::SystemThemeChanged(mode) => self.system_theme = mode,
            Message::OpenMenu | Message::TogglePower => {}
        }

        Task::none()
    }

    fn selected_bottle_handle(&self) -> Option<Bottle> {
        let SplitViewState::Bottle(id) = self.split_view_state else {
            return None;
        };

        self.bottles.bottle_handle(id)
    }

    fn selected_bottle_state(&self) -> Option<&Arc<BottleState>> {
        let SplitViewState::Bottle(id) = self.split_view_state else {
            return None;
        };

        self.bottles.bottle_state(id)
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

        let theme_changes = iced::system::theme_changes().map(Message::SystemThemeChanged);

        let mut subscriptions = vec![keys, theme_changes];

        if let Some(handle) = self.bottles.bottle_manager_handle() {
            subscriptions.push(
                Subscription::run_with(handle, crate::features::bottles::bottle_events)
                    .map(Message::Bottles),
            );
        }

        if let Some(handle) = self.profiles.manager_handle() {
            subscriptions.push(
                Subscription::run_with(handle, crate::features::profiles::profile_events)
                    .map(Message::Profiles),
            );
        }

        if let Some(profile) = self.profiles.active_profile() {
            let handle = crate::features::library::LibraryHandle(profile.id.clone());
            subscriptions.push(
                Subscription::run_with(handle, crate::features::library::library_events)
                    .map(Message::Library),
            );
        }

        Subscription::batch(subscriptions)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let split = SplitView::new(
            |_, _| {
                SplitView::new(
                    |width, mode| self.primary_page(width, mode),
                    |width, mode| self.detail_page(width, mode),
                )
                .show_detail(matches!(self.split_view_state, SplitViewState::Bottle(_)))
                .into()
            },
            |width, mode| {
                if matches!(self.split_view_state, SplitViewState::Profiles) {
                    self.profile_settings_page(width, mode)
                } else {
                    self.new_bottle_page(width, mode)
                }
            },
        )
        .side(match self.split_view_state {
            SplitViewState::Bottle(_) => PaneSide::Start,
            SplitViewState::NewBottle => PaneSide::Start,
            SplitViewState::Profiles => PaneSide::End,
            SplitViewState::None => PaneSide::Start,
        })
        .show_detail(matches!(
            self.split_view_state,
            SplitViewState::NewBottle | SplitViewState::Profiles
        ))
        .block_master();

        let content = container(split)
            .width(Fill)
            .height(Fill)
            .padding(Padding::ZERO.horizontal(12).bottom(12));

        window_frame::WindowFrame::new(content, Message::Window).into()
    }

    fn primary_page(&self, width: f32, mode: PaneMode) -> Element<'_, Message> {
        let tabs = Tabs::new(
            [
                Tab::new(PrimaryTab::Bottles, "Bottles"),
                Tab::new(PrimaryTab::Library, "Library"),
            ],
            Some(self.primary_tab),
            Message::PrimaryTabSelected,
        );
        let header = HeaderBar::new(Message::Window)
            .show_window_controls(if cfg!(target_os = "macos") {
                !matches!(
                    self.split_view_state,
                    SplitViewState::Bottle(_) | SplitViewState::NewBottle
                )
            } else {
                matches!(self.split_view_state, SplitViewState::None)
            })
            .start(header_button("Add bottle", Icon::Plus, Message::AddBottle))
            .middle(tabs)
            .end(self.profiles.view_switcher().map(Message::Profiles));
        let content: Element<'_, Message> = match self.primary_tab {
            PrimaryTab::Bottles => self.bottles.rows_view(width, mode, Message::BottleSelected),
            PrimaryTab::Library => self.library.view(width, mode).map(Message::Library),
        };

        column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn profile_settings_page(&self, _width: f32, mode: PaneMode) -> Element<'_, Message> {
        let header = HeaderBar::new(Message::Window)
            .show_window_controls(mode == PaneMode::Single)
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
                Message::Profiles(crate::features::profiles::Message::ToggleNewProfile),
            ));

        let content: Element<'_, Message> = if let Some(active) = self.profiles.active_profile() {
            let accounts_ctx = crate::features::accounts::Context {
                active_profile: Some(active),
                profiles: self.profiles.profiles(),
                profile_manager: self.profiles.manager_handle(),
            };
            let links = self
                .accounts
                .view_links(&accounts_ctx)
                .map(Message::Accounts);

            column![
                RowGroup::new()
                    .title("Profile")
                    .add(
                        TextRow::new("Profile name", self.profiles.name_draft())
                            .icon(Icon::Person)
                            .on_input(|name| {
                                Message::Profiles(crate::features::profiles::Message::NameChanged(
                                    name,
                                ))
                            })
                            .on_submit(Message::Profiles(
                                crate::features::profiles::Message::RenameSubmit,
                            )),
                    )
                    .add(crate::features::accounts::action_button_row(
                        Icon::Cross,
                        "Delete profile",
                        "Removes this profile and its linked accounts from this device",
                        "Delete",
                        Message::Profiles(crate::features::profiles::Message::DeleteProfile(
                            active.id.clone(),
                        )),
                    )),
                links,
            ]
            .spacing(18)
            .into()
        } else {
            column![].into()
        };

        let page: Element<'_, Message> = column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into();

        if let Some(login) = self.accounts.login_dialog() {
            return modal(
                page,
                login.map(Message::Accounts),
                Message::Accounts(crate::features::accounts::Message::CancelLogin),
            );
        }

        if let Some(draft) = self.profiles.new_profile_draft() {
            return modal(
                page,
                crate::features::profiles::new_profile_dialog(draft).map(Message::Profiles),
                Message::Profiles(crate::features::profiles::Message::CancelNewProfile),
            );
        }

        page
    }

    fn new_bottle_page(&self, _width: f32, mode: PaneMode) -> Element<'_, Message> {
        let header = HeaderBar::new(Message::Window)
            .show_window_controls(mode == PaneMode::Single)
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
                Message::Bottles(crate::features::bottles::Message::CreateBottle),
            ));
        let content = self.bottles.creation_view().map(Message::Bottles);

        column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn detail_page(&self, width: f32, mode: PaneMode) -> Element<'_, Message> {
        let tabs = Tabs::new(
            [
                Tab::new(DetailTab::Programs, "Programs"),
                Tab::new(DetailTab::Settings, "Settings"),
                Tab::new(DetailTab::Snapshots, "Snapshots"),
            ],
            Some(self.detail_tab),
            Message::DetailTabSelected,
        );
        let mut header =
            HeaderBar::new(Message::Window).show_window_controls(if cfg!(target_os = "macos") {
                matches!(
                    self.split_view_state,
                    SplitViewState::Bottle(_) | SplitViewState::NewBottle
                ) && mode == PaneMode::Single
            } else {
                matches!(self.split_view_state, SplitViewState::Bottle(_))
            });

        if mode == PaneMode::Single {
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
        let settings_ctx = crate::features::settings::Context {
            bottle: self.selected_bottle_handle(),
            bottle_state: self.selected_bottle_state(),
        };
        let content = match self.detail_tab {
            DetailTab::Programs => {
                if let SplitViewState::Bottle(id) = self.split_view_state {
                    self.bottles.program_grid(id, width).map(Message::Bottles)
                } else {
                    column![].into()
                }
            }
            DetailTab::Settings => self.settings.view(&settings_ctx).map(Message::Settings),
            DetailTab::Snapshots => self.snapshots.view(width).map(Message::Snapshots),
        };

        column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into()
    }
}

fn scroll_panel<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    let content = container(content).width(Fill).padding(24).center_x(Fill);

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

fn modal<'a>(
    base: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    on_dismiss: Message,
) -> Element<'a, Message> {
    stack![
        base.into(),
        opaque(
            mouse_area(center(opaque(content)).style(|_theme| container::Style {
                background: Some(Background::Color(theme::SCRIM)),
                ..container::Style::default()
            }))
            .on_press(on_dismiss)
        ),
    ]
    .into()
}
