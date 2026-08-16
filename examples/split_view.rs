use std::sync::Arc;

use bottles_core::{Bottle, BottleManager, BottleState, Bottles, Config as CoreConfig};
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

const PROGRAMS: [[(&str, &str); 4]; 4] = [
    [
        ("Battle.net", "12 days ago"),
        ("Assassin’s Creed Valhalla", "Running…"),
        ("Cyberpunk 2077", "Last week"),
        ("Steam", "Win64"),
    ],
    [
        ("Visual Studio Build Tools", "Updated today"),
        (".NET SDK 8", "Installed"),
        ("SQL Server Express", "Stopped"),
        ("WinDbg Preview", "Last used yesterday"),
    ],
    [
        ("Unreal Engine 5.4", "Project Aurora"),
        ("Unity Hub", "3 editors installed"),
        ("Godot 4.3", "Last used yesterday"),
        ("Blender 4.2", "Rendering…"),
    ],
    [
        ("DXVK Testbed", "Experimental"),
        ("Vulkan Cube", "Running…"),
        ("Winecfg Sandbox", "Custom registry"),
        ("Registry Lab", "Snapshot protected"),
    ],
];

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

const SNAPSHOTS: [[(&str, &str, Icon); 3]; 4] = [
    [
        ("Current games", "Today at 22:45", Icon::Timer),
        ("Before runner update", "Yesterday at 18:12", Icon::Timer),
        ("Clean gaming setup", "30 July at 09:30", Icon::Timer),
    ],
    [
        ("SDK configured", "Today at 16:20", Icon::Timer),
        ("Before SQL install", "2 August at 11:05", Icon::Timer),
        ("Base development image", "28 July at 08:15", Icon::Timer),
    ],
    [
        ("Engine toolchain", "Yesterday at 21:10", Icon::Timer),
        ("Before Unreal update", "1 August at 14:30", Icon::Timer),
        ("Empty project setup", "25 July at 12:00", Icon::Timer),
    ],
    [
        ("Working experiment", "Today at 23:55", Icon::Timer),
        ("Before registry edits", "Today at 19:42", Icon::Timer),
        ("Disposable baseline", "3 August at 07:10", Icon::Timer),
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
            Message::OpenMenu | Message::TogglePower | Message::Noop => {}
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
            DetailTab::Programs => self.program_grid(width, bottle),
            DetailTab::Settings => action_grid(&SETTINGS[bottle], width),
            DetailTab::Snapshots => action_grid(&SNAPSHOTS[bottle], width),
        };

        column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn program_grid(&self, width: f32, bottle: usize) -> Element<'_, Message> {
        if width >= CONTENT_GRID_BREAKPOINT {
            column![
                row![program_card(bottle, 0), program_card(bottle, 1)].spacing(12),
                row![program_card(bottle, 2), program_card(bottle, 3)].spacing(12),
            ]
            .spacing(12)
            .into()
        } else {
            Column::with_children(
                (0..PROGRAMS[bottle].len()).map(|index| program_card(bottle, index)),
            )
            .spacing(12)
            .into()
        }
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

fn program_card(bottle: usize, index: usize) -> Element<'static, Message> {
    let (title, subtitle) = PROGRAMS[bottle][index];

    ArtworkCard::new(title, subtitle)
        .secondary(CardAction::new("Settings", Icon::Gear).on_press(Message::Noop))
        .primary(CardAction::new("Play", Icon::Play).on_press(Message::Noop))
        .banner(sample_image(bottle, index))
        .into()
}

fn sample_image(bottle: usize, index: usize) -> image::Handle {
    let seed = (bottle * PROGRAMS[0].len() + index) as u8;
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
