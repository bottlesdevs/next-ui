use std::sync::Arc;

use bottles_core::{
    Bottle, BottleManager, BottleState, Bottles, Config as CoreConfig, MangoHudConfig, Slot,
    SnapshotSummary, Storage,
    profile::ProfileManager,
};
use iced::{
    Element, Fill, Padding, Subscription, Task, Theme,
    futures::StreamExt as _,
    keyboard::{self, key},
    widget::{Column, column, container, image, row, scrollable, svg, text},
};
use next_proto::bottles::{
    common::v1::{Game, Storefront},
    library::v1::{ListGamesRequest, library_client::LibraryClient},
    profiles::v1::{UserProfile, profile_event},
};
use uuid::Uuid;
use next_ui::{
    components::{
        action_row::{ActionRow, State},
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
        switcher_row::SwitcherRow,
        tabs::{Tab, Tabs},
        text::TextExt as _,
        text_row::TextRow,
        title::Title,
        window_frame,
    },
    icons::Icon,
    theme,
};

const CONTENT_GRID_BREAKPOINT: f32 = 720.0;

const PURPOSES: [&str; 4] = ["Gaming", "Software", "Gaming (ULWGL)", "Custom"];
const ARCHITECTURES: [&str; 2] = ["Win64", "Win32"];
const SERVER_ENDPOINT: &str = "http://127.0.0.1:50052";

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Bottles Next")
        .theme(App::theme)
        .subscription(App::subscription)
        .style(|_, theme| theme::application(theme))
        .window_size((1600.0, 1000.0))
        .centered()
        .decorations(false)
        .transparent(true)
        .run()
}

enum App {
    Onboarding(Box<next_ui::onboarding::State>),
    Main(Box<Example>),
}

#[derive(Clone)]
enum AppMessage {
    Onboarding(next_ui::onboarding::Message),
    Main(Message),
}

impl App {
    fn new() -> (Self, Task<AppMessage>) {
        let (state, task) = next_ui::onboarding::State::new();

        (Self::Onboarding(Box::new(state)), task.map(AppMessage::Onboarding))
    }

    fn theme(&self) -> Theme {
        match self {
            Self::Onboarding(state) => state.theme(),
            Self::Main(example) => example.theme(),
        }
    }

    fn subscription(&self) -> Subscription<AppMessage> {
        match self {
            Self::Onboarding(_) => Subscription::none(),
            Self::Main(example) => example.subscription().map(AppMessage::Main),
        }
    }

    fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match message {
            AppMessage::Onboarding(message) => {
                let Self::Onboarding(state) = self else {
                    return Task::none();
                };

                if let next_ui::onboarding::Message::Finished = message {
                    let (example, task) = match state.take_bottles() {
                        Some(bottles) => Example::new_with_bottles(bottles),
                        None => Example::new(),
                    };
                    *self = Self::Main(Box::new(example));
                    return task.map(AppMessage::Main);
                }

                state.update(message).map(AppMessage::Onboarding)
            }
            AppMessage::Main(message) => {
                let Self::Main(example) = self else {
                    return Task::none();
                };

                example.update(message).map(AppMessage::Main)
            }
        }
    }

    fn view(&self) -> Element<'_, AppMessage> {
        match self {
            Self::Onboarding(state) => state.view().map(AppMessage::Onboarding),
            Self::Main(example) => example.view().map(AppMessage::Main),
        }
    }
}

#[derive(Clone, PartialEq)]
struct RunnerOption {
    id: Uuid,
    label: String,
}

impl std::fmt::Display for RunnerOption {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrimaryTab {
    Bottles,
    Library,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetailTab {
    Programs,
    Settings,
    Snapshots,
}

struct Example {
    primary_tab: PrimaryTab,
    detail_tab: DetailTab,
    bottles: Option<Bottles>,
    bottle_list: Vec<Bottle>,
    bottle_states: Vec<Arc<BottleState>>,
    selected_bottle: Option<Uuid>,
    snapshots: Vec<SnapshotSummary>,
    snapshot_rows: Vec<(String, String)>,
    creating_bottle: bool,
    bottle_name: String,
    runners: Vec<RunnerOption>,
    selected_runner: Option<RunnerOption>,
    purpose: &'static str,
    architecture: &'static str,
    profile_manager: Option<ProfileManagerHandle>,
    profiles: Vec<UserProfile>,
    active_profile: Option<UserProfile>,
    profile_switcher_open: bool,
    games: Vec<Game>,
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
enum Message {
    PrimaryTabSelected(PrimaryTab),
    DetailTabSelected(DetailTab),
    BottleSelected(Uuid),
    Back,
    AddBottle,
    CancelBottle,
    CreateBottle,
    BottleCreated(Result<Bottle, String>),
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
    SnapshotsLoaded(Result<Vec<SnapshotSummary>, String>),
    LaunchProgram(Uuid),
    ProgramLaunched(Result<u32, String>),
    ToggleGamescope(bool),
    ToggleMangoHud(bool),
    WrapperUpdated(Result<(), String>),
    ProfileManagerLoaded(Result<Arc<ProfileManager>, String>),
    ProfilesLoaded(Vec<UserProfile>),
    ProfileEvent(profile_event::Event),
    ToggleProfileSwitcher,
    ActivateProfile(String),
    CreateProfile,
    ProfileUpdated(Result<UserProfile, String>),
    LibraryLoaded(Result<Vec<Game>, String>),
    Noop,
}

impl Example {
    fn empty() -> Self {
        Self {
            primary_tab: PrimaryTab::Bottles,
            detail_tab: DetailTab::Programs,
            bottles: None,
            bottle_list: Vec::new(),
            bottle_states: Vec::new(),
            selected_bottle: None,
            snapshots: Vec::new(),
            snapshot_rows: Vec::new(),
            creating_bottle: false,
            bottle_name: "Gaming paradise".into(),
            runners: Vec::new(),
            selected_runner: None,
            purpose: PURPOSES[0],
            architecture: ARCHITECTURES[0],
            profile_manager: None,
            profiles: Vec::new(),
            active_profile: None,
            profile_switcher_open: false,
            games: Vec::new(),
        }
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

    fn new() -> (Self, Task<Message>) {
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

    fn new_with_bottles(bottles: Bottles) -> (Self, Task<Message>) {
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

    fn theme(&self) -> Theme {
        theme::theme()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PrimaryTabSelected(tab) => {
                self.primary_tab = tab;

                if tab == PrimaryTab::Library {
                    self.selected_bottle = None;
                    self.creating_bottle = false;

                    if let Some(profile_id) =
                        self.active_profile.as_ref().map(|profile| profile.id.clone())
                    {
                        return Task::perform(list_games(profile_id), Message::LibraryLoaded);
                    }
                }
            }
            Message::DetailTabSelected(tab) => self.detail_tab = tab,
            Message::BottleSelected(id) => {
                self.primary_tab = PrimaryTab::Bottles;
                self.selected_bottle = Some(id);
                self.creating_bottle = false;
                self.snapshots.clear();
                self.snapshot_rows.clear();

                if let Some(bottle) = self.selected_bottle_handle() {
                    return Task::perform(
                        async move { bottle.snapshots().await.map_err(|err| err.to_string()) },
                        Message::SnapshotsLoaded,
                    );
                }
            }
            Message::Back => self.selected_bottle = None,
            Message::AddBottle => self.creating_bottle = true,
            Message::CancelBottle => self.creating_bottle = false,
            Message::CreateBottle => {
                if let (Some(bottles), Some(runner)) =
                    (&self.bottles, self.selected_runner.clone())
                {
                    let name = self.bottle_name.clone();
                    let manager = bottles.bottles().clone();

                    return Task::perform(
                        async move {
                            manager
                                .create(name, Storage::Standard, runner.id)
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::BottleCreated,
                    );
                }
            }
            Message::BottleCreated(Ok(_)) => self.creating_bottle = false,
            Message::BottleCreated(Err(err)) => eprintln!("failed to create bottle: {err}"),
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
            Message::SnapshotsLoaded(Ok(snapshots)) => {
                self.snapshot_rows = snapshots
                    .iter()
                    .map(|snapshot| {
                        let title = if snapshot.message.is_empty() {
                            snapshot.state_id.chars().take(12).collect()
                        } else {
                            snapshot.message.clone()
                        };
                        let description = snapshot
                            .created_at
                            .as_ref()
                            .map(|timestamp| relative_time(timestamp.seconds))
                            .unwrap_or_default();

                        (title, description)
                    })
                    .collect();
                self.snapshots = snapshots;
            }
            Message::SnapshotsLoaded(Err(err)) => eprintln!("failed to load snapshots: {err}"),
            Message::LaunchProgram(id) => {
                if let Some(bottle) = self.selected_bottle_handle() {
                    return Task::perform(
                        async move { bottle.run(id).await.map_err(|err| err.to_string()) },
                        Message::ProgramLaunched,
                    );
                }
            }
            Message::ProgramLaunched(Err(err)) => eprintln!("failed to launch program: {err}"),
            Message::ToggleGamescope(enabled) => {
                if let (Some(bottle), Some(state)) =
                    (self.selected_bottle_handle(), self.selected_bottle_state())
                {
                    let mut config = state.wrappers().gamescope.clone();
                    config.enabled = enabled;

                    return Task::perform(
                        async move {
                            let mut edit = bottle.edit();
                            edit.set_gamescope(config);
                            edit.commit().await.map_err(|err| err.to_string())
                        },
                        Message::WrapperUpdated,
                    );
                }
            }
            Message::ToggleMangoHud(enabled) => {
                if let Some(bottle) = self.selected_bottle_handle() {
                    let config = MangoHudConfig { enabled };

                    return Task::perform(
                        async move {
                            let mut edit = bottle.edit();
                            edit.set_mangohud(config);
                            edit.commit().await.map_err(|err| err.to_string())
                        },
                        Message::WrapperUpdated,
                    );
                }
            }
            Message::WrapperUpdated(Ok(())) => self.refresh_bottle_states(),
            Message::WrapperUpdated(Err(err)) => eprintln!("failed to update settings: {err}"),
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
                upsert_profile(&mut self.profiles, profile.clone());
                self.active_profile = Some(profile);
            }
            Message::ProfileEvent(profile_event::Event::Updated(profile)) => {
                upsert_profile(&mut self.profiles, profile.clone());

                if self.active_profile.as_ref().is_some_and(|active| active.id == profile.id) {
                    self.active_profile = Some(profile);
                }
            }
            Message::ProfileEvent(profile_event::Event::DeletedProfileId(id)) => {
                self.profiles.retain(|profile| profile.id != id);

                if self.active_profile.as_ref().is_some_and(|active| active.id == id) {
                    self.active_profile = self.profiles.first().cloned();
                }
            }
            Message::ToggleProfileSwitcher => self.profile_switcher_open = !self.profile_switcher_open,
            Message::ActivateProfile(id) => {
                self.profile_switcher_open = false;

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
            Message::CreateProfile => {
                self.profile_switcher_open = false;

                if let Some(handle) = self.profile_manager.clone() {
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
            Message::ProfileUpdated(Ok(profile)) => {
                upsert_profile(&mut self.profiles, profile.clone());
                let profile_id = profile.id.clone();
                self.active_profile = Some(profile);

                if self.primary_tab == PrimaryTab::Library {
                    return Task::perform(list_games(profile_id), Message::LibraryLoaded);
                }
            }
            Message::ProfileUpdated(Err(err)) => eprintln!("failed to update profile: {err}"),
            Message::LibraryLoaded(Ok(games)) => self.games = games,
            Message::LibraryLoaded(Err(err)) => eprintln!("failed to load library: {err}"),
            Message::OpenMenu | Message::TogglePower | Message::ProgramLaunched(Ok(_)) | Message::Noop => {}
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
        let id = self.selected_bottle?;

        self.bottle_list
            .iter()
            .find(|bottle| bottle.state().is_ok_and(|state| state.id() == id))
            .cloned()
    }

    fn selected_bottle_state(&self) -> Option<&Arc<BottleState>> {
        let id = self.selected_bottle?;

        self.bottle_states.iter().find(|state| state.id() == id)
    }

    fn subscription(&self) -> Subscription<Message> {
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

        Subscription::batch(subscriptions)
    }

    fn view(&self) -> Element<'_, Message> {
        let split = SplitView::new(
            |_, _| {
                SplitView::new(
                    |width, mode| self.primary_page(width, mode),
                    |width, mode| self.detail_page(width, mode),
                )
                .show_detail(self.selected_bottle.is_some())
                .into()
            },
            |width, mode| self.new_bottle_page(width, mode),
        )
        .side(PaneSide::Start)
        .show_detail(self.creating_bottle)
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
                !self.creating_bottle
            } else {
                self.selected_bottle.is_none()
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
                                State::Ready(Message::BottleSelected(state.id())),
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
                self.selected_bottle.is_some() && mode == PaneMode::Single && !self.creating_bottle
            } else {
                self.selected_bottle.is_some()
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
        let content = match self.detail_tab {
            DetailTab::Programs => self.program_grid(width),
            DetailTab::Settings => self.settings_view(),
            DetailTab::Snapshots => self.snapshot_grid(width),
        };

        column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn library_view(&self, width: f32, mode: PaneMode) -> Element<'_, Message> {
        if self.active_profile.is_none() {
            return container(InfoCard::new(
                Kind::Hint,
                "No active profile",
                "Sign in to a profile to see its library.",
            ))
            .max_width(1150)
            .into();
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

        let columns =
            usize::from(mode == PaneMode::Single && width >= CONTENT_GRID_BREAKPOINT) + 1;
        let rows = self.games.iter().fold(RowGroup::new().columns(columns), |rows, game| {
            let storefront = Storefront::try_from(game.storefront).unwrap_or_default();

            rows.add(
                ActionRow::new(&game.title, State::Ready(Message::Noop))
                    .description(storefront_label(storefront))
                    .icon(storefront_icon(storefront)),
            )
        });

        container(rows).max_width(1150).into()
    }

    fn program_grid(&self, width: f32) -> Element<'_, Message> {
        let programs = self
            .selected_bottle
            .and_then(|id| self.bottle_states.iter().find(|state| state.id() == id))
            .map(|state| state.programs())
            .unwrap_or_default();

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
            .kind(ButtonKind::Transparent)
            .on_press(Message::ToggleProfileSwitcher);
        let mut switcher = Popover::new(trigger, self.profile_switcher_open)
            .on_dismiss(Message::ToggleProfileSwitcher)
            .footer("New profile", Message::CreateProfile);

        for profile in &self.profiles {
            switcher = switcher.add(
                PopoverItem::new(&profile.name)
                    .icon(Icon::Person)
                    .on_select(Message::ActivateProfile(profile.id.clone())),
            );
        }

        switcher.into()
    }

    fn settings_view(&self) -> Element<'_, Message> {
        let Some(state) = self.selected_bottle_state() else {
            return column![].into();
        };
        let wrappers = state.wrappers();
        let environment_count = state.environment().iter().count();
        let environment_label = if environment_count == 0 {
            "None set".to_string()
        } else {
            format!("{environment_count} variables set")
        };

        let bottle = RowGroup::new()
            .title("Bottle")
            .add(
                ActionRow::new(state.runner().name(), State::Disabled)
                    .description(state.runner().version())
                    .icon(Icon::Run),
            )
            .add(environment_row(environment_label));

        let graphics = RowGroup::new()
            .title("Graphics")
            .add(
                SwitcherRow::new("DLSS", false)
                    .description("Deep Learning Super Sampling"),
            )
            .add(SwitcherRow::new("vkBasalt", false).description("Add post-processing effects"))
            .add(
                SwitcherRow::new("Discrete GPU", false)
                    .description("Force use your dedicated GPU"),
            )
            .add(
                SwitcherRow::new("Gamescope", wrappers.gamescope.enabled)
                    .description("Use the SteamOS compositor")
                    .on_toggle(Message::ToggleGamescope),
            )
            .add(
                SwitcherRow::new("MangoHud", wrappers.mangohud.enabled)
                    .description("Show a performance overlay")
                    .on_toggle(Message::ToggleMangoHud),
            )
            .add(PickerRow::new("Display Settings").description("Resolution and other options"));

        container(column![bottle, graphics].spacing(12))
            .max_width(1150)
            .into()
    }

    fn snapshot_grid(&self, width: f32) -> Element<'_, Message> {
        let columns = usize::from(width >= CONTENT_GRID_BREAKPOINT) + 1;
        let rows = self.snapshot_rows.iter().fold(
            RowGroup::new().columns(columns),
            |rows, (title, description)| {
                rows.add(
                    ActionRow::new(title, State::Ready(Message::Noop))
                        .description(description)
                        .icon(Icon::Timer),
                )
            },
        );

        container(rows).max_width(1150).into()
    }
}

fn environment_row(description: String) -> ListRow<'static, Message> {
    let labels = column![
        text("Environment variables").label(),
        text(description).detail().muted(),
    ]
    .spacing(6);

    ListRow::new(labels).leading(
        svg(Icon::Gear.handle())
            .width(24)
            .height(24)
            .content_fit(iced::ContentFit::Contain),
    )
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

fn relative_time(seconds: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(seconds, |duration| duration.as_secs() as i64);
    let diff = (now - seconds).max(0);

    match diff {
        0..=59 => "Just now".to_string(),
        60..=3599 => format!("{} minutes ago", diff / 60),
        3600..=86399 => format!("{} hours ago", diff / 3600),
        _ => format!("{} days ago", diff / 86400),
    }
}

async fn list_games(profile_id: String) -> Result<Vec<Game>, String> {
    let mut client = LibraryClient::connect(SERVER_ENDPOINT)
        .await
        .map_err(|err| format!("next-server unavailable: {err}"))?;

    client
        .list_games(ListGamesRequest {
            profile_id,
            storefronts: Vec::new(),
        })
        .await
        .map(|response| response.into_inner().games)
        .map_err(|err| err.to_string())
}

fn upsert_profile(profiles: &mut Vec<UserProfile>, profile: UserProfile) {
    if let Some(existing) = profiles.iter_mut().find(|existing| existing.id == profile.id) {
        *existing = profile;
    } else {
        profiles.push(profile);
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
