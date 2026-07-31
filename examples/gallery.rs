use iced::{
    Element, Fill, Task, Theme,
    widget::{Space, column, container, image, row, scrollable, text},
};
use next_ui::components::text::TextExt as _;
use next_ui::components::{
    action_row, artwork_card, button, card, cycle_row, expander_row, header_bar,
    info_card::{self, Kind},
    info_row, picker_row, program_card, row_group,
    search::{self, Action as SearchAction},
    selector_row, status_bar, switcher_row, tabs, text_row, title,
};
use next_ui::{icons::Icon, theme};

const SELECTOR_OPTIONS: &[&str] = &["Option 1", "Option 2", "Option 3"];
const TAB_LABELS: &[&str] = &["Bottles", "Library", "Settings"];
const DLSS_LEVELS: &[&str] = &["Off", "Quality", "Balanced", "Performance"];
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
        .style(|_, current_theme| theme::application(current_theme))
        .window_size((1200.0, 900.0))
        .decorations(false)
        .transparent(true)
        .run()
}

fn theme(_: &Gallery) -> Theme {
    theme::theme()
}

struct Gallery {
    search: String,
    text_rows: [String; 3],
    selected_option: Option<&'static str>,
    expander_expanded: bool,
    selected_tab: usize,
    switched_on: bool,
    group_switched_on: bool,
    group_expanded: bool,
    status_expanded: bool,
    value: usize,
}

impl Default for Gallery {
    fn default() -> Self {
        Self {
            search: String::new(),
            text_rows: std::array::from_fn(|_| String::new()),
            selected_option: None,
            expander_expanded: false,
            selected_tab: 0,
            switched_on: false,
            group_switched_on: false,
            group_expanded: true,
            status_expanded: false,
            value: 1,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    SearchChanged(String),
    TextRowChanged(usize, String),
    OptionSelected(&'static str),
    ExpanderToggled,
    TabSelected(usize),
    Switched(bool),
    GroupSwitched(bool),
    GroupToggled,
    HeaderBar(header_bar::Action),
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
            Message::OptionSelected(value) => {
                self.selected_option = Some(value);
            }
            Message::ExpanderToggled => self.expander_expanded = !self.expander_expanded,
            Message::TabSelected(index) => self.selected_tab = index,
            Message::Switched(value) => self.switched_on = value,
            Message::GroupSwitched(value) => self.group_switched_on = value,
            Message::GroupToggled => self.group_expanded = !self.group_expanded,
            Message::HeaderBar(action) => return action.task(),
            Message::StatusToggled => self.status_expanded = !self.status_expanded,
            Message::Previous => {
                self.value = (self.value + DLSS_LEVELS.len() - 1) % DLSS_LEVELS.len();
            }
            Message::Next => self.value = (self.value + 1) % DLSS_LEVELS.len(),
            Message::Noop => {}
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let headings = column![
            text("Heading 1").h1(),
            text("Heading 2").h2(),
            text("Heading 3").h3(),
            text("Heading 4").h4(),
            text("Heading 5").h5(),
        ]
        .spacing(8);

        let titles = row![
            title::Title::new("Title").subtitle("Subtitle"),
            Space::new().width(80),
            title::Title::new("Title").status("Status"),
        ];

        let buttons = row![
            button::Button::new("Play")
                .icon(Icon::Play)
                .rectangular()
                .on_press(Message::Noop),
            button::Button::new("Pill").pill().on_press(Message::Noop),
            button::Button::icon_only("Play", Icon::Play).on_press(Message::Noop),
        ]
        .spacing(12);

        let cards = column![
            row![
                card::Card::new(
                    column![
                        text("Text card").title(),
                        text("Subtitle").subtitle().muted(),
                        text("Cards accept arbitrary content.").body().muted(),
                    ]
                    .spacing(8),
                )
                .padding(24),
                artwork_card::ArtworkCard::new(Message::Noop, Message::Noop)
                    .title("Artwork card")
                    .subtitle("Ready")
                    .banner(sample_image()),
                program_card::ProgramCard::new(Message::Noop, Message::Noop)
                    .title("Program card")
                    .subtitle("Last played today")
                    .banner(sample_image()),
            ]
            .spacing(16),
            row![
                info_card::InfoCard::new(Kind::Hint, "Hint", "Helpful contextual information."),
                info_card::InfoCard::new(Kind::Info, "Info", "General information for the user."),
            ]
            .spacing(12),
            row![
                info_card::InfoCard::new(Kind::Error, "Error", "Something needs attention."),
                info_card::InfoCard::new(Kind::Warning, "Warning", "Proceed with care."),
            ]
            .spacing(12),
            row![
                info_card::InfoCard::new(Kind::Success, "Success", "The operation completed."),
                Space::new().width(Fill),
            ]
            .spacing(12),
        ]
        .spacing(16);

        let tabs = tabs::Tabs::new(
            TAB_LABELS
                .iter()
                .enumerate()
                .map(|(index, label)| tabs::Tab::new(index, label)),
            Some(self.selected_tab),
            Message::TabSelected,
        );

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
            text_row::TextRow::new("Input Name", &self.text_rows[0])
                .placeholder("Placeholder")
                .icon(Icon::Person)
                .on_input(|value| Message::TextRowChanged(0, value)),
            text_row::TextRow::new("Input Name", &self.text_rows[1])
                .placeholder("Placeholder")
                .icon(Icon::Person)
                .on_input(|value| Message::TextRowChanged(1, value)),
            text_row::TextRow::new("Input Name", &self.text_rows[2])
                .placeholder("Placeholder")
                .icon(Icon::Person)
                .on_input(|value| Message::TextRowChanged(2, value))
                .error(Some("Example validation error")),
            selector_row::SelectorRow::new(
                "Selector Name",
                SELECTOR_OPTIONS,
                selected,
                Message::OptionSelected,
            )
            .placeholder("Placeholder")
            .icon(Icon::Person),
            action_row::ActionRow::new("Title", action_row::ActionRowState::Ready(Message::Noop),)
                .description("Description"),
            info_row::InfoRow::new("Title")
                .description("Description")
                .icon(Icon::Timer),
            switcher_row::SwitcherRow::new("Title", self.switched_on, Message::Switched)
                .description("Description"),
            cycle_row::CycleRow::new("DLSS Level", DLSS_LEVELS[self.value])
                .on_previous(Message::Previous)
                .on_next(Message::Next),
            picker_row::PickerRow::new("Title")
                .description("Choose the location")
                .on_press(Message::Noop),
            expander_row::ExpanderRow::new(Message::ExpanderToggled)
                .header(
                    switcher_row::SwitcherRow::new("FSR", self.switched_on, Message::Switched)
                        .description("FidelityFX Super Resolution"),
                )
                .expanded(self.expander_expanded)
                .columns(2)
                .add(
                    action_row::ActionRow::new(
                        "Quality",
                        action_row::ActionRowState::Ready(Message::Noop),
                    )
                    .description("Balanced"),
                )
                .add(
                    cycle_row::CycleRow::new("Sharpening", "5")
                        .on_previous(Message::Previous)
                        .on_next(Message::Next),
                )
                .content_enabled(self.switched_on),
        ]
        .spacing(32);

        let row_group = row_group::RowGroup::new()
            .title("Graphics")
            .description("Rows wrap according to the configured column count.")
            .columns(2)
            .add(
                switcher_row::SwitcherRow::new("DLSS", self.switched_on, Message::Switched)
                    .description("Deep Learning Super Sampling"),
            )
            .add(
                picker_row::PickerRow::new("Shader directory")
                    .description("Choose the location")
                    .on_press(Message::Noop),
            )
            .add(
                action_row::ActionRow::new(
                    "Discrete GPU",
                    action_row::ActionRowState::Ready(Message::Noop),
                )
                .description("Configure graphics adapter"),
            )
            .add(
                expander_row::ExpanderRow::new(Message::GroupToggled)
                    .header(
                        switcher_row::SwitcherRow::new(
                            "FSR",
                            self.group_switched_on,
                            Message::GroupSwitched,
                        )
                        .description("FidelityFX Super Resolution"),
                    )
                    .expanded(self.group_expanded)
                    .columns(2)
                    .add(
                        action_row::ActionRow::new(
                            "Quality",
                            action_row::ActionRowState::Ready(Message::Noop),
                        )
                        .description("Balanced"),
                    )
                    .add(
                        cycle_row::CycleRow::new("Sharpening", DLSS_LEVELS[self.value])
                            .on_previous(Message::Previous)
                            .on_next(Message::Next),
                    )
                    .content_enabled(self.group_switched_on),
            );

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

        let header = header_bar::HeaderBar::new(Message::HeaderBar).middle(
            iced::widget::container(
                search::Search::new(
                    "Search for software and games…",
                    &self.search,
                    Message::SearchChanged,
                )
                .padding_y(8),
            )
            .width(370),
        );

        let gallery = scrollable(
            container(
                column![
                    section("Headings", headings),
                    section("Title", titles),
                    section("Buttons", buttons),
                    section("Cards", cards),
                    section("Tabs", tabs),
                    section("Search", search),
                    section("Rows", fields),
                    section("Row group", row_group),
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

        container(column![header, gallery])
            .width(Fill)
            .height(Fill)
            .padding(1)
            .style(theme::window)
            .clip(true)
            .into()
    }
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
