#[path = "../src/chrome.rs"]
mod chrome;

use iced::{
    Center, Element, Fill, Subscription, Task, Theme,
    widget::{Space, column, container, grid, image, row, scrollable, text},
};
use next_ui::{
    Icon, theme,
    widget::{
        ActionRow, ActionTile, ArtworkCard, Button, ButtonKind, Card, CardAction, CycleRow, Dialog,
        ExpanderRow, InfoCard, InfoCardKind, InfoRow, ListRow, LogPanel, PickerRow, Popover,
        PopoverItem, ProgressRing, RowGroup, Search, SearchResult, SearchState, SelectorRow,
        StatusBar, Switcher, SwitcherRow, Tab, Tabs, TextExt as _, TextRow, Title, WindowModal,
    },
};

const SELECTOR_OPTIONS: &[&str] = &["Option 1", "Option 2", "Option 3"];
const EMPTY_OPTIONS: &[&str] = &[];
const GALLERY_TABS: &[&str] = &["Bottles", "Library", "Settings"];
const DLSS_LEVELS: &[&str] = &["Off", "Quality", "Balanced", "Performance"];
const SEARCH_CATALOG: &[(&str, &str, Icon)] = &[
    ("Epic Games Store", "Install", Icon::Arrow),
    ("Epic Fight", "Run", Icon::Play),
    ("GOG Galaxy", "Install", Icon::Arrow),
    ("Steam", "Run", Icon::Play),
];
const LOG: &str = "11:42:12 (INFO) Doing runner update for bottle: games\n\
11:42:12 (INFO) Setting Key Runner=caffe-8.18\n\
11:42:36 (INFO) Using Wine Registry CLI";

fn main() -> iced::Result {
    iced::application(Gallery::default, Gallery::update, Gallery::view)
        .title("Bottles Next component gallery")
        .theme(theme)
        .subscription(Gallery::subscription)
        .style(|_, current_theme| theme::application(current_theme))
        .window_size((1200.0, 900.0))
        .decorations(false)
        .transparent(true)
        .run()
}

fn theme(_: &Gallery) -> Theme {
    theme::dark()
}

struct Gallery {
    search: String,
    text_rows: [String; 3],
    selected_option: Option<&'static str>,
    selected_tab: usize,
    switched_on: bool,
    group_switched_on: bool,
    dialog_open: bool,
    value: usize,
}

impl Default for Gallery {
    fn default() -> Self {
        Self {
            search: String::new(),
            text_rows: std::array::from_fn(|_| String::new()),
            selected_option: None,
            selected_tab: 0,
            switched_on: false,
            group_switched_on: false,
            dialog_open: false,
            value: 1,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    SearchChanged(String),
    TextRowChanged(usize, String),
    OptionSelected(&'static str),
    TabSelected(usize),
    Switched(bool),
    GroupSwitched(bool),
    Window(chrome::Action),
    OpenDialog,
    DismissDialog,
    Previous,
    Next,
    Noop,
}

impl Gallery {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchChanged(value) => self.search = value,
            Message::TextRowChanged(index, value) => self.text_rows[index] = value,
            Message::OptionSelected(value) => {
                self.selected_option = Some(value);
            }
            Message::TabSelected(index) => self.selected_tab = index,
            Message::Switched(value) => self.switched_on = value,
            Message::GroupSwitched(value) => self.group_switched_on = value,
            Message::Window(action) => {
                return match action {
                    chrome::Action::Drag => iced::window::latest().and_then(iced::window::drag),
                    chrome::Action::Resize(direction) => iced::window::latest()
                        .and_then(move |id| iced::window::drag_resize(id, direction)),
                    chrome::Action::RequestClose => iced::exit(),
                    chrome::Action::FocusPrevious => iced::widget::operation::focus_previous(),
                    chrome::Action::FocusNext => iced::widget::operation::focus_next(),
                };
            }
            Message::OpenDialog => self.dialog_open = true,
            Message::DismissDialog => self.dialog_open = false,
            Message::Previous => self.value = self.value.saturating_sub(1),
            Message::Next => self.value = (self.value + 1).min(DLSS_LEVELS.len() - 1),
            Message::Noop => {}
        }

        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        chrome::subscription().map(Message::Window)
    }

    fn view(&self) -> Element<'_, Message> {
        let headings = column![
            text("Heading 1").size(72),
            text("Heading 2").size(64),
            text("Heading 3").h3(),
            text("Heading 4").h4(),
            text("Heading 5").size(40),
        ]
        .spacing(6);

        let titles = row![
            Title::new("Title").subtitle("Subtitle"),
            Title::new("Title").status("Status"),
        ]
        .spacing(24);

        let buttons = row![
            Button::new("Play").icon(Icon::Play).on_press(Message::Noop),
            Button::new("Pill").pill().on_press(Message::Noop),
            Button::icon_only("Play", Icon::Play).on_press(Message::Noop),
            Button::custom(row![Icon::Wand.view(), text("Custom")].spacing(6))
                .on_press(Message::Noop),
            Button::new("Disabled"),
            Button::new("Loading").on_press(Message::Noop).loading(true),
            Button::new("Open dialog").on_press(Message::OpenDialog),
        ]
        .spacing(12);

        let cards = column![
            grid![action_tile_example(true), action_tile_example(false)]
                .fluid(400.0)
                .height(iced::widget::grid::aspect_ratio(3.0, 2.0))
                .spacing(12),
            grid![
                Card::new(
                    column![
                        text("Text card").title(),
                        text("Subtitle").subtitle().muted(),
                        text("Cards accept arbitrary content.").body().muted(),
                    ]
                    .spacing(6),
                )
                .width(Fill)
                .padding(24),
                ArtworkCard::new("Artwork card", "Ready")
                    .menu(
                        CardAction::new("More actions", Icon::EllipsisVertical)
                            .on_press(Message::Noop),
                    )
                    .primary(CardAction::new("Play", Icon::Play).on_press(Message::Noop))
                    .banner(sample_image()),
                ArtworkCard::new("Program card", "Last played today")
                    .secondary(CardAction::new("Settings", Icon::Gear).on_press(Message::Noop),)
                    .primary(
                        CardAction::new("Play", Icon::Play)
                            .on_press(Message::Noop)
                            .loading(true),
                    )
                    .banner(sample_image()),
            ]
            .fluid(400.0)
            .height(iced::widget::grid::aspect_ratio(3.0, 2.0))
            .spacing(12),
            row![
                InfoCard::new(
                    InfoCardKind::Hint,
                    "Hint",
                    "Helpful contextual information.",
                )
                .width(Fill),
                InfoCard::new(
                    InfoCardKind::Info,
                    "Info",
                    "General information for the user.",
                )
                .width(Fill),
            ]
            .spacing(12),
            row![
                InfoCard::new(InfoCardKind::Error, "Error", "Something needs attention.")
                    .width(Fill),
                InfoCard::new(InfoCardKind::Warning, "Warning", "Proceed with care.").width(Fill),
            ]
            .spacing(12),
            row![
                InfoCard::new(InfoCardKind::Success, "Success", "The operation completed.",)
                    .width(Fill),
                Space::new().width(Fill),
            ]
            .spacing(12),
        ]
        .spacing(18);

        let tabs = Tabs::new(
            GALLERY_TABS
                .iter()
                .enumerate()
                .map(|(index, label)| Tab::new(index, label)),
            Some(self.selected_tab),
            Message::TabSelected,
        );

        let search = column![
            Search::new(
                "Search for software and games…",
                &self.search,
                Message::SearchChanged,
            )
            .on_submit(Message::Noop),
            Search::new(
                "Search for software and games…",
                &self.search,
                Message::SearchChanged,
            )
            .state(self.search_state())
            .footer("Not listed, install manually", Message::Noop),
            Search::new(
                "Focus to see loading state…",
                &self.search,
                Message::SearchChanged,
            )
            .state(SearchState::Loading),
            Search::new(
                "Focus to see error state…",
                &self.search,
                Message::SearchChanged,
            )
            .state(SearchState::Error("The catalog could not be loaded")),
        ]
        .spacing(18);

        let selected = self
            .selected_option
            .and_then(|selected| SELECTOR_OPTIONS.iter().find(|option| **option == selected));
        let popover = Popover::new(Button::new("Open menu").trailing_icon(Icon::DownCaret))
            .item(
                PopoverItem::new("Current profile")
                    .subtitle("Selected")
                    .icon(Icon::Person)
                    .selected(true)
                    .on_select(Message::Noop),
            )
            .item(
                PopoverItem::new("Available account")
                    .subtitle("Trailing action is the only interactive target")
                    .action("Link", Message::Noop),
            )
            .item(
                PopoverItem::new("Unavailable account")
                    .disabled_action("Taken")
                    .tooltip(text("Already linked to another profile")),
            )
            .item(PopoverItem::new("Manage profiles").on_select(Message::Noop));
        let popovers = column![popover].spacing(12);
        let fields = column![
            TextRow::new("Input Name", &self.text_rows[0])
                .placeholder("Placeholder")
                .icon(Icon::Person)
                .on_input(|value| Message::TextRowChanged(0, value)),
            TextRow::new("Input Name", &self.text_rows[1])
                .placeholder("Placeholder")
                .icon(Icon::Person)
                .secure(true)
                .on_input(|value| Message::TextRowChanged(1, value)),
            TextRow::new("Input Name", &self.text_rows[2])
                .placeholder("Placeholder")
                .icon(Icon::Person)
                .on_input(|value| Message::TextRowChanged(2, value))
                .error(Some("Example validation error")),
            SelectorRow::new("Selector Name", SELECTOR_OPTIONS, selected)
                .on_selected(Message::OptionSelected)
                .placeholder("Placeholder")
                .icon(Icon::Person),
            SelectorRow::new("Empty selector", EMPTY_OPTIONS, None)
                .placeholder("No options available"),
            ActionRow::new("Title", Message::Noop).description("Description"),
            ActionRow::new("Unavailable action", None)
                .description("This action cannot currently run"),
            ActionRow::progress("In progress", 0.65).description("65% complete"),
            InfoRow::new("Title")
                .description("Description")
                .icon(Icon::Timer),
            ListRow::new(text("Direct ListRow with arbitrary content"))
                .leading(Icon::Info.view().width(16).height(16))
                .trailing(Button::new("Action").on_press(Message::Noop)),
            SwitcherRow::new("Title", self.switched_on)
                .on_toggle(Message::Switched)
                .description("Description"),
            CycleRow::new("DLSS Level", DLSS_LEVELS[self.value])
                .on_previous_maybe((self.value > 0).then_some(Message::Previous))
                .on_next_maybe((self.value + 1 < DLSS_LEVELS.len()).then_some(Message::Next),),
            PickerRow::new("Title")
                .description("Choose the location")
                .on_press(Message::Noop),
            ExpanderRow::with_header(
                SwitcherRow::new("FSR", self.switched_on)
                    .on_toggle(Message::Switched)
                    .description("FidelityFX Super Resolution"),
            )
            .columns(2)
            .row(
                ActionRow::new("Quality", self.switched_on.then_some(Message::Noop))
                    .description("Balanced"),
            )
            .row(
                CycleRow::new("Sharpening", "5")
                    .on_previous_maybe(
                        (self.switched_on && self.value > 0).then_some(Message::Previous),
                    )
                    .on_next_maybe(
                        (self.switched_on && self.value + 1 < DLSS_LEVELS.len())
                            .then_some(Message::Next),
                    ),
            ),
        ]
        .spacing(24);

        let standalone_controls = row![
            column![text("Progress ring").medium(), ProgressRing::new(0.65),].spacing(12),
            column![
                text("Switcher").medium(),
                Switcher::new(self.switched_on).on_toggle(Message::Switched),
            ]
            .spacing(12),
        ]
        .spacing(48);

        let row_group = RowGroup::new()
            .title("Graphics")
            .description("Rows wrap according to the configured column count.")
            .columns(2)
            .row(
                SwitcherRow::new("DLSS", self.switched_on)
                    .on_toggle(Message::Switched)
                    .description("Deep Learning Super Sampling"),
            )
            .row(
                PickerRow::new("Shader directory")
                    .description("Choose the location")
                    .on_press(Message::Noop),
            )
            .row(
                ActionRow::new("Discrete GPU", Message::Noop)
                    .description("Configure graphics adapter"),
            )
            .expander(
                ExpanderRow::with_header(
                    SwitcherRow::new("FSR", self.group_switched_on)
                        .on_toggle(Message::GroupSwitched)
                        .description("FidelityFX Super Resolution"),
                )
                .columns(2)
                .row(
                    ActionRow::new("Quality", self.group_switched_on.then_some(Message::Noop))
                        .description("Balanced"),
                )
                .row(
                    CycleRow::new("Sharpening", DLSS_LEVELS[self.value])
                        .on_previous_maybe(
                            (self.group_switched_on && self.value > 0).then_some(Message::Previous),
                        )
                        .on_next_maybe(
                            (self.group_switched_on && self.value + 1 < DLSS_LEVELS.len())
                                .then_some(Message::Next),
                        ),
                ),
            );

        let multiple_expanders = RowGroup::new()
            .title("Non-overlapping expanders")
            .description("Both expanders can remain open because their panels do not overlap")
            .columns(3)
            .expander(
                ExpanderRow::new("First expander")
                    .description("One-column panel")
                    .row(
                        ActionRow::new("First action", Message::Noop)
                            .description("Inside the first column"),
                    ),
            )
            .expander(
                ExpanderRow::new("Second expander")
                    .description("Two-column panel")
                    .columns(2)
                    .row(
                        ActionRow::new("Second action", Message::Noop)
                            .description("First panel column"),
                    )
                    .row(
                        ActionRow::new("Third action", Message::Noop)
                            .description("Second panel column"),
                    ),
            )
            .row(
                ActionRow::new("Independent action", Message::Noop)
                    .description("Beside the two expanders"),
            );

        let expander_matrix = RowGroup::new()
            .title("2 × 2 expander grid")
            .description("Opening a sibling closes the overlapping panel on the same grid line")
            .columns(2)
            .expander(action_grid_expander("Expander A"))
            .expander(action_grid_expander("Expander B"))
            .expander(action_grid_expander("Expander C"))
            .expander(action_grid_expander("Expander D"));

        let status = column![
            LogPanel::new("Win64", "soda-7.0.9", "Running", Icon::Lightning).log(LOG),
            LogPanel::new("Win64", "soda-7.0.9", "Stopped", Icon::Power).log(LOG),
            LogPanel::new("Win64", "soda-7.0.9", "Starting", Icon::Lightning),
            LogPanel::new("Win64", "soda-7.0.9", "Stopping", Icon::Power),
            LogPanel::new("Win64", "soda-7.0.9", "Failed", Icon::Cross).danger(),
            StatusBar::new("Generic status", Icon::Info).detail(Icon::Chip, "Generic detail"),
        ]
        .spacing(18);

        let header = chrome::header(Message::Window(chrome::Action::Drag), true, |header| {
            header.middle(
                iced::widget::container(
                    Search::new(
                        "Search for software and games…",
                        &self.search,
                        Message::SearchChanged,
                    )
                    .state(self.search_state()),
                )
                .width(370),
            )
        });

        let gallery = scrollable(
            container(
                column![
                    section("Headings", headings),
                    section("Title", titles),
                    section("Buttons", buttons),
                    section("Cards", cards),
                    section("Tabs", tabs),
                    section("Search", search),
                    section("Popovers", popovers),
                    section("Rows", fields),
                    section("Standalone controls", standalone_controls),
                    section("Row group", row_group),
                    section("Overlap-aware expanders", multiple_expanders),
                    section("Expander matrix", expander_matrix),
                    section("Status bar", status),
                ]
                .spacing(12)
                .padding(12)
                .width(Fill)
                .max_width(1150),
            )
            .center_x(Fill),
        )
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(4)
                .scroller_width(4)
                .margin(12),
        ))
        .style(theme::scrollbar)
        .width(Fill)
        .height(Fill);

        let page = chrome::window_frame(column![header, gallery], Message::Window);
        let dialog = self.dialog_open.then(|| {
            Dialog::new(
                column![
                    Title::new("Dialog").subtitle("Modal content can use any widget."),
                    Button::new("Close")
                        .kind(ButtonKind::Primary)
                        .on_press(Message::DismissDialog),
                ]
                .spacing(18),
                Message::DismissDialog,
            )
        });

        WindowModal::new(page).dialog(dialog).into()
    }

    fn search_state(&self) -> SearchState<'_, Message> {
        let query = self.search.trim().to_lowercase();

        if query.is_empty() {
            return SearchState::Hidden;
        }

        let results: Vec<_> = SEARCH_CATALOG
            .iter()
            .filter(|(title, _, _)| title.to_lowercase().contains(&query))
            .map(|(title, action, action_icon)| {
                SearchResult::new(title, Message::Noop)
                    .icon(Icon::Bottles)
                    .action(action, *action_icon, Message::Noop)
            })
            .collect();

        if results.is_empty() {
            SearchState::Empty
        } else {
            SearchState::Results(results)
        }
    }
}

fn action_tile_example<'a>(active: bool) -> ActionTile<'a, Message> {
    const ICON_CONTAINER_SIZE: f32 = 44.0;

    let icon = container(Icon::Plus.view().width(16).height(16))
        .width(ICON_CONTAINER_SIZE)
        .height(ICON_CONTAINER_SIZE)
        .align_x(Center)
        .align_y(Center)
        .style(|theme: &Theme| {
            container::Style::default()
                .background(theme::colors(theme).surface)
                .border(iced::Border::default().rounded(ICON_CONTAINER_SIZE / 2.0))
        });
    let labels = column![
        text(if active {
            "New Program"
        } else {
            "Unavailable target"
        })
        .size(17)
        .medium(),
        text(if active {
            "Install or add a program."
        } else {
            "Activation is disabled."
        })
        .size(14),
    ]
    .spacing(6);

    let content = container(row![icon, labels].spacing(16).align_y(Center))
        .center_x(Fill)
        .center_y(Fill);

    if active {
        ActionTile::new(content, Message::OpenDialog)
            .width(Fill)
            .height(Fill)
            .padding(24)
    } else {
        ActionTile::disabled(content)
            .width(Fill)
            .height(Fill)
            .padding(24)
    }
}

fn action_grid_expander(title: &'static str) -> ExpanderRow<'static, Message> {
    ["Action 1", "Action 2", "Action 3", "Action 4"]
        .into_iter()
        .fold(
            ExpanderRow::new(title)
                .description("Contains four actions")
                .columns(2),
            |expander, title| {
                expander.row(ActionRow::new(title, Message::Noop).description("Available action"))
            },
        )
}

fn section<'a>(label: &'a str, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(column![text(label).title(), content.into()].spacing(12))
        .width(Fill)
        .padding(24)
        .style(theme::panel)
        .into()
}

fn sample_image() -> image::Handle {
    image::Handle::from_rgba(
        2,
        2,
        vec![
            51, 53, 71, 255, 53, 71, 51, 255, 92, 63, 63, 255, 107, 93, 71, 255,
        ],
    )
}
