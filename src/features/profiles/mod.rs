//! Profile management: the switcher popover, the profile list/activation
//! machinery, rename/delete, and the new-profile dialog. Storefront/Steam
//! login and account linking stay in `features::accounts` (extracted in a
//! later step) — this module owns everything else about profiles
//! themselves, including the `profile_events` subscription.
//!
//! `active_profile` is owned here; the shell reads it (via
//! [`State::active_profile`]) to drive cross-feature effects such as
//! resetting the library when the active profile changes, since profiles
//! must never call other features directly.

use std::sync::Arc;

use bottles_core::{accounts::AccountManager, profile::ProfileManager, steam::SteamManager};
use iced::{
    Element, Task,
    futures::StreamExt as _,
    widget::{column, row},
};

use next_proto::bottles::profiles::v1::{UserProfile, profile_event};

use crate::{
    icons::Icon,
    widgets::{
        button::{Button, ButtonKind},
        popover::{Popover, PopoverItem},
        text_row::TextRow,
        title::Title,
    },
};

#[derive(Clone)]
pub struct ProfileManagerHandle {
    profile: Arc<ProfileManager>,
    accounts: Arc<AccountManager>,
    steam: Arc<SteamManager>,
}

impl std::hash::Hash for ProfileManagerHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.profile) as usize).hash(state);
    }
}

impl ProfileManagerHandle {
    fn new(profile: Arc<ProfileManager>) -> Self {
        Self {
            accounts: Arc::new(AccountManager::new((*profile).clone())),
            steam: Arc::new(SteamManager::new((*profile).clone())),
            profile,
        }
    }

    /// Exposes the underlying manager for other features (namely
    /// `features::accounts`) that need to issue mutations but don't own
    /// the manager themselves.
    pub fn manager(&self) -> &Arc<ProfileManager> {
        &self.profile
    }

    pub fn accounts(&self) -> &Arc<AccountManager> {
        &self.accounts
    }

    pub fn steam(&self) -> &Arc<SteamManager> {
        &self.steam
    }
}

pub fn profile_events(
    handle: &ProfileManagerHandle,
) -> std::pin::Pin<Box<dyn iced::futures::Stream<Item = Message> + Send>> {
    let manager = handle.profile.clone();

    Box::pin(
        manager
            .watch_active_profile()
            .filter_map(|event| async move { event.event.map(Message::ProfileEvent) }),
    )
}

#[derive(Clone)]
pub enum Message {
    ProfileManagerLoaded(Result<Arc<ProfileManager>, String>),
    ProfilesLoaded(Vec<UserProfile>),
    ProfileEvent(profile_event::Event),
    ToggleProfileSwitcher,
    ToggleProfileSettings,
    ActivateProfile(String),
    ToggleNewProfile,
    CancelNewProfile,
    NewProfileNameChanged(String),
    SubmitNewProfile,
    ProfileUpdated(Result<UserProfile, String>),
    NameChanged(String),
    RenameSubmit,
    DeleteProfile(String),
    ProfileDeleted(Result<(), String>),
}

pub struct State {
    profile_manager: Option<ProfileManagerHandle>,
    profiles: Vec<UserProfile>,
    active_profile: Option<UserProfile>,
    profile_switcher_open: bool,
    name_draft: String,
    /// The new-profile dialog's draft name, or `None` when the dialog is
    /// closed.
    new_profile_draft: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            profile_manager: None,
            profiles: Vec::new(),
            active_profile: None,
            profile_switcher_open: false,
            name_draft: String::new(),
            new_profile_draft: None,
        }
    }

    pub fn boot() -> Task<Message> {
        Task::perform(
            async {
                ProfileManager::open()
                    .await
                    .map(Arc::new)
                    .map_err(|err| err.to_string())
            },
            Message::ProfileManagerLoaded,
        )
    }

    pub fn active_profile(&self) -> Option<&UserProfile> {
        self.active_profile.as_ref()
    }

    pub fn profiles(&self) -> &[UserProfile] {
        &self.profiles
    }

    pub fn name_draft(&self) -> &str {
        &self.name_draft
    }

    pub fn new_profile_draft(&self) -> Option<&str> {
        self.new_profile_draft.as_deref()
    }

    pub fn manager_handle(&self) -> Option<ProfileManagerHandle> {
        self.profile_manager.clone()
    }

    /// Applies a freshly-activated/updated profile, mirroring it into the
    /// profile list. Used both for profile-management flows handled here
    /// and by account-linking flows still living in the shell (which call
    /// this directly after a successful `ProfileUpdated`-shaped result).
    pub fn set_active_profile(&mut self, profile: UserProfile) {
        upsert_profile(&mut self.profiles, profile.clone());
        self.name_draft = profile.name.clone();
        self.active_profile = Some(profile);
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ProfileManagerLoaded(Ok(manager)) => {
                self.profile_manager = Some(ProfileManagerHandle::new(manager.clone()));
                let list_manager = manager.clone();

                let activate = Task::perform(
                    async move {
                        if let Some(active) = manager.active().await {
                            return Ok(active);
                        }

                        let profiles = manager.list().await;
                        let profile = match profiles.into_iter().next() {
                            Some(profile) => profile,
                            None => manager
                                .create("Player".into(), "person".into())
                                .await
                                .map_err(|err| err.to_string())?,
                        };

                        manager
                            .activate(&profile.id)
                            .await
                            .map_err(|err| err.to_string())
                    },
                    Message::ProfileUpdated,
                );
                let list = Task::perform(
                    async move { list_manager.list().await },
                    Message::ProfilesLoaded,
                );

                return Task::batch([activate, list]);
            }
            Message::ProfilesLoaded(profiles) => self.profiles = profiles,
            Message::ProfileManagerLoaded(Err(err)) => {
                eprintln!("failed to load profile manager: {err}");
            }
            Message::ProfileEvent(profile_event::Event::Activated(profile)) => {
                self.set_active_profile(profile);
            }
            Message::ProfileEvent(profile_event::Event::Updated(profile)) => {
                upsert_profile(&mut self.profiles, profile.clone());

                if self
                    .active_profile
                    .as_ref()
                    .is_some_and(|active| active.id == profile.id)
                {
                    self.active_profile = Some(profile);
                }
            }
            Message::ProfileEvent(profile_event::Event::DeletedProfileId(id)) => {
                self.profiles.retain(|profile| profile.id != id);

                if self
                    .active_profile
                    .as_ref()
                    .is_some_and(|active| active.id == id)
                {
                    self.active_profile = None;
                    self.name_draft.clear();

                    if let (Some(handle), Some(fallback)) =
                        (self.profile_manager.clone(), self.profiles.first().cloned())
                    {
                        return Task::perform(
                            async move {
                                handle
                                    .profile
                                    .activate(&fallback.id)
                                    .await
                                    .map_err(|err| err.to_string())
                            },
                            Message::ProfileUpdated,
                        );
                    }
                }
            }
            Message::ToggleProfileSwitcher => {
                self.profile_switcher_open = !self.profile_switcher_open
            }
            Message::ToggleProfileSettings => {}
            Message::ActivateProfile(id) => {
                if let Some(handle) = self.profile_manager.clone() {
                    return Task::perform(
                        async move { handle.profile.activate(&id).await.map_err(|err| err.to_string()) },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::ToggleNewProfile => {
                self.new_profile_draft = Some(String::new());
            }
            Message::CancelNewProfile => self.new_profile_draft = None,
            Message::NewProfileNameChanged(name) => {
                if let Some(draft) = &mut self.new_profile_draft {
                    *draft = name;
                }
            }
            Message::SubmitNewProfile => {
                let Some(draft) = self.new_profile_draft.take() else {
                    return Task::none();
                };

                if let Some(handle) = self.profile_manager.clone() {
                    let name = if draft.trim().is_empty() {
                        "New profile".to_string()
                    } else {
                        draft.trim().to_string()
                    };

                    return Task::perform(
                        async move {
                            let profile = handle
                                .profile
                                .create(name, "person".into())
                                .await
                                .map_err(|err| err.to_string())?;
                            handle
                                .profile
                                .activate(&profile.id)
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::ProfileUpdated(Ok(profile)) => {
                self.new_profile_draft = None;
                self.set_active_profile(profile);
            }
            Message::ProfileUpdated(Err(err)) => {
                eprintln!("failed to update profile: {err}");
            }
            Message::NameChanged(name) => self.name_draft = name,
            Message::RenameSubmit => {
                if let (Some(handle), Some(active)) =
                    (self.profile_manager.clone(), self.active_profile.clone())
                {
                    let name = self.name_draft.clone();

                    return Task::perform(
                        async move {
                            handle
                                .profile
                                .rename(&active.id, name)
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::DeleteProfile(id) => {
                if let Some(handle) = self.profile_manager.clone() {
                    return Task::perform(
                        async move { handle.profile.delete(&id).await.map_err(|err| err.to_string()) },
                        Message::ProfileDeleted,
                    );
                }
            }
            Message::ProfileDeleted(Err(err)) => eprintln!("failed to delete profile: {err}"),
            Message::ProfileDeleted(Ok(())) => {}
        }

        Task::none()
    }

    pub fn view_switcher(&self) -> Element<'_, Message> {
        let label = self
            .active_profile
            .as_ref()
            .map(|profile| profile.name.as_str())
            .unwrap_or("No profile");
        let trigger = Button::icon_only(label, Icon::Person)
            .diameter(32.0)
            .icon_size(16.0)
            .kind(ButtonKind::Transparent)
            .on_press(Message::ToggleProfileSwitcher);

        let mut switcher = Popover::new(trigger, self.profile_switcher_open)
            .on_dismiss(Message::ToggleProfileSwitcher)
            .footer("Profiles", Message::ToggleProfileSettings);

        for profile in &self.profiles {
            let selected = self
                .active_profile
                .as_ref()
                .is_some_and(|active| active.id == profile.id);

            switcher = switcher.add(
                PopoverItem::new(&profile.name)
                    .icon(Icon::Person)
                    .selected(selected)
                    .on_select(Message::ActivateProfile(profile.id.clone())),
            );
        }

        switcher.into()
    }
}

pub fn new_profile_dialog(name: &str) -> Element<'_, Message> {
    use crate::theme;

    iced::widget::container(
        column![
            Title::new("New profile").subtitle("Give this profile a name."),
            TextRow::new("Profile name", name)
                .icon(Icon::Person)
                .on_input(Message::NewProfileNameChanged)
                .on_submit(Message::SubmitNewProfile),
            row![
                Button::new("Create")
                    .kind(ButtonKind::Primary)
                    .on_press(Message::SubmitNewProfile),
                Button::new("Cancel")
                    .kind(ButtonKind::Transparent)
                    .on_press(Message::CancelNewProfile),
            ]
            .spacing(12),
        ]
        .spacing(18),
    )
    .width(420)
    .padding(24)
    .style(theme::panel)
    .into()
}

fn upsert_profile(profiles: &mut Vec<UserProfile>, profile: UserProfile) {
    if let Some(existing) = profiles
        .iter_mut()
        .find(|existing| existing.id == profile.id)
    {
        *existing = profile;
    } else {
        profiles.push(profile);
    }
}
