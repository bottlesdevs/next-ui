use iced::{
    Element, Fill, Task, Theme,
    widget::{Space, column, image, row, scrollable, text},
};
use next_ui::components::{
    action_row, button, card,
    heading::{self, Level},
    info_panel::{self, Kind},
    info_row, picker_row, popover,
    search::{self, Action as SearchAction},
    selector_row, status_bar, switcher, tab, tabs, text_field, text_row, title,
};
use next_ui::{icons, theme};

const POPOVER_OPTIONS: &[&str] = &["Option 1", "Option 2", "Option 3", "Option 4"];
const SELECTOR_OPTIONS: &[&str] = &["Option 1", "Option 2", "Option 3"];
const TAB_LABELS: &[&str] = &["Bottles", "Library", "Settings"];
const VALUES: &[&str] = &["One", "Two", "Three"];
const TEXT_ROW_IDS: [&str; 3] = ["text-row-1", "text-row-2", "text-row-3"];
const SEARCH_CATALOG: &[(&str, SearchAction)] = &[
    ("Epic Games Store", SearchAction::Install),
    ("Epic Fight", SearchAction::Run),
    ("GOG Galaxy", SearchAction::Install),
    ("Steam", SearchAction::Run),
];
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
    text_rows: [String; 3],
    selected_option: Option<&'static str>,
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
            text_rows: std::array::from_fn(|_| String::new()),
            selected_option: None,
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
    TextRowChanged(usize, String),
    TextRowPressed(usize),
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
            Message::TextRowChanged(index, value) => self.text_rows[index] = value,
            Message::TextRowPressed(index) => {
                return iced::widget::operation::focus(TEXT_ROW_IDS[index]);
            }
            Message::OptionSelected(value) => {
                self.selected_option = Some(value);
                self.selector_expanded = false;
            }
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
            action_row::ActionRow::new()
                .title("Gaming bottle")
                .description("Gaming")
                .icon(icons::get("controller"))
                .on_press(Message::Noop),
            action_row::ActionRow::new()
                .title("Development bottle")
                .description("Software")
                .icon(icons::get("hollow-gear"))
                .on_press(Message::Noop),
            action_row::ActionRow::new()
                .title("Custom bottle")
                .description("Custom")
                .icon(icons::get("custom"))
                .progress(50),
        ]
        .spacing(12);

        let cards = row![
            card::TextCard::new(
                "Text card",
                "Subtitle",
                "Cards accept caller-owned content and messages.",
            ),
            card::ArtworkCard::new("Artwork card", "Ready", Message::Noop, Message::Noop,)
                .image(sample_image()),
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

        let query = self.search.trim().to_lowercase();
        let search_results = SEARCH_CATALOG
            .iter()
            .filter(|item| item.0.to_lowercase().contains(&query))
            .map(|(title, action)| {
                search::SearchResultRow::new(*title, Message::Noop).action(*action)
            });
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
            .results(search_results)
            .footer("Not listed, install manually", Message::Noop),
        ]
        .spacing(16);

        let selected = self
            .selected_option
            .and_then(|selected| SELECTOR_OPTIONS.iter().find(|option| **option == selected));
        let fields = column![
            text_row::TextRow::new()
                .title("Input Name")
                .placeholder("Placeholder")
                .value(&self.text_rows[0])
                .icon(icons::get("person"))
                .on_input(|value| Message::TextRowChanged(0, value))
                .id(TEXT_ROW_IDS[0])
                .on_press(Message::TextRowPressed(0))
                .variant_1(),
            text_row::TextRow::new()
                .title("Input Name")
                .placeholder("Placeholder")
                .value(&self.text_rows[1])
                .icon(icons::get("person"))
                .on_input(|value| Message::TextRowChanged(1, value))
                .id(TEXT_ROW_IDS[1])
                .on_press(Message::TextRowPressed(1))
                .variant_2(),
            text_row::TextRow::new()
                .title("Input Name")
                .placeholder("Placeholder")
                .value(&self.text_rows[2])
                .icon(icons::get("person"))
                .on_input(|value| Message::TextRowChanged(2, value))
                .id(TEXT_ROW_IDS[2])
                .on_press(Message::TextRowPressed(2))
                .variant_3(),
            selector_row::SelectorRow::new(
                SELECTOR_OPTIONS,
                Message::OptionSelected,
                Message::SelectorToggled,
            )
            .title("Selector Name")
            .placeholder("Placeholder")
            .icon(icons::get("person"))
            .selected(selected)
            .expanded(self.selector_expanded),
            action_row::ActionRow::new()
                .title("Title")
                .description("Description")
                .on_press(Message::Noop),
            info_row::InfoRow::new()
                .title("Title")
                .description("Description")
                .icon(icons::get("timer")),
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
            picker_row::PickerRow::new()
                .title("Title")
                .description("Choose the location")
                .on_press(Message::Noop),
        ]
        .spacing(32);

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

fn sample_image() -> image::Handle {
    image::Handle::from_rgba(
        2,
        2,
        vec![
            51, 53, 71, 255, 53, 71, 51, 255, 92, 63, 63, 255, 107, 93, 71, 255,
        ],
    )
}
