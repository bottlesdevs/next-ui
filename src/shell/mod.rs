use std::sync::Arc;

use bottles_core::{
    Bottle, BottleManager, BottleState, Bottles, Config as CoreConfig, Progress, Slot,
    Storage, profile::ProfileManager,
};
use iced::{
    Background, Element, Fill, Padding, Subscription, Task, Theme,
    futures::{SinkExt as _, StreamExt as _},
    keyboard::{self, key},
    widget::{
        Column, center, column, container, image, mouse_area, opaque, row, scrollable, stack, svg,
        text,
    },
};
use next_proto::bottles::{
    common::v1::{AuthState, Game, Storefront},
    library::v1::{WatchGamesRequest, game_event, library_client::LibraryClient},
    profiles::v1::{
        LinkAccountRequest, SteamLink, UserProfile, profile_client::ProfileClient, profile_event,
    },
    registry::v1::{ResolvePluginRequest, registry_client::RegistryClient},
    store::v1::{BeginLoginRequest, login_challenge, store_client::StoreClient},
};
use crate::{
    components::{
        action_row::{ActionRow, State as ActionRowState},
        artwork_card::{ArtworkCard, CardAction},
        button::{Button, ButtonKind},
        header_bar::HeaderBar,
        info_card::{InfoCard, Kind},
        list_row::ListRow,
        picker_row::PickerRow,
        popover::{Popover, PopoverItem},
        row_group::RowGroup,
        selector_row::SelectorRow,
        split_view::{PaneMode, PaneSide, SplitView},
        status_bar::{StatusBar, StatusState},
        tabs::{Tab, Tabs},
        text::TextExt as _,
        text_row::TextRow,
        title::Title,
        window_frame,
    },
    icons::Icon,
    theme,
};
use uuid::Uuid;

const CONTENT_GRID_BREAKPOINT: f32 = 720.0;

const PURPOSES: [&str; 4] = ["Gaming", "Software", "Gaming (ULWGL)", "Custom"];
const ARCHITECTURES: [&str; 2] = ["Win64", "Win32"];
const SERVER_ENDPOINT: &str = "http://127.0.0.1:50052";
const REGISTRY_ENDPOINT: &str = "http://127.0.0.1:50250";

const STOREFRONTS: &[Storefront] = &[
    Storefront::Steam,
    Storefront::EpicGames,
    Storefront::Gog,
    Storefront::AmazonGames,
    Storefront::EaApp,
    Storefront::UbisoftConnect,
    Storefront::BattleNet,
];
const LOGIN_STOREFRONTS: &[Storefront] = &[Storefront::EpicGames, Storefront::Gog];

#[derive(Clone, PartialEq)]
pub struct RunnerOption {
    id: Uuid,
    label: String,
}

impl std::fmt::Display for RunnerOption {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.label)
    }
}

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
    bottles: Option<Bottles>,
    bottle_list: Vec<Bottle>,
    bottle_states: Vec<Arc<BottleState>>,
    split_view_state: SplitViewState,
    snapshots: crate::features::snapshots::State,
    bottle_name: String,
    runners: Vec<RunnerOption>,
    selected_runner: Option<RunnerOption>,
    purpose: &'static str,
    architecture: &'static str,
    creation_log: String,
    creation_log_expanded: bool,
    creation_failed: bool,
    profile_manager: Option<ProfileManagerHandle>,
    profiles: Vec<UserProfile>,
    active_profile: Option<UserProfile>,
    profile_switcher_open: bool,
    games: Vec<Game>,
    library_state: LibraryState,
    name_draft: String,
    account_link_popover: AccountLinkPopover,
    steam_candidates: Vec<bottles_core::steam::SteamUser>,
    profile_modal: ProfileModal,
    settings: crate::features::settings::State,
}

#[derive(Clone, PartialEq, Eq)]
pub enum SplitViewState {
    Bottle(Uuid),
    NewBottle,
    Profiles,
    None,
}

/// Whether the Library tab has anything to show yet for the active
/// profile — distinct from `games` being empty, since an empty list can
/// mean either "still waiting on the first `WatchGames` event" or
/// "loaded, and there's genuinely nothing linked".
#[derive(Clone, PartialEq, Eq)]
enum LibraryState {
    /// No active profile to load a library for.
    Idle,
    /// Waiting on the first event from `WatchGames`.
    Loading,
    /// At least one event has arrived (or the profile has nothing to
    /// watch in the first place), so an empty `games` list is meaningful.
    Loaded,
    Failed(String),
}

/// Which of the two storefront-picker popovers on the Profiles pane is
/// open, if any — `ToggleLink`/`ToggleSteam` are mutually exclusive.
#[derive(Clone, PartialEq, Eq)]
enum AccountLinkPopover {
    Closed,
    Storefront,
    Steam,
}

/// The Profiles pane's modal, if any — signing in to a storefront and
/// naming a new profile are mutually exclusive, so one field covers both
/// instead of an `Option<LoginChallenge>` alongside its own open flag.
#[derive(Clone, PartialEq, Eq, Default)]
enum ProfileModal {
    #[default]
    None,
    Login(LoginChallenge),
    NewProfile(String),
}

#[derive(Clone, PartialEq, Eq)]
struct LoginChallenge {
    storefront: Storefront,
    challenge_id: String,
    url: String,
    code_draft: String,
    submitting: bool,
}

#[derive(Clone)]
struct ProfileManagerHandle(Arc<ProfileManager>);

impl std::hash::Hash for ProfileManagerHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.0) as usize).hash(state);
    }
}

fn profile_events(
    handle: &ProfileManagerHandle,
) -> std::pin::Pin<Box<dyn iced::futures::Stream<Item = Message> + Send>> {
    let manager = handle.0.clone();

    Box::pin(
        manager
            .watch()
            .filter_map(|event| async move { event.event.map(Message::ProfileEvent) }),
    )
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct LibraryHandle(String);

fn library_events(
    handle: &LibraryHandle,
) -> std::pin::Pin<Box<dyn iced::futures::Stream<Item = Message> + Send>> {
    let profile_id = handle.0.clone();

    Box::pin(iced::stream::channel(
        16,
        move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
            let mut client = match LibraryClient::connect(SERVER_ENDPOINT).await {
                Ok(client) => client,
                Err(err) => {
                    let _ = output
                        .send(Message::LibraryError(format!(
                            "next-server unavailable: {err}"
                        )))
                        .await;
                    return;
                }
            };
            let response = client.watch_games(WatchGamesRequest { profile_id }).await;
            let mut events = match response {
                Ok(response) => response.into_inner(),
                Err(err) => {
                    let _ = output.send(Message::LibraryError(err.to_string())).await;
                    return;
                }
            };

            while let Some(event) = events.next().await {
                match event {
                    Ok(event) => {
                        if let Some(event) = event.event {
                            let _ = output.send(Message::LibraryEvent(event)).await;
                        }
                    }
                    Err(err) => {
                        let _ = output.send(Message::LibraryError(err.to_string())).await;
                        break;
                    }
                }
            }
        },
    ))
}

#[derive(Clone)]
struct BottleManagerHandle(BottleManager);

impl std::hash::Hash for BottleManagerHandle {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

fn bottle_events(
    handle: &BottleManagerHandle,
) -> std::pin::Pin<Box<dyn iced::futures::Stream<Item = Message> + Send>> {
    let manager = handle.0.clone();

    Box::pin(manager.watch().map(Message::BottleListChanged))
}

#[derive(Clone)]
pub enum Message {
    PrimaryTabSelected(PrimaryTab),
    DetailTabSelected(DetailTab),
    BottleSelected(Uuid),
    Back,
    AddBottle,
    CancelBottle,
    CreateBottle,
    BottleCreationProgress(Progress),
    BottleCreated(Result<Bottle, String>),
    ToggleCreationLog,
    BottleNameChanged(String),
    RunnerSelected(RunnerOption),
    PurposeSelected(&'static str),
    ArchitectureSelected(&'static str),
    OpenMenu,
    TogglePower,
    Window(window_frame::Action),
    MoveFocus(bool),
    BottlesLoaded(Result<Arc<Bottles>, String>),
    BottleListChanged(Vec<Bottle>),
    LaunchProgram(Uuid),
    ProgramLaunched(Result<u32, String>),
    Settings(crate::features::settings::Message),
    Snapshots(crate::features::snapshots::Message),
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
    LibraryEvent(game_event::Event),
    LibraryError(String),
    ToggleLink,
    ToggleSteam,
    SteamCandidatesDetected(Vec<bottles_core::steam::SteamUser>),
    Dismiss,
    NameChanged(String),
    RenameSubmit,
    UnlinkAccount(i32),
    BeginLogin(Storefront),
    LoginChallengeReceived(Result<(Storefront, String, String), String>),
    LoginCodeChanged(String),
    OpenLoginUrl,
    CopyLoginUrl,
    SubmitLoginCode,
    CancelLogin,
    UnlinkSteam,
    LinkSteam(String, String),
    DeleteProfile(String),
    ProfileDeleted(Result<(), String>),
    Noop,
}

impl State {
    fn empty() -> Self {
        Self {
            primary_tab: PrimaryTab::Bottles,
            detail_tab: DetailTab::Programs,
            bottles: None,
            bottle_list: Vec::new(),
            bottle_states: Vec::new(),
            split_view_state: SplitViewState::None,
            snapshots: crate::features::snapshots::State::new(),
            bottle_name: "Gaming paradise".into(),
            runners: Vec::new(),
            selected_runner: None,
            purpose: PURPOSES[0],
            architecture: ARCHITECTURES[0],
            creation_log: String::new(),
            creation_log_expanded: false,
            creation_failed: false,
            profile_manager: None,
            profiles: Vec::new(),
            active_profile: None,
            profile_switcher_open: false,
            games: Vec::new(),
            library_state: LibraryState::Idle,
            name_draft: String::new(),
            account_link_popover: AccountLinkPopover::Closed,
            steam_candidates: Vec::new(),
            profile_modal: ProfileModal::None,
            settings: crate::features::settings::State::new(),
        }
    }

    fn set_active_profile(&mut self, profile: UserProfile) {
        upsert_profile(&mut self.profiles, profile.clone());

        if self
            .active_profile
            .as_ref()
            .is_none_or(|active| active.id != profile.id)
        {
            self.games.clear();
            self.library_state = if profile.accounts.is_empty() && profile.steam_link.is_none() {
                // Nothing for `WatchGames` to ever report, so there's no
                // event to wait on — treat it as already loaded (empty).
                LibraryState::Loaded
            } else {
                LibraryState::Loading
            };
        }

        self.name_draft = profile.name.clone();
        self.active_profile = Some(profile);
    }

    fn profile_manager_boot() -> Task<Message> {
        Task::perform(
            async {
                ProfileManager::load()
                    .await
                    .map(Arc::new)
                    .map_err(|err| err.to_string())
            },
            Message::ProfileManagerLoaded,
        )
    }

    pub fn new() -> (Self, Task<Message>) {
        let bottles_boot = Task::perform(
            async {
                Bottles::open(CoreConfig::default())
                    .await
                    .map(Arc::new)
                    .map_err(|err| err.to_string())
            },
            Message::BottlesLoaded,
        );

        (
            Self::empty(),
            Task::batch([bottles_boot, Self::profile_manager_boot()]),
        )
    }

    pub fn new_with_bottles(bottles: Bottles) -> (Self, Task<Message>) {
        let mut state = Self::empty();
        state.apply_bottles(bottles);

        (state, Self::profile_manager_boot())
    }

    fn apply_bottles(&mut self, bottles: Bottles) {
        self.bottle_list = bottles.bottles().list();
        self.runners = bottles
            .addons()
            .components()
            .iter()
            .filter(|entry| entry.slot() == Slot::Runner)
            .map(|entry| RunnerOption {
                id: entry.id(),
                label: format!("{} {}", entry.name(), entry.version()),
            })
            .collect();
        self.selected_runner = self.runners.first().cloned();
        self.bottles = Some(bottles);
        self.refresh_bottle_states();
    }

    pub fn theme(&self) -> Theme {
        theme::theme()
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
                self.creation_log.clear();
                self.creation_failed = false;
            }
            Message::CancelBottle => self.split_view_state = SplitViewState::None,
            Message::CreateBottle => {
                if let (Some(bottles), Some(runner)) = (&self.bottles, self.selected_runner.clone())
                {
                    let name = self.bottle_name.clone();
                    let manager = bottles.bottles().clone();
                    let operation = manager.create(name, Storage::Standard, runner.id);
                    let progress = operation.progress();

                    self.creation_log.clear();
                    self.creation_failed = false;
                    self.creation_log_expanded = true;

                    let progress_task = Task::stream(progress).map(Message::BottleCreationProgress);
                    let result_task = Task::perform(
                        async move { operation.await.map_err(|err| err.to_string()) },
                        Message::BottleCreated,
                    );

                    return Task::batch([progress_task, result_task]);
                }
            }
            Message::BottleCreationProgress(progress) => {
                if !self.creation_log.is_empty() {
                    self.creation_log.push('\n');
                }
                self.creation_log.push_str(&progress_log_line(&progress));
            }
            Message::BottleCreated(Ok(_)) => self.split_view_state = SplitViewState::None,
            Message::BottleCreated(Err(err)) => {
                self.creation_failed = true;

                if !self.creation_log.is_empty() {
                    self.creation_log.push('\n');
                }
                self.creation_log
                    .push_str(&format!("{} Failed: {err}", timestamp()));

                eprintln!("failed to create bottle: {err}");
            }
            Message::ToggleCreationLog => {
                self.creation_log_expanded = !self.creation_log_expanded;
            }
            Message::BottleNameChanged(name) => self.bottle_name = name,
            Message::RunnerSelected(runner) => self.selected_runner = Some(runner),
            Message::PurposeSelected(purpose) => self.purpose = purpose,
            Message::ArchitectureSelected(architecture) => self.architecture = architecture,
            Message::Window(action) => return action.task(),
            Message::MoveFocus(previous) => {
                return if previous {
                    iced::widget::operation::focus_previous()
                } else {
                    iced::widget::operation::focus_next()
                };
            }
            Message::BottlesLoaded(Ok(bottles)) => {
                if let Ok(bottles) = Arc::try_unwrap(bottles) {
                    self.apply_bottles(bottles);
                }
            }
            Message::BottlesLoaded(Err(err)) => eprintln!("failed to open Bottles: {err}"),
            Message::BottleListChanged(list) => {
                self.bottle_list = list;
                self.refresh_bottle_states();
            }
            Message::Snapshots(message) => {
                return self.snapshots.update(message).map(Message::Snapshots);
            }
            Message::LaunchProgram(id) => {
                if let Some(bottle) = self.selected_bottle_handle() {
                    return Task::perform(
                        async move { bottle.run(id).await.map_err(|err| err.to_string()) },
                        Message::ProgramLaunched,
                    );
                }
            }
            Message::ProgramLaunched(Err(err)) => eprintln!("failed to launch program: {err}"),
            Message::Settings(message) => {
                let is_wrapper_updated_ok =
                    matches!(message, crate::features::settings::Message::WrapperUpdated(Ok(())));
                let selected_id = match self.split_view_state {
                    SplitViewState::Bottle(id) => Some(id),
                    _ => None,
                };
                let bottle = selected_id.and_then(|id| {
                    self.bottle_list
                        .iter()
                        .find(|bottle| bottle.state().is_ok_and(|state| state.id() == id))
                        .cloned()
                });
                let bottle_state = selected_id
                    .and_then(|id| self.bottle_states.iter().find(|state| state.id() == id));
                let ctx = crate::features::settings::Context {
                    bottle,
                    bottle_state,
                };
                let task = self.settings.update(message, &ctx).map(Message::Settings);

                if is_wrapper_updated_ok {
                    self.refresh_bottle_states();
                }

                return task;
            }
            Message::ProfileManagerLoaded(Ok(manager)) => {
                self.profile_manager = Some(ProfileManagerHandle(manager.clone()));
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
                            .apply_activation(&profile.id, Default::default())
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
                    self.games.clear();
                    self.library_state = LibraryState::Idle;

                    if let (Some(handle), Some(fallback)) =
                        (self.profile_manager.clone(), self.profiles.first().cloned())
                    {
                        return Task::perform(
                            async move {
                                handle
                                    .0
                                    .apply_activation(&fallback.id, Default::default())
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
            Message::ToggleProfileSettings => {
                self.split_view_state = if self.split_view_state == SplitViewState::Profiles {
                    SplitViewState::None
                } else {
                    SplitViewState::Profiles
                };
            }
            Message::ActivateProfile(id) => {
                if let Some(handle) = self.profile_manager.clone() {
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
            Message::ToggleNewProfile => {
                self.profile_modal = ProfileModal::NewProfile(String::new());
            }
            Message::CancelNewProfile => self.profile_modal = ProfileModal::None,
            Message::NewProfileNameChanged(name) => {
                if let ProfileModal::NewProfile(draft) = &mut self.profile_modal {
                    *draft = name;
                }
            }
            Message::SubmitNewProfile => {
                let ProfileModal::NewProfile(draft) = std::mem::take(&mut self.profile_modal)
                else {
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
                                .0
                                .create(name, "person".into())
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
            Message::ProfileUpdated(Ok(profile)) => {
                self.profile_modal = ProfileModal::None;
                self.set_active_profile(profile);
            }
            Message::ProfileUpdated(Err(err)) => {
                eprintln!("failed to update profile: {err}");

                if let ProfileModal::Login(login) = &mut self.profile_modal {
                    login.submitting = false;
                }
            }
            Message::ToggleLink => {
                self.account_link_popover = match self.account_link_popover {
                    AccountLinkPopover::Storefront => AccountLinkPopover::Closed,
                    AccountLinkPopover::Closed | AccountLinkPopover::Steam => {
                        AccountLinkPopover::Storefront
                    }
                };
            }
            Message::ToggleSteam => {
                self.account_link_popover = match self.account_link_popover {
                    AccountLinkPopover::Steam => AccountLinkPopover::Closed,
                    AccountLinkPopover::Closed | AccountLinkPopover::Storefront => {
                        AccountLinkPopover::Steam
                    }
                };

                if self.account_link_popover == AccountLinkPopover::Steam {
                    return Task::perform(
                        async { detect_steam_users() },
                        Message::SteamCandidatesDetected,
                    );
                }
            }
            Message::SteamCandidatesDetected(candidates) => self.steam_candidates = candidates,
            Message::Dismiss => self.account_link_popover = AccountLinkPopover::Closed,
            Message::NameChanged(name) => self.name_draft = name,
            Message::RenameSubmit => {
                if let (Some(handle), Some(active)) =
                    (self.profile_manager.clone(), self.active_profile.clone())
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
            Message::DeleteProfile(id) => {
                if let Some(handle) = self.profile_manager.clone() {
                    return Task::perform(
                        async move { handle.0.delete(&id).await.map_err(|err| err.to_string()) },
                        Message::ProfileDeleted,
                    );
                }
            }
            Message::ProfileDeleted(Err(err)) => eprintln!("failed to delete profile: {err}"),
            Message::ProfileDeleted(Ok(())) => {}
            Message::UnlinkAccount(storefront) => {
                if let (Some(handle), Some(active)) =
                    (self.profile_manager.clone(), self.active_profile.clone())
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
            Message::BeginLogin(storefront) => {
                self.account_link_popover = AccountLinkPopover::Closed;

                if let Some(active) = self.active_profile.clone() {
                    return Task::perform(
                        async move {
                            begin_login(active.id, storefront)
                                .await
                                .map(|(challenge_id, url)| (storefront, challenge_id, url))
                        },
                        Message::LoginChallengeReceived,
                    );
                }
            }
            Message::LoginChallengeReceived(Ok((storefront, challenge_id, url))) => {
                self.profile_modal = ProfileModal::Login(LoginChallenge {
                    storefront,
                    challenge_id,
                    url,
                    code_draft: String::new(),
                    submitting: false,
                });
            }
            Message::LoginChallengeReceived(Err(err)) => eprintln!("failed to start login: {err}"),
            Message::LoginCodeChanged(code) => {
                if let ProfileModal::Login(login) = &mut self.profile_modal {
                    login.code_draft = code;
                }
            }
            Message::OpenLoginUrl => {
                if let ProfileModal::Login(login) = &self.profile_modal {
                    open_url(&login.url);
                }
            }
            Message::CopyLoginUrl => {
                if let ProfileModal::Login(login) = &self.profile_modal {
                    return iced::clipboard::write(login.url.clone());
                }
            }
            Message::CancelLogin => self.profile_modal = ProfileModal::None,
            Message::SubmitLoginCode => {
                if let (Some(active), ProfileModal::Login(login)) =
                    (self.active_profile.clone(), &mut self.profile_modal)
                {
                    login.submitting = true;

                    let profile_id = active.id;
                    let challenge_id = login.challenge_id.clone();
                    let storefront = login.storefront;
                    let user_input = login.code_draft.clone();

                    return Task::perform(
                        async move {
                            complete_login(profile_id, challenge_id, storefront, user_input).await
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::UnlinkSteam => {
                if let (Some(handle), Some(active)) =
                    (self.profile_manager.clone(), self.active_profile.clone())
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
                self.account_link_popover = AccountLinkPopover::Closed;

                if let (Some(handle), Some(active)) =
                    (self.profile_manager.clone(), self.active_profile.clone())
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
            Message::LibraryEvent(game_event::Event::Added(added)) => {
                self.library_state = LibraryState::Loaded;

                if let Some(game) = added.game {
                    upsert_game(&mut self.games, game);
                }
            }
            Message::LibraryEvent(game_event::Event::Updated(updated)) => {
                self.library_state = LibraryState::Loaded;

                if let Some(game) = updated.game {
                    upsert_game(&mut self.games, game);
                }
            }
            Message::LibraryEvent(game_event::Event::Removed(removed)) => {
                self.games.retain(|game| {
                    !(game.id == removed.game_id && game.storefront == removed.storefront)
                });
            }
            Message::LibraryError(err) => {
                self.library_state = LibraryState::Failed(err.clone());
                eprintln!("failed to watch library: {err}");
            }
            Message::OpenMenu
            | Message::TogglePower
            | Message::ProgramLaunched(Ok(_))
            | Message::Noop => {}
        }

        Task::none()
    }

    fn refresh_bottle_states(&mut self) {
        self.bottle_states = self
            .bottle_list
            .iter()
            .filter_map(|bottle| bottle.state().ok())
            .collect();
    }

    fn selected_bottle_handle(&self) -> Option<Bottle> {
        let SplitViewState::Bottle(id) = self.split_view_state else {
            return None;
        };

        self.bottle_list
            .iter()
            .find(|bottle| bottle.state().is_ok_and(|state| state.id() == id))
            .cloned()
    }

    fn selected_bottle_state(&self) -> Option<&Arc<BottleState>> {
        let SplitViewState::Bottle(id) = self.split_view_state else {
            return None;
        };

        self.bottle_states.iter().find(|state| state.id() == id)
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

        if let Some(bottles) = &self.bottles {
            let handle = BottleManagerHandle(bottles.bottles().clone());
            subscriptions.push(Subscription::run_with(handle, bottle_events));
        }

        if let Some(handle) = self.profile_manager.clone() {
            subscriptions.push(Subscription::run_with(handle, profile_events));
        }

        if let Some(profile) = &self.active_profile {
            let handle = LibraryHandle(profile.id.clone());
            subscriptions.push(Subscription::run_with(handle, library_events));
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
            .end(self.profile_switcher());
        let content: Element<'_, Message> = match self.primary_tab {
            PrimaryTab::Bottles => {
                let columns =
                    usize::from(mode == PaneMode::Single && width >= CONTENT_GRID_BREAKPOINT) + 1;
                let rows = self.bottle_states.iter().fold(
                    RowGroup::new().columns(columns),
                    |rows, state| {
                        rows.add(
                            ActionRow::new(
                                state.name(),
                                ActionRowState::Ready(Message::BottleSelected(state.id())),
                            )
                            .description(state.runner().name())
                            .icon(Icon::Bottles),
                        )
                    },
                );

                container(rows).max_width(1150).into()
            }
            PrimaryTab::Library => self.library_view(width, mode),
        };

        column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn profile_settings_page(&self, _width: f32, mode: PaneMode) -> Element<'_, Message> {
        let header = HeaderBar::new(Message::Window)
            .show_window_controls(cfg!(target_os = "macos") || mode == PaneMode::Single)
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
                Message::ToggleNewProfile,
            ));

        let content: Element<'_, Message> = if let Some(active) = &self.active_profile {
            let mut accounts = RowGroup::new().title("Linked accounts");

            for account in &active.accounts {
                let storefront = Storefront::try_from(account.storefront).unwrap_or_default();
                accounts = accounts.add(account_row(
                    storefront_icon(storefront),
                    format!(
                        "{} on {}",
                        account.account_display_name,
                        storefront_label(storefront)
                    ),
                    auth_state_label(account.auth_state),
                    Message::UnlinkAccount(account.storefront),
                ));
            }

            let link_trigger = PickerRow::new("Link a storefront account")
                .description("Choose the account to connect")
                .on_press(Message::ToggleLink);
            let mut link_popover = Popover::new(
                link_trigger,
                self.account_link_popover == AccountLinkPopover::Storefront,
            )
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

                let mut item = PopoverItem::new(storefront_label(*storefront))
                    .icon(storefront_icon(*storefront));

                item = if LOGIN_STOREFRONTS.contains(storefront) {
                    item.action("Link", Message::BeginLogin(*storefront))
                } else {
                    item.subtitle("Coming soon")
                };

                link_popover = link_popover.add(item);
            }

            let steam_row: Element<'_, Message> = if let Some(link) = &active.steam_link {
                account_row(
                    Icon::Computer,
                    &link.account_name,
                    "Linked Steam account",
                    Message::UnlinkSteam,
                )
                .into()
            } else {
                let steam_trigger = PickerRow::new("Link Steam account")
                    .description("Detected from your local Steam installation")
                    .on_press(Message::ToggleSteam);
                let mut steam_popover = Popover::new(
                    steam_trigger,
                    self.account_link_popover == AccountLinkPopover::Steam,
                )
                .on_dismiss(Message::Dismiss);

                for user in &self.steam_candidates {
                    let taken_by = self.profiles.iter().find(|profile| {
                        profile.id != active.id
                            && profile
                                .steam_link
                                .as_ref()
                                .is_some_and(|link| link.steam_id64 == user.steam_id64)
                    });

                    let mut item = PopoverItem::new(&user.account_name).icon(Icon::Computer);

                    item = if let Some(owner) = taken_by {
                        item.disabled_action("Taken").tooltip(
                            column![
                                text("Already linked").detail(),
                                text(&owner.name).detail().muted(),
                            ]
                            .spacing(2),
                        )
                    } else {
                        item.action(
                            "Link",
                            Message::LinkSteam(user.steam_id64.clone(), user.account_name.clone()),
                        )
                    };

                    steam_popover = steam_popover.add(item);
                }

                steam_popover.into()
            };

            column![
                RowGroup::new()
                    .title("Profile")
                    .add(
                        TextRow::new("Profile name", &self.name_draft)
                            .icon(Icon::Person)
                            .on_input(Message::NameChanged)
                            .on_submit(Message::RenameSubmit),
                    )
                    .add(action_button_row(
                        Icon::Cross,
                        "Delete profile",
                        "Removes this profile and its linked accounts from this device",
                        "Delete",
                        Message::DeleteProfile(active.id.clone()),
                    )),
                accounts,
                container(link_popover).width(Fill),
                container(steam_row).width(Fill),
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

        match &self.profile_modal {
            ProfileModal::Login(login) => modal(page, login_dialog(login), Message::CancelLogin),
            ProfileModal::NewProfile(name) => {
                modal(page, new_profile_dialog(name), Message::CancelNewProfile)
            }
            ProfileModal::None => page,
        }
    }

    fn new_bottle_page(&self, _width: f32, mode: PaneMode) -> Element<'_, Message> {
        let header = HeaderBar::new(Message::Window)
            .show_window_controls(cfg!(target_os = "macos") || mode == PaneMode::Single)
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
                Message::CreateBottle,
            ));
        let content = column![
            TextRow::new("Bottle Name", &self.bottle_name)
                .icon(Icon::Person)
                .on_input(Message::BottleNameChanged),
            SelectorRow::new("Runner", &self.runners, self.selected_runner.as_ref())
                .icon(Icon::Run)
                .on_selected(Message::RunnerSelected),
            SelectorRow::new("Purpose", &PURPOSES, Some(&self.purpose))
                .on_selected(Message::PurposeSelected),
            SelectorRow::new("Architecture", &ARCHITECTURES, Some(&self.architecture),)
                .icon(Icon::Chip)
                .on_selected(Message::ArchitectureSelected),
            PickerRow::new("Use Recipe")
                .description("Choose the location")
                .on_press(Message::Noop),
        ]
        .spacing(12);

        let mut page = column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill);

        if !self.creation_log.is_empty() {
            let state = if self.creation_failed {
                StatusState::Failed
            } else {
                StatusState::Starting
            };

            page = page.push(
                StatusBar::new(
                    self.architecture,
                    self.selected_runner
                        .as_ref()
                        .map(|runner| runner.label.as_str())
                        .unwrap_or_default(),
                    state,
                )
                .log(&self.creation_log)
                .expanded(self.creation_log_expanded)
                .on_toggle(Message::ToggleCreationLog),
            );
        }

        page.into()
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
            DetailTab::Programs => self.program_grid(width),
            DetailTab::Settings => self.settings.view(&settings_ctx).map(Message::Settings),
            DetailTab::Snapshots => self.snapshots.view(width).map(Message::Snapshots),
        };

        column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn library_view(&self, width: f32, mode: PaneMode) -> Element<'_, Message> {
        match &self.library_state {
            LibraryState::Idle => {
                return container(InfoCard::new(
                    Kind::Hint,
                    "No active profile",
                    "Sign in to a profile to see its library.",
                ))
                .max_width(1150)
                .into();
            }
            LibraryState::Loading => {
                return container(InfoCard::new(
                    Kind::Hint,
                    "Loading library",
                    "Fetching games linked to this profile's storefronts.",
                ))
                .max_width(1150)
                .into();
            }
            LibraryState::Failed(err) => {
                return container(InfoCard::new(
                    Kind::Error,
                    "Couldn't load library",
                    err.as_str(),
                ))
                .max_width(1150)
                .into();
            }
            LibraryState::Loaded => {}
        }

        if self.games.is_empty() {
            return container(InfoCard::new(
                Kind::Hint,
                "Nothing here yet",
                "Games linked to this profile's storefronts will show up here.",
            ))
            .max_width(1150)
            .into();
        }

        let columns = usize::from(mode == PaneMode::Single && width >= CONTENT_GRID_BREAKPOINT) + 1;
        let rows = self
            .games
            .iter()
            .fold(RowGroup::new().columns(columns), |rows, game| {
                let storefront = Storefront::try_from(game.storefront).unwrap_or_default();

                rows.add(
                    ActionRow::new(&game.title, ActionRowState::Ready(Message::Noop))
                        .description(storefront_label(storefront))
                        .icon(storefront_icon(storefront)),
                )
            });

        container(rows).max_width(1150).into()
    }

    fn program_grid(&self, width: f32) -> Element<'_, Message> {
        let programs = if let SplitViewState::Bottle(bottle_id) = self.split_view_state {
            self.bottle_states
                .iter()
                .find(|state| state.id() == bottle_id)
                .map(|state| state.programs())
                .unwrap_or_default()
        } else {
            &[]
        };

        if width >= CONTENT_GRID_BREAKPOINT {
            Column::with_children(
                programs
                    .chunks(2)
                    .map(|chunk| row(chunk.iter().map(program_card)).spacing(12).into()),
            )
            .spacing(12)
            .into()
        } else {
            Column::with_children(programs.iter().map(program_card))
                .spacing(12)
                .into()
        }
    }

    fn profile_switcher(&self) -> Element<'_, Message> {
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

fn timestamp() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    format!(
        "{:02}:{:02}:{:02}",
        (seconds / 3600) % 24,
        (seconds / 60) % 60,
        seconds % 60
    )
}

fn progress_log_line(progress: &Progress) -> String {
    match progress.transfer.as_ref().and_then(|_| progress.fraction()) {
        Some(fraction) => format!(
            "{} {} ({:.0}%)",
            timestamp(),
            progress.stage,
            fraction * 100.0
        ),
        None => format!("{} {}", timestamp(), progress.stage),
    }
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

fn upsert_game(games: &mut Vec<Game>, game: Game) {
    if let Some(existing) = games
        .iter_mut()
        .find(|existing| existing.id == game.id && existing.storefront == game.storefront)
    {
        *existing = game;
    } else {
        games.push(game);
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

fn program_card(program: &bottles_core::Program) -> Element<'_, Message> {
    ArtworkCard::new(&program.name, &program.executable)
        .secondary(CardAction::new("Settings", Icon::Gear).on_press(Message::Noop))
        .primary(CardAction::new("Play", Icon::Play).on_press(Message::LaunchProgram(program.id)))
        .banner(sample_image(program.id))
        .into()
}

fn login_dialog(login: &LoginChallenge) -> Element<'_, Message> {
    let submit_label = if login.submitting {
        "Submitting…"
    } else {
        "Submit"
    };

    container(
        column![
            Title::new("Sign in").subtitle(storefront_label(login.storefront)),
            RowGroup::new()
                .add(
                    label_row(
                        storefront_icon(login.storefront),
                        "Sign-in link (click to copy)",
                        &login.url
                    )
                    .on_press(Message::CopyLoginUrl),
                )
                .add(action_button_row(
                    Icon::Arrow,
                    "Open in your browser",
                    "Sign in there, then paste the code you're given below.",
                    "Open",
                    Message::OpenLoginUrl,
                ))
                .add(
                    TextRow::new("Authorization code", &login.code_draft)
                        .icon(Icon::Checkmark)
                        .on_input(Message::LoginCodeChanged)
                        .on_submit(Message::SubmitLoginCode),
                ),
            row![
                Button::new(submit_label)
                    .kind(ButtonKind::Primary)
                    .on_press_maybe((!login.submitting).then_some(Message::SubmitLoginCode)),
                Button::new("Cancel")
                    .kind(ButtonKind::Transparent)
                    .on_press(Message::CancelLogin),
            ]
            .spacing(12),
        ]
        .spacing(18),
    )
    .width(560)
    .padding(24)
    .style(theme::panel)
    .into()
}

fn new_profile_dialog(name: &str) -> Element<'_, Message> {
    container(
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

fn account_row<'a>(
    icon: Icon,
    title: impl text::IntoFragment<'a>,
    description: &'a str,
    on_unlink: Message,
) -> ListRow<'a, Message> {
    action_button_row(icon, title, description, "Unlink", on_unlink)
}

fn label_row<'a>(
    icon: Icon,
    title: &'a str,
    description: impl text::IntoFragment<'a>,
) -> ListRow<'a, Message> {
    let labels = column![text(title).label(), text(description).detail().muted()].spacing(6);

    ListRow::new(labels).leading(
        svg(icon.handle())
            .width(24)
            .height(24)
            .content_fit(iced::ContentFit::Contain),
    )
}

fn action_button_row<'a>(
    icon: Icon,
    title: impl text::IntoFragment<'a>,
    description: &'a str,
    button_label: &'a str,
    on_press: Message,
) -> ListRow<'a, Message> {
    let labels = column![text(title).label(), text(description).detail().muted()].spacing(6);

    ListRow::new(labels)
        .leading(
            svg(icon.handle())
                .width(24)
                .height(24)
                .content_fit(iced::ContentFit::Contain),
        )
        .trailing(
            Button::new(button_label)
                .kind(ButtonKind::Surface)
                .on_press(on_press),
        )
}

async fn begin_login(
    profile_id: String,
    storefront: Storefront,
) -> Result<(String, String), String> {
    let mut registry = RegistryClient::connect(REGISTRY_ENDPOINT)
        .await
        .map_err(|err| format!("plugin registry unavailable: {err}"))?;
    let resolved = registry
        .resolve_plugin(ResolvePluginRequest {
            storefront: storefront as i32,
        })
        .await
        .map_err(|err| err.to_string())?
        .into_inner();
    let endpoint = resolved
        .endpoint
        .ok_or_else(|| format!("no {} plugin is running", storefront_label(storefront)))?;
    let mut store = StoreClient::connect(endpoint)
        .await
        .map_err(|err| err.to_string())?;
    let challenge = store
        .begin_login(BeginLoginRequest {
            profile_id,
            storefront: storefront as i32,
        })
        .await
        .map_err(|err| err.to_string())?
        .into_inner();

    if let Some(error) = challenge.error {
        return Err(error);
    }

    let url = match challenge.kind {
        Some(login_challenge::Kind::BrowserRedirect(challenge)) => challenge.url,
        Some(login_challenge::Kind::OauthRedirect(challenge)) => challenge.auth_url,
        _ => return Err("this storefront's login flow isn't supported yet".into()),
    };

    Ok((challenge.challenge_id, url))
}

async fn complete_login(
    profile_id: String,
    challenge_id: String,
    storefront: Storefront,
    user_input: String,
) -> Result<UserProfile, String> {
    let mut client = ProfileClient::connect(SERVER_ENDPOINT)
        .await
        .map_err(|err| format!("next-server unavailable: {err}"))?;

    client
        .link_account(LinkAccountRequest {
            profile_id,
            challenge_id,
            storefront: storefront as i32,
            user_input,
        })
        .await
        .map(|response| response.into_inner())
        .map_err(|err| err.to_string())
}

fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

fn detect_steam_users() -> Vec<bottles_core::steam::SteamUser> {
    bottles_core::steam::loginusers_vdf_path()
        .and_then(|path| bottles_core::steam::parse_loginusers(&path).ok())
        .unwrap_or_default()
}

fn auth_state_label(state: i32) -> &'static str {
    match AuthState::try_from(state).unwrap_or_default() {
        AuthState::Active => "Connected",
        AuthState::Stale => "Needs re-authentication",
        AuthState::Inactive => "Signed out",
        AuthState::Unspecified => "Unknown",
    }
}

fn sample_image(id: Uuid) -> image::Handle {
    let seed = id.as_bytes()[0] % 16;
    let first = [45 + seed * 5, 50 + seed * 3, 65 + seed * 4];
    let second = [first[2], first[0] + 20, first[1] + 10];

    image::Handle::from_rgba(
        2,
        2,
        vec![
            first[0], first[1], first[2], 255, second[0], second[1], second[2], 255, second[0],
            second[1], second[2], 255, first[0], first[1], first[2], 255,
        ],
    )
}
