use std::sync::Arc;

use bottles_core::{Bottle, BottleManager, BottleState, Bottles, Config as CoreConfig, SnapshotSummary};
use iced::{
    Element, Fill, Padding, Subscription, Task, Theme,
    futures::StreamExt as _,
    keyboard::{self, key},
    widget::{Column, column, container, image, row, scrollable},
};
use uuid::Uuid;
use next_ui::{
    components::{
        action_row::{ActionRow, State},
        artwork_card::{ArtworkCard, CardAction},
        button::{Button, ButtonKind},
        header_bar::HeaderBar,
        picker_row::PickerRow,
        row_group::RowGroup,
        selector_row::SelectorRow,
        split_view::{PaneMode, PaneSide, SplitView},
        tabs::{Tab, Tabs},
        text_row::TextRow,
        title::Title,
        window_frame,
    },
    icons::Icon,
    theme,
};

const CONTENT_GRID_BREAKPOINT: f32 = 720.0;

const RUNNERS: [&str; 3] = ["soda-7.0-9", "soda-9.0-1", "sys-wine"];
const PURPOSES: [&str; 4] = ["Gaming", "Software", "Gaming (ULWGL)", "Custom"];
const ARCHITECTURES: [&str; 2] = ["Win64", "Win32"];

const LIBRARY: [(&str, &str, Icon); 4] = [
    ("Battle.net", "Installed program", Icon::Computer),
    (
        "Assassin’s Creed Valhalla",
        "Installed program",
        Icon::Controller,
    ),
    ("Cyberpunk 2077", "Installed program", Icon::Controller),
    ("Steam", "Runtime library", Icon::Computer),
];

const SETTINGS: [[(&str, &str, Icon); 4]; 4] = [
    [
        ("Runner", "soda-9.0-1", Icon::Run),
        ("Windows version", "Windows 11", Icon::Computer),
        ("Working directory", "Games", Icon::Folder),
        ("Environment variables", "Game mode enabled", Icon::Gear),
    ],
    [
        ("Runner", "caffe-9.7", Icon::Run),
        ("Windows version", "Windows 10", Icon::Computer),
        ("Working directory", "Development", Icon::Folder),
        (
            "Environment variables",
            "Compiler paths configured",
            Icon::Gear,
        ),
    ],
    [
        ("Runner", "soda-experimental", Icon::Run),
        ("Windows version", "Windows 11", Icon::Computer),
        ("Working directory", "Engine projects", Icon::Folder),
        ("Environment variables", "GPU tools enabled", Icon::Gear),
    ],
    [
        ("Runner", "sys-wine", Icon::Run),
        ("Windows version", "Windows 7", Icon::Computer),
        ("Working directory", "Temporary lab", Icon::Folder),
        (
            "Environment variables",
            "Debug overrides active",
            Icon::Gear,
        ),
    ],
];

fn main() -> iced::Result {
    iced::application(Example::new, Example::update, Example::view)
        .title("Bottles Next split view")
        .theme(Example::theme)
        .subscription(Example::subscription)
        .style(|_, theme| theme::application(theme))
        .window_size((1600.0, 1000.0))
        .centered()
        .decorations(false)
        .transparent(true)
        .run()
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
    runner: &'static str,
    purpose: &'static str,
    architecture: &'static str,
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
    BottleNameChanged(String),
    RunnerSelected(&'static str),
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
    Noop,
}

impl Example {
    fn new() -> (Self, Task<Message>) {
        let state = Self {
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
            runner: RUNNERS[0],
            purpose: PURPOSES[0],
            architecture: ARCHITECTURES[0],
        };
        let boot = Task::perform(
            async {
                Bottles::open(CoreConfig::default())
                    .await
                    .map(Arc::new)
                    .map_err(|err| err.to_string())
            },
            Message::BottlesLoaded,
        );

        (state, boot)
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
                }
            }
            Message::DetailTabSelected(tab) => self.detail_tab = tab,
            Message::BottleSelected(id) => {
                self.primary_tab = PrimaryTab::Bottles;
                self.selected_bottle = Some(id);
                self.creating_bottle = false;
                self.snapshots.clear();
                self.snapshot_rows.clear();

                if let Some(bottle) = self
                    .bottle_list
                    .iter()
                    .find(|bottle| bottle.state().is_ok_and(|state| state.id() == id))
                    .cloned()
                {
                    return Task::perform(
                        async move { bottle.snapshots().await.map_err(|err| err.to_string()) },
                        Message::SnapshotsLoaded,
                    );
                }
            }
            Message::Back => self.selected_bottle = None,
            Message::AddBottle => self.creating_bottle = true,
            Message::CancelBottle | Message::CreateBottle => self.creating_bottle = false,
            Message::BottleNameChanged(name) => self.bottle_name = name,
            Message::RunnerSelected(runner) => self.runner = runner,
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
                    self.bottle_list = bottles.bottles().list();
                    self.bottles = Some(bottles);
                    self.refresh_bottle_states();
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
                if let Some(bottle) = self.selected_bottle.and_then(|selected| {
                    self.bottle_list
                        .iter()
                        .find(|bottle| bottle.state().is_ok_and(|state| state.id() == selected))
                        .cloned()
                }) {
                    return Task::perform(
                        async move { bottle.run(id).await.map_err(|err| err.to_string()) },
                        Message::ProgramLaunched,
                    );
                }
            }
            Message::ProgramLaunched(Err(err)) => eprintln!("failed to launch program: {err}"),
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

        let Some(bottles) = &self.bottles else {
            return keys;
        };
        let handle = BottleManagerHandle(bottles.bottles().clone());

        Subscription::batch([keys, Subscription::run_with(handle, bottle_events)])
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
            .middle(tabs);
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
            PrimaryTab::Library => action_grid(&LIBRARY, width),
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
            SelectorRow::new("Runner", &RUNNERS, Some(&self.runner))
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
        let bottle = self
            .selected_bottle
            .and_then(|id| self.bottle_states.iter().position(|state| state.id() == id))
            .unwrap_or(0)
            .min(SETTINGS.len() - 1);
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
            DetailTab::Settings => action_grid(&SETTINGS[bottle], width),
            DetailTab::Snapshots => self.snapshot_grid(width),
        };

        column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into()
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

fn action_grid(
    entries: &'static [(&'static str, &'static str, Icon)],
    width: f32,
) -> Element<'static, Message> {
    let columns = usize::from(width >= CONTENT_GRID_BREAKPOINT) + 1;
    let rows = entries.iter().fold(
        RowGroup::new().columns(columns),
        |rows, (title, description, icon)| {
            rows.add(
                ActionRow::new(title, State::Ready(Message::Noop))
                    .description(description)
                    .icon(*icon),
            )
        },
    );

    container(rows).max_width(1150).into()
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
