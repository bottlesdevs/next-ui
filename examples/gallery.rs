use iced::{
    Center, Element, Fill, Subscription, Task, Theme,
    keyboard::{self, key},
    widget::{Space, column, container, image, row, scrollable, text},
};
use next_ui::widgets::text::TextExt as _;
use next_ui::widgets::{
    action_row, artwork_card, button, card, cycle_row, dialog, drop_target, expander_row,
    header_bar,
    info_card::{self, Kind},
    info_row, picker_row, popover, row_group, search, selector_row, status_bar, switcher_row, tabs,
    text_row, title,
};
use next_ui::{icons::Icon, theme, ui::chrome};

const SELECTOR_OPTIONS: &[&str] = &["Option 1", "Option 2", "Option 3"];
const EMPTY_OPTIONS: &[&str] = &[];
const TAB_LABELS: &[&str] = &["Bottles", "Library", "Settings"];
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
    MoveFocus(bool),
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
            Message::Window(chrome::Action::RequestClose) => return iced::exit(),
            Message::Window(action) => return action.task().unwrap_or_else(Task::none),
            Message::OpenDialog => self.dialog_open = true,
            Message::DismissDialog => self.dialog_open = false,
            Message::Previous => self.value = self.value.saturating_sub(1),
            Message::Next => self.value = (self.value + 1).min(DLSS_LEVELS.len() - 1),
            Message::MoveFocus(previous) => {
                return if previous {
                    iced::widget::operation::focus_previous()
                } else {
                    iced::widget::operation::focus_next()
                };
            }
            Message::Noop => {}
        }

        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Tab),
                modifiers,
                repeat: false,
                ..
            } => Some(Message::MoveFocus(modifiers.shift())),
            _ => None,
        })
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
            title::Title::new("Title").subtitle("Subtitle"),
            title::Title::new("Title").status("Status"),
        ]
        .spacing(24);

        let buttons = row![
            button::Button::new("Play")
                .icon(Icon::Play)
                .on_press(Message::Noop),
            button::Button::new("Pill").pill().on_press(Message::Noop),
            button::Button::icon_only("Play", Icon::Play).on_press(Message::Noop),
            button::Button::new("Disabled"),
            button::Button::new("Loading")
                .on_press(Message::Noop)
                .loading(true),
            button::Button::new("Open dialog").on_press(Message::OpenDialog),
        ]
        .spacing(12);

        let cards = column![
            drop_target_example(),
            row![
                card::Card::new(
                    column![
                        text("Text card").title(),
                        text("Subtitle").subtitle().muted(),
                        text("Cards accept arbitrary content.").body().muted(),
                    ]
                    .spacing(6),
                )
                .width(Fill)
                .padding(24),
                artwork_card::ArtworkCard::new("Artwork card", "Ready")
                    .menu(
                        artwork_card::CardAction::new("More actions", Icon::EllipsisVertical)
                            .on_press(Message::Noop),
                    )
                    .primary(
                        artwork_card::CardAction::new("Play", Icon::Play).on_press(Message::Noop),
                    )
                    .banner(sample_image()),
                artwork_card::ArtworkCard::new("Program card", "Last played today")
                    .secondary(
                        artwork_card::CardAction::new("Settings", Icon::Gear)
                            .on_press(Message::Noop),
                    )
                    .primary(
                        artwork_card::CardAction::new("Play", Icon::Play)
                            .on_press(Message::Noop)
                            .loading(true),
                    )
                    .banner(sample_image()),
            ]
            .spacing(18),
            row![
                info_card::InfoCard::new(Kind::Hint, "Hint", "Helpful contextual information.")
                    .width(Fill),
                info_card::InfoCard::new(Kind::Info, "Info", "General information for the user.")
                    .width(Fill),
            ]
            .spacing(12),
            row![
                info_card::InfoCard::new(Kind::Error, "Error", "Something needs attention.")
                    .width(Fill),
                info_card::InfoCard::new(Kind::Warning, "Warning", "Proceed with care.")
                    .width(Fill),
            ]
            .spacing(12),
            row![
                info_card::InfoCard::new(Kind::Success, "Success", "The operation completed.")
                    .width(Fill),
                Space::new().width(Fill),
            ]
            .spacing(12),
        ]
        .spacing(18);

        let tabs = tabs::Tabs::new(
            TAB_LABELS
                .iter()
                .enumerate()
                .map(|(index, label)| tabs::Tab::new(index, label)),
            Some(self.selected_tab),
            Message::TabSelected,
        );

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
            .state(self.search_state())
            .footer("Not listed, install manually", Message::Noop),
            search::Search::new(
                "Focus to see loading state…",
                &self.search,
                Message::SearchChanged,
            )
            .state(search::SearchState::Loading),
            search::Search::new(
                "Focus to see error state…",
                &self.search,
                Message::SearchChanged,
            )
            .state(search::SearchState::Error(
                "The catalog could not be loaded"
            )),
        ]
        .spacing(18);

        let selected = self
            .selected_option
            .and_then(|selected| SELECTOR_OPTIONS.iter().find(|option| **option == selected));
        let popover = popover::Popover::new(
            button::Button::new("Open menu")
                .trailing_icon(Icon::DownCaret)
                .on_press(()),
        )
        .item(
            popover::PopoverItem::new("Current profile")
                .subtitle("Selected")
                .icon(Icon::Person)
                .selected(true)
                .on_select(Message::Noop),
        )
        .item(
            popover::PopoverItem::new("Available account")
                .subtitle("Child action captures the row click")
                .action("Link", Message::Noop),
        )
        .item(
            popover::PopoverItem::new("Unavailable account")
                .disabled_action("Taken")
                .tooltip(text("Already linked to another profile")),
        )
        .item(popover::PopoverItem::new("Manage profiles").on_select(Message::Noop));
        let popovers = column![popover].spacing(12);
        let fields = column![
            text_row::TextRow::new("Input Name", &self.text_rows[0])
                .placeholder("Placeholder")
                .icon(Icon::Person)
                .on_input(|value| Message::TextRowChanged(0, value)),
            text_row::TextRow::new("Input Name", &self.text_rows[1])
                .placeholder("Placeholder")
                .icon(Icon::Person)
                .secure(true)
                .on_input(|value| Message::TextRowChanged(1, value)),
            text_row::TextRow::new("Input Name", &self.text_rows[2])
                .placeholder("Placeholder")
                .icon(Icon::Person)
                .on_input(|value| Message::TextRowChanged(2, value))
                .error(Some("Example validation error")),
            selector_row::SelectorRow::new("Selector Name", SELECTOR_OPTIONS, selected,)
                .on_selected(Message::OptionSelected)
                .placeholder("Placeholder")
                .icon(Icon::Person),
            selector_row::SelectorRow::new("Empty selector", EMPTY_OPTIONS, None)
                .placeholder("No options available"),
            action_row::ActionRow::new("Title", action_row::State::Ready(Message::Noop))
                .description("Description"),
            action_row::ActionRow::new("Unavailable action", action_row::State::Disabled)
                .description("This action cannot currently run"),
            info_row::InfoRow::new("Title")
                .description("Description")
                .icon(Icon::Timer),
            switcher_row::SwitcherRow::new("Title", self.switched_on)
                .on_toggle(Message::Switched)
                .description("Description"),
            cycle_row::CycleRow::new("DLSS Level", DLSS_LEVELS[self.value])
                .on_previous_maybe((self.value > 0).then_some(Message::Previous))
                .on_next_maybe((self.value + 1 < DLSS_LEVELS.len()).then_some(Message::Next),),
            picker_row::PickerRow::new("Title")
                .description("Choose the location")
                .on_press(Message::Noop),
            expander_row::ExpanderRow::with_header(
                switcher_row::SwitcherRow::new("FSR", self.switched_on)
                    .on_toggle(Message::Switched)
                    .description("FidelityFX Super Resolution"),
            )
            .columns(2)
            .add(
                action_row::ActionRow::new("Quality", action_row::State::Ready(Message::Noop),)
                    .description("Balanced"),
            )
            .add(
                cycle_row::CycleRow::new("Sharpening", "5")
                    .on_previous_maybe((self.value > 0).then_some(Message::Previous))
                    .on_next_maybe((self.value + 1 < DLSS_LEVELS.len()).then_some(Message::Next),),
            )
            .content_enabled(self.switched_on),
        ]
        .spacing(24);

        let row_group = row_group::RowGroup::new()
            .title("Graphics")
            .description("Rows wrap according to the configured column count.")
            .columns(2)
            .row(
                switcher_row::SwitcherRow::new("DLSS", self.switched_on)
                    .on_toggle(Message::Switched)
                    .description("Deep Learning Super Sampling"),
            )
            .row(
                picker_row::PickerRow::new("Shader directory")
                    .description("Choose the location")
                    .on_press(Message::Noop),
            )
            .row(
                action_row::ActionRow::new("Discrete GPU", action_row::State::Ready(Message::Noop))
                    .description("Configure graphics adapter"),
            )
            .expander(
                expander_row::ExpanderRow::with_header(
                    switcher_row::SwitcherRow::new("FSR", self.group_switched_on)
                        .on_toggle(Message::GroupSwitched)
                        .description("FidelityFX Super Resolution"),
                )
                .columns(2)
                .add(
                    action_row::ActionRow::new("Quality", action_row::State::Ready(Message::Noop))
                        .description("Balanced"),
                )
                .add(
                    cycle_row::CycleRow::new("Sharpening", DLSS_LEVELS[self.value])
                        .on_previous_maybe((self.value > 0).then_some(Message::Previous))
                        .on_next_maybe(
                            (self.value + 1 < DLSS_LEVELS.len()).then_some(Message::Next),
                        ),
                )
                .content_enabled(self.group_switched_on),
            );

        let multiple_expanders = row_group::RowGroup::new()
            .title("Non-overlapping expanders")
            .description("Both expanders can remain open because their panels do not overlap")
            .columns(3)
            .expander(
                expander_row::ExpanderRow::new("First expander")
                    .description("One-column panel")
                    .add(
                        action_row::ActionRow::new(
                            "First action",
                            action_row::State::Ready(Message::Noop),
                        )
                        .description("Inside the first column"),
                    ),
            )
            .expander(
                expander_row::ExpanderRow::new("Second expander")
                    .description("Two-column panel")
                    .columns(2)
                    .add(
                        action_row::ActionRow::new(
                            "Second action",
                            action_row::State::Ready(Message::Noop),
                        )
                        .description("First panel column"),
                    )
                    .add(
                        action_row::ActionRow::new(
                            "Third action",
                            action_row::State::Ready(Message::Noop),
                        )
                        .description("Second panel column"),
                    ),
            )
            .row(
                action_row::ActionRow::new(
                    "Independent action",
                    action_row::State::Ready(Message::Noop),
                )
                .description("Beside the two expanders"),
            );

        let expander_matrix = row_group::RowGroup::new()
            .title("2 × 2 expander grid")
            .description("Opening a sibling closes the overlapping panel on the same grid line")
            .columns(2)
            .expander(action_grid_expander("Expander A"))
            .expander(action_grid_expander("Expander B"))
            .expander(action_grid_expander("Expander C"))
            .expander(action_grid_expander("Expander D"));

        let status = column![
            status_bar::StatusBar::new("Win64", "soda-7.0.9", status_bar::BottleStatus::Running,)
                .log(LOG),
            status_bar::StatusBar::new("Win64", "soda-7.0.9", status_bar::BottleStatus::Stopped,)
                .log(LOG),
            status_bar::StatusBar::new("Win64", "soda-7.0.9", status_bar::BottleStatus::Starting,),
            status_bar::StatusBar::new("Win64", "soda-7.0.9", status_bar::BottleStatus::Failed,),
        ]
        .spacing(18);

        let header = header_bar::HeaderBar::new(Message::Window(chrome::Action::Drag)).middle(
            iced::widget::container(
                search::Search::new(
                    "Search for software and games…",
                    &self.search,
                    Message::SearchChanged,
                )
                .state(self.search_state()),
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
                    section("Popovers", popovers),
                    section("Rows", fields),
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

        let page: Element<'_, Message> =
            chrome::WindowFrame::new(column![header, gallery], Message::Window).into();
        let dialog = self.dialog_open.then(|| {
            dialog::Dialog::new(
                column![
                    title::Title::new("Dialog").subtitle("Modal content can use any widget."),
                    button::Button::new("Close")
                        .kind(button::ButtonKind::Primary)
                        .on_press(Message::DismissDialog),
                ]
                .spacing(18),
                Message::DismissDialog,
            )
        });

        dialog::WindowModal::new(page).dialog(dialog).into()
    }

    fn search_state(&self) -> search::SearchState<'_, Message> {
        let query = self.search.trim().to_lowercase();

        if query.is_empty() {
            return search::SearchState::Hidden;
        }

        let results: Vec<_> = SEARCH_CATALOG
            .iter()
            .filter(|(title, _, _)| title.to_lowercase().contains(&query))
            .map(|(title, action, action_icon)| {
                search::SearchResult::new(*title, *title, Message::Noop)
                    .icon(Icon::Bottles)
                    .action(action, *action_icon, Message::Noop)
            })
            .collect();

        if results.is_empty() {
            search::SearchState::Empty
        } else {
            search::SearchState::Results(results)
        }
    }
}

fn drop_target_example<'a>() -> drop_target::DropTarget<'a, Message> {
    const ICON_CONTAINER_SIZE: f32 = 44.0;

    let icon = container(Icon::Plus.view().width(16).height(16))
        .width(ICON_CONTAINER_SIZE)
        .height(ICON_CONTAINER_SIZE)
        .align_x(Center)
        .align_y(Center)
        .style(|theme: &Theme| {
            container::Style::default()
                .background(theme.extended_palette().background.weak.color)
                .border(iced::Border::default().rounded(ICON_CONTAINER_SIZE / 2.0))
        });
    let labels = column![
        text("New Program").size(17).medium(),
        text("Install or add a program.").size(14),
    ]
    .spacing(6);

    let content = container(row![icon, labels].spacing(16).align_y(Center)).center_x(Fill);

    drop_target::DropTarget::new(content, Message::Noop)
        .width(Fill)
        .padding([72.0, 24.0])
}

fn action_grid_expander(title: &'static str) -> expander_row::ExpanderRow<'static, Message> {
    ["Action 1", "Action 2", "Action 3", "Action 4"]
        .into_iter()
        .fold(
            expander_row::ExpanderRow::new(title)
                .description("Contains four actions")
                .columns(2),
            |expander, title| {
                expander.add(
                    action_row::ActionRow::new(title, action_row::State::Ready(Message::Noop))
                        .description("Available action"),
                )
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
