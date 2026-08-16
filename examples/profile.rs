//! Profile switcher + management screen, driven directly by
//! `bottles_core::profile::ProfileManager` (in-process, no RPC).
//!
//! Linking a storefront account here fabricates a `LinkedAccount` locally
//! instead of performing a real login handshake: per `ProfileManager`'s own
//! module docs, that handshake is deliberately a multi-process concern
//! (Store plugins via `next-server`'s `ProfileService`), not something this
//! local persistence layer does. This screen is structured so a real
//! "begin login" step can be dropped in ahead of `Message::LinkAccount`
//! later without changing anything else.

use std::sync::Arc;

use bottles_core::profile::ProfileManager;
use iced::{
    Element, Fill, Subscription, Task, Theme,
    futures::StreamExt as _,
    widget::{column, container},
};
use next_proto::bottles::{
    common::v1::{AuthState, LinkedAccount, Storefront},
    profiles::v1::{SteamLink, UserProfile, profile_event},
};
use next_ui::{
    components::{
        action_row::{ActionRow, State as RowState},
        button::{Button, ButtonKind},
        header_bar::HeaderBar,
        picker_row::PickerRow,
        popover::{Popover, PopoverItem},
        row_group::RowGroup,
        text_row::TextRow,
        title::Title,
        window_frame,
    },
    icons::Icon,
    theme,
};

const STOREFRONTS: &[Storefront] = &[
    Storefront::Steam,
    Storefront::EpicGames,
    Storefront::Gog,
    Storefront::AmazonGames,
    Storefront::EaApp,
    Storefront::UbisoftConnect,
    Storefront::BattleNet,
];

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Bottles Next profiles")
        .theme(App::theme)
        .subscription(App::subscription)
        .style(|_, current_theme| theme::application(current_theme))
        .window_size((900.0, 760.0))
        .centered()
        .decorations(false)
        .transparent(true)
        .run()
}

#[derive(Clone)]
struct ManagerHandle(Arc<ProfileManager>);

impl std::hash::Hash for ManagerHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.0) as usize).hash(state);
    }
}

struct App {
    manager: Option<ManagerHandle>,
    profiles: Vec<UserProfile>,
    active: Option<UserProfile>,
    name_draft: String,
    switcher_open: bool,
    link_open: bool,
    steam_open: bool,
    steam_candidates: Vec<bottles_core::steam::SteamUser>,
    error: Option<String>,
}

#[derive(Clone)]
enum Message {
    ManagerLoaded(Result<Arc<ProfileManager>, String>),
    ProfilesLoaded(Vec<UserProfile>),
    ProfileEvent(profile_event::Event),
    ToggleSwitcher,
    ToggleLink,
    ToggleSteam,
    SteamCandidatesDetected(Vec<bottles_core::steam::SteamUser>),
    Dismiss,
    ActivateProfile(String),
    CreateProfile,
    NameChanged(String),
    RenameSubmit,
    UnlinkAccount(i32),
    LinkAccount(Storefront),
    UnlinkSteam,
    LinkSteam(String, String),
    ProfileUpdated(Result<UserProfile, String>),
    Window(window_frame::Action),
    Noop,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let state = Self {
            manager: None,
            profiles: Vec::new(),
            active: None,
            name_draft: String::new(),
            switcher_open: false,
            link_open: false,
            steam_open: false,
            steam_candidates: Vec::new(),
            error: None,
        };
        let boot = Task::perform(
            async {
                ProfileManager::load()
                    .await
                    .map(Arc::new)
                    .map_err(|err| err.to_string())
            },
            Message::ManagerLoaded,
        );

        (state, boot)
    }

    fn theme(&self) -> Theme {
        theme::theme()
    }

    fn subscription(&self) -> Subscription<Message> {
        let Some(handle) = self.manager.clone() else {
            return Subscription::none();
        };

        Subscription::run_with(handle, profile_events)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ManagerLoaded(Ok(manager)) => {
                self.manager = Some(ManagerHandle(manager.clone()));

                return Task::perform(
                    async move {
                        let profiles = manager.list().await;

                        if profiles.is_empty() {
                            let profile = manager
                                .create("Player".into(), "person".into())
                                .await
                                .map_err(|err| err.to_string())?;
                            manager
                                .apply_activation(&profile.id, Default::default())
                                .await
                                .map_err(|err| err.to_string())?;
                            Ok(manager.list().await)
                        } else {
                            Ok(profiles)
                        }
                    },
                    |result: Result<Vec<UserProfile>, String>| match result {
                        Ok(profiles) => Message::ProfilesLoaded(profiles),
                        Err(_) => Message::ProfilesLoaded(Vec::new()),
                    },
                );
            }
            Message::ManagerLoaded(Err(err)) => self.error = Some(err),
            Message::ProfilesLoaded(profiles) => {
                self.active = profiles
                    .iter()
                    .find(|profile| Some(&profile.id) == self.active.as_ref().map(|p| &p.id))
                    .or_else(|| profiles.first())
                    .cloned();
                self.name_draft = self
                    .active
                    .as_ref()
                    .map(|profile| profile.name.clone())
                    .unwrap_or_default();
                self.profiles = profiles;
            }
            Message::ProfileEvent(profile_event::Event::Activated(profile)) => {
                upsert_profile(&mut self.profiles, profile.clone());
                self.name_draft = profile.name.clone();
                self.active = Some(profile);
            }
            Message::ProfileEvent(profile_event::Event::Updated(profile)) => {
                upsert_profile(&mut self.profiles, profile.clone());

                if self.active.as_ref().is_some_and(|active| active.id == profile.id) {
                    self.name_draft = profile.name.clone();
                    self.active = Some(profile);
                }
            }
            Message::ProfileEvent(profile_event::Event::DeletedProfileId(id)) => {
                self.profiles.retain(|profile| profile.id != id);
                if self.active.as_ref().is_some_and(|active| active.id == id) {
                    self.active = self.profiles.first().cloned();
                }
            }
            Message::ToggleSwitcher => {
                self.switcher_open = !self.switcher_open;
                self.link_open = false;
                self.steam_open = false;
            }
            Message::ToggleLink => {
                self.link_open = !self.link_open;
                self.switcher_open = false;
                self.steam_open = false;
            }
            Message::ToggleSteam => {
                self.steam_open = !self.steam_open;
                self.switcher_open = false;
                self.link_open = false;

                if self.steam_open {
                    return Task::perform(
                        async { detect_steam_users() },
                        Message::SteamCandidatesDetected,
                    );
                }
            }
            Message::SteamCandidatesDetected(candidates) => self.steam_candidates = candidates,
            Message::Dismiss => {
                self.switcher_open = false;
                self.link_open = false;
                self.steam_open = false;
            }
            Message::ActivateProfile(id) => {
                self.switcher_open = false;

                if let Some(handle) = self.manager.clone() {
                    return Task::perform(
                        async move {
                            handle
                                .0
                                .apply_activation(&id, Default::default())
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::CreateProfile => {
                self.switcher_open = false;

                if let Some(handle) = self.manager.clone() {
                    return Task::perform(
                        async move {
                            let profile = handle
                                .0
                                .create("New profile".into(), "person".into())
                                .await
                                .map_err(|err| err.to_string())?;
                            handle
                                .0
                                .apply_activation(&profile.id, Default::default())
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::NameChanged(name) => self.name_draft = name,
            Message::RenameSubmit => {
                if let (Some(handle), Some(active)) = (self.manager.clone(), self.active.clone())
                {
                    let name = self.name_draft.clone();
                    return Task::perform(
                        async move {
                            handle
                                .0
                                .rename(&active.id, name)
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::UnlinkAccount(storefront) => {
                if let (Some(handle), Some(active)) = (self.manager.clone(), self.active.clone())
                {
                    return Task::perform(
                        async move {
                            handle
                                .0
                                .unlink_account(&active.id, storefront)
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::LinkAccount(storefront) => {
                self.link_open = false;

                if let (Some(handle), Some(active)) = (self.manager.clone(), self.active.clone())
                {
                    let account = LinkedAccount {
                        storefront: storefront as i32,
                        account_display_name: format!(
                            "{} on {}",
                            active.name,
                            storefront_label(storefront)
                        ),
                        account_id: uuid::Uuid::new_v4().to_string(),
                        auth_state: AuthState::Active as i32,
                        linked_at: None,
                        last_verified_at: None,
                        expires_at: None,
                    };

                    return Task::perform(
                        async move {
                            handle
                                .0
                                .link_account(&active.id, account)
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::UnlinkSteam => {
                if let (Some(handle), Some(active)) = (self.manager.clone(), self.active.clone())
                {
                    return Task::perform(
                        async move {
                            handle
                                .0
                                .unlink_steam(&active.id)
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::LinkSteam(steam_id64, account_name) => {
                self.steam_open = false;

                if let (Some(handle), Some(active)) = (self.manager.clone(), self.active.clone())
                {
                    return Task::perform(
                        async move {
                            handle
                                .0
                                .link_steam(
                                    &active.id,
                                    SteamLink {
                                        steam_id64,
                                        account_name,
                                    },
                                )
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::ProfileUpdated(Ok(profile)) => {
                upsert_profile(&mut self.profiles, profile.clone());
                self.name_draft = profile.name.clone();
                self.active = Some(profile);
            }
            Message::ProfileUpdated(Err(err)) => self.error = Some(err),
            Message::Window(action) => return action.task(),
            Message::Noop => {}
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let active_label = self
            .active
            .as_ref()
            .map(|profile| profile.name.as_str())
            .unwrap_or("No profile");
        let switcher_trigger = Button::new(active_label)
            .icon(Icon::Person)
            .kind(ButtonKind::Surface)
            .on_press(Message::ToggleSwitcher);
        let mut switcher = Popover::new(switcher_trigger, self.switcher_open)
            .on_dismiss(Message::Dismiss)
            .footer("New profile", Message::CreateProfile);

        for profile in &self.profiles {
            let mut item = PopoverItem::new(&profile.name)
                .icon(Icon::Person)
                .on_select(Message::ActivateProfile(profile.id.clone()));

            if let Some(account) = profile.accounts.first() {
                item = item.subtitle(&account.account_display_name);
            }

            switcher = switcher.add(item);
        }

        let header = HeaderBar::new(Message::Window)
            .show_window_controls(true)
            .middle(Title::new("Profiles").subtitle("Manage local profiles and linked accounts"))
            .end(switcher);

        let content: Element<'_, Message> = if let Some(active) = &self.active {
            let mut accounts = RowGroup::new().title("Linked accounts");

            for account in &active.accounts {
                let storefront = Storefront::try_from(account.storefront).unwrap_or_default();
                accounts = accounts.add(
                    ActionRow::new(
                        &account.account_display_name,
                        RowState::Ready(Message::UnlinkAccount(account.storefront)),
                    )
                    .description(auth_state_label(account.auth_state))
                    .icon(storefront_icon(storefront)),
                );
            }

            let link_trigger = PickerRow::new("Link a storefront account")
                .description("Choose the account to connect")
                .on_press(Message::ToggleLink);
            let mut link_popover = Popover::new(link_trigger, self.link_open)
                .on_dismiss(Message::Dismiss)
                .footer("Not listed, install manually", Message::Noop);

            for storefront in STOREFRONTS {
                if active
                    .accounts
                    .iter()
                    .any(|account| account.storefront == *storefront as i32)
                {
                    continue;
                }

                link_popover = link_popover.add(
                    PopoverItem::new(storefront_label(*storefront))
                        .icon(storefront_icon(*storefront))
                        .action("Link", Message::LinkAccount(*storefront)),
                );
            }

            let steam_row: Element<'_, Message> = if let Some(link) = &active.steam_link {
                ActionRow::new(&link.account_name, RowState::Ready(Message::UnlinkSteam))
                    .description("Linked Steam account")
                    .icon(Icon::Computer)
                    .into()
            } else {
                let steam_trigger = PickerRow::new("Link Steam account")
                    .description("Detected from your local Steam installation")
                    .on_press(Message::ToggleSteam);
                let mut steam_popover =
                    Popover::new(steam_trigger, self.steam_open).on_dismiss(Message::Dismiss);

                for user in &self.steam_candidates {
                    steam_popover = steam_popover.add(
                        PopoverItem::new(&user.account_name).icon(Icon::Computer).action(
                            "Link",
                            Message::LinkSteam(user.steam_id64.clone(), user.account_name.clone()),
                        ),
                    );
                }

                steam_popover.into()
            };

            column![
                RowGroup::new().title("Profile").add(
                    TextRow::new("Profile name", &self.name_draft)
                        .icon(Icon::Person)
                        .on_input(Message::NameChanged)
                        .on_submit(Message::RenameSubmit),
                ),
                accounts,
                container(link_popover).width(Fill),
                container(steam_row).width(Fill),
            ]
            .spacing(18)
            .into()
        } else {
            column![].into()
        };

        let body = container(content).width(Fill).padding(24);

        window_frame::WindowFrame::new(
            column![header, body].width(Fill).height(Fill),
            Message::Window,
        )
        .into()
    }
}

fn profile_events(
    handle: &ManagerHandle,
) -> std::pin::Pin<Box<dyn iced::futures::Stream<Item = Message> + Send>> {
    let manager = handle.0.clone();

    Box::pin(
        manager
            .watch()
            .filter_map(|event| async move { event.event.map(Message::ProfileEvent) }),
    )
}

fn upsert_profile(profiles: &mut Vec<UserProfile>, profile: UserProfile) {
    if let Some(existing) = profiles.iter_mut().find(|existing| existing.id == profile.id) {
        *existing = profile;
    } else {
        profiles.push(profile);
    }
}

fn detect_steam_users() -> Vec<bottles_core::steam::SteamUser> {
    bottles_core::steam::loginusers_vdf_path()
        .and_then(|path| bottles_core::steam::parse_loginusers(&path).ok())
        .unwrap_or_default()
}

fn storefront_label(storefront: Storefront) -> &'static str {
    match storefront {
        Storefront::Steam => "Steam",
        Storefront::EpicGames => "Epic Games Store",
        Storefront::Gog => "GOG",
        Storefront::AmazonGames => "Amazon Games",
        Storefront::EaApp => "EA App",
        Storefront::UbisoftConnect => "Ubisoft Connect",
        Storefront::BattleNet => "Battle.net",
        Storefront::Unspecified => "Unknown storefront",
    }
}

fn storefront_icon(storefront: Storefront) -> Icon {
    match storefront {
        Storefront::Steam => Icon::Computer,
        Storefront::EpicGames | Storefront::Gog | Storefront::AmazonGames => Icon::Disk,
        Storefront::EaApp | Storefront::UbisoftConnect | Storefront::BattleNet => Icon::Controller,
        Storefront::Unspecified => Icon::Warning,
    }
}

fn auth_state_label(state: i32) -> &'static str {
    match AuthState::try_from(state).unwrap_or_default() {
        AuthState::Active => "Connected",
        AuthState::Stale => "Needs re-authentication",
        AuthState::Inactive => "Signed out",
        AuthState::Unspecified => "Unknown",
    }
}
