use iced::{
    Element, Fill, Task, Theme,
    widget::{Space, column, container, image, row, scrollable, text},
};
use next_ui::components::{
    bottle_entry::{self, BottleEntryStatus, BottleKind},
    button, card,
    heading::{self, Level},
    info_panel::{self, Kind},
    popover,
    search::{self, Action as SearchAction},
    status_bar, switcher, tab, tabs, text_field, title,
};
use next_ui::{icons, theme};

const POPOVER_OPTIONS: &[&str] = &["Option 1", "Option 2", "Option 3", "Option 4"];
const SELECTOR_OPTIONS: &[&str] = &["Option 1", "Option 2", "Option 3"];
const TAB_LABELS: &[&str] = &["Bottles", "Library", "Settings"];
const VALUES: &[&str] = &["One", "Two", "Three"];
const LOG: &str = "11:42:12 (INFO) Doing runner update for bottle: games\n\
11:42:12 (INFO) Setting Key Runner=caffe-8.18\n\
11:42:36 (INFO) Using Wine Registry CLI";

fn main() -> iced::Result {
    iced::application(Gallery::default, Gallery::update, Gallery::view)
        .title("Bottles Next component gallery")
        .theme(theme)
        .window_size((1200.0, 900.0))
        .run()
}

fn theme(_: &Gallery) -> Theme {
    theme::theme()
}

struct Gallery {
    search: String,
    input: String,
    selected_option: &'static str,
    selector_expanded: bool,
    selected_popover: usize,
    selected_tab: usize,
    switched_on: bool,
    details_expanded: bool,
    status_expanded: bool,
    value: usize,
}

impl Default for Gallery {
    fn default() -> Self {
        Self {
            search: String::new(),
            input: String::new(),
            selected_option: SELECTOR_OPTIONS[0],
            selector_expanded: false,
            selected_popover: 2,
            selected_tab: 0,
            switched_on: true,
            details_expanded: false,
            status_expanded: false,
            value: 1,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    SearchChanged(String),
    InputChanged(String),
    OptionSelected(&'static str),
    SelectorToggled,
    PopoverSelected(usize),
    TabSelected(usize),
    Switched(bool),
    DetailsToggled,
    StatusToggled,
    Previous,
    Next,
    Noop,
}

impl Gallery {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchChanged(value) => self.search = value,
            Message::InputChanged(value) => self.input = value,
            Message::OptionSelected(value) => self.selected_option = value,
            Message::SelectorToggled => self.selector_expanded = !self.selector_expanded,
            Message::PopoverSelected(index) => self.selected_popover = index,
            Message::TabSelected(index) => self.selected_tab = index,
            Message::Switched(value) => self.switched_on = value,
            Message::DetailsToggled => self.details_expanded = !self.details_expanded,
            Message::StatusToggled => self.status_expanded = !self.status_expanded,
            Message::Previous => self.value = (self.value + VALUES.len() - 1) % VALUES.len(),
            Message::Next => self.value = (self.value + 1) % VALUES.len(),
            Message::Noop => {}
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let headings = column![
            heading::Heading::new(Level::H1, "Heading 1"),
            heading::Heading::new(Level::H2, "Heading 2"),
            heading::Heading::new(Level::H3, "Heading 3"),
            heading::Heading::new(Level::H4, "Heading 4"),
            heading::Heading::new(Level::H5, "Heading 5"),
        ]
        .spacing(8);

        let titles = row![
            title::Title::new("Title").subtitle("Subtitle"),
            Space::new().width(80),
            title::Title::new("Title").status("Status"),
        ];

        let buttons = row![
            button::Button::new("Play")
                .icon(icons::play())
                .rectangular()
                .on_press(Message::Noop),
            button::Button::new("Pill").pill().on_press(Message::Noop),
            button::Button::new("Play")
                .icon(icons::play())
                .circular()
                .on_press(Message::Noop),
        ]
        .spacing(12);

        let bottles = column![
            bottle_entry::BottleEntry::new(
                "Gaming bottle",
                BottleKind::Gaming,
                BottleEntryStatus::Ready(Message::Noop),
            ),
            bottle_entry::BottleEntry::new(
                "Development bottle",
                BottleKind::Software,
                BottleEntryStatus::Ready(Message::Noop),
            ),
            bottle_entry::BottleEntry::new(
                "Custom bottle",
                BottleKind::Custom,
                BottleEntryStatus::Progress(50),
            ),
        ]
        .spacing(12);

        let cards = row![
            card::TextCard::new(
                "Text card",
                "Subtitle",
                "Cards accept caller-owned content and messages.",
            ),
            card::ArtworkCard::new("Artwork card", "Ready", Message::Noop, Message::Noop,)
                .artwork(artwork("Artwork")),
            card::ProgramCard::new(
                "Program card",
                "Last played today",
                Message::Noop,
                Message::Noop,
            )
            .image(sample_image()),
        ]
        .spacing(16);

        let panels = column![
            info_panel::InfoPanel::new(Kind::Hint, "Hint", "Helpful contextual information."),
            info_panel::InfoPanel::new(Kind::Info, "Info", "General information for the user."),
            info_panel::InfoPanel::new(Kind::Error, "Error", "Something needs attention."),
            info_panel::InfoPanel::new(Kind::Warning, "Warning", "Proceed with care."),
            info_panel::InfoPanel::new(Kind::Success, "Success", "The operation completed."),
        ]
        .spacing(12);

        let navigation = column![
            row![
                tab::Tab::new("Single tab", Message::Noop).selected(true),
                tab::Tab::new("Inactive tab", Message::Noop),
            ],
            tabs::Tabs::new(TAB_LABELS, Message::TabSelected).selected(self.selected_tab),
            row![
                text("Switcher"),
                Space::new().width(20),
                switcher::Switcher::new(self.switched_on, Message::Switched),
            ],
            popover::Popover::new(POPOVER_OPTIONS, Message::PopoverSelected)
                .selected(Some(self.selected_popover)),
        ]
        .spacing(20);

        let search = column![
            search::Search::new(
                "Search for software and games…",
                &self.search,
                Message::SearchChanged,
            ),
            search::Search::new(
                "Search for software and games…",
                &self.search,
                Message::SearchChanged,
            )
            .results([
                search::SearchResultRow::new("Epic Games Store", Message::Noop)
                    .action(SearchAction::Install),
                search::SearchResultRow::new("Epic fight", Message::Noop).action(SearchAction::Run),
            ])
            .footer("Not listed, install manually  →", Message::Noop),
        ]
        .spacing(16);

        let selected = SELECTOR_OPTIONS
            .iter()
            .find(|option| **option == self.selected_option);
        let fields = column![
            text_field::Editable::new("Input Name", "Editable value", Message::Noop),
            text_field::Input::new(
                "Input Name",
                "Placeholder",
                &self.input,
                Message::InputChanged,
            ),
            text_field::Disabled::new("Input Name", "Disabled", ""),
            text_field::Selector::new(
                "Selector Name",
                "Choose an option",
                SELECTOR_OPTIONS,
                selected,
                Message::OptionSelected,
                Message::SelectorToggled,
            )
            .expanded(self.selector_expanded),
            text_field::Action::new("Action", "Open another view", Message::Noop),
            text_field::Collapsible::new(
                "Collapsible",
                "Show more content",
                Message::DetailsToggled,
            )
            .expanded(self.details_expanded),
            text_field::Toggle::new(
                "Toggle",
                "Caller-owned boolean",
                self.switched_on,
                Message::Switched,
            ),
            text_field::Value::new(
                "Value",
                VALUES[self.value],
                Message::Previous,
                Message::Next
            ),
            text_field::Path::new("Location", "/home/user/Games", Message::Noop),
        ]
        .spacing(12);

        let status = column![
            status_bar::StatusBar::new("Win64", "soda-7.0.9", Message::Noop)
                .running(true)
                .log(LOG),
            status_bar::StatusBar::new("Win64", "soda-7.0.9", Message::StatusToggled)
                .running(true)
                .log(LOG)
                .expanded(self.status_expanded),
        ]
        .spacing(16);

        scrollable(
            column![
                section("Headings", headings),
                section("Title", titles),
                section("Buttons", buttons),
                section("Bottle entry", bottles),
                section("Cards", cards),
                section("Information panels", panels),
                section("Navigation and selection", navigation),
                section("Search", search),
                section("Text fields", fields),
                section("Status bar", status),
            ]
            .spacing(24)
            .padding(24)
            .max_width(1150),
        )
        .width(Fill)
        .height(Fill)
        .into()
    }
}

fn section<'a>(label: &'a str, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![text(label).size(28), content.into()]
        .spacing(12)
        .into()
}

fn artwork(label: &'static str) -> Element<'static, Message> {
    container(text(label)).center_x(Fill).center_y(Fill).into()
}

fn sample_image() -> image::Handle {
    image::Handle::from_rgba(
        2,
        2,
        vec![
            151, 71, 255, 255, 27, 184, 175, 255, 65, 57, 60, 255, 250, 230, 237, 255,
        ],
    )
}
