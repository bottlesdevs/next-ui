//! Library tab backed directly by `next-core`.

use std::{collections::HashMap, sync::Arc};

use bottles_core::{Library, LibraryItem, SearchEntry, SearchSource, error::Error as CoreError};
use iced::{
    Element,
    futures::{StreamExt as _, stream},
    widget::{column, container, text_input},
};
use tokio_util::sync::CancellationToken;

use crate::{
    icons::Icon,
    widgets::{
        action_row::{ActionRow, State as ActionRowState},
        info_card::{InfoCard, Kind},
        row_group::RowGroup,
        split_view::PaneMode,
    },
};

const CONTENT_GRID_BREAKPOINT: f32 = 720.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum LibraryState {
    Idle,
    Loading,
    Loaded,
}

struct Search {
    entries: Vec<SearchEntry>,
    state: LibraryState,
    generation: u64,
    active: HashMap<u64, CancellationToken>,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            state: LibraryState::Idle,
            generation: 0,
            active: HashMap::new(),
        }
    }
}

impl Search {
    fn begin(&mut self) -> (u64, CancellationToken) {
        self.cancel();
        self.generation = self.generation.wrapping_add(1);
        self.entries.clear();
        self.state = LibraryState::Loading;

        let cancellation = CancellationToken::new();
        self.active.insert(self.generation, cancellation.clone());
        (self.generation, cancellation)
    }

    fn push_entry(&mut self, generation: u64, entry: SearchEntry) {
        if generation == self.generation {
            self.state = LibraryState::Loaded;
            self.entries.push(entry);
        }
    }

    fn finish(&mut self, generation: u64) {
        self.active.remove(&generation);
        if generation == self.generation {
            self.state = LibraryState::Loaded;
        }
    }

    fn is_active(&self) -> bool {
        !self.active.is_empty()
    }

    fn cancel(&self) {
        for cancellation in self.active.values() {
            cancellation.cancel();
        }
    }
}

#[derive(Clone)]
pub enum Message {
    QueryChanged(String),
    Entry { generation: u64, entry: SearchEntry },
    Loaded(u64),
    Launch(LibraryItem),
    Launched(Result<u32, Arc<CoreError>>),
    Noop,
}

pub enum Output {
    Reload,
}

pub struct State {
    library: Library,
    query: String,
    search: Search,
    launching: bool,
    last_error: Option<String>,
}

impl State {
    pub fn new(library: Library) -> Self {
        Self {
            library,
            query: String::new(),
            search: Search::default(),
            launching: false,
            last_error: None,
        }
    }

    pub fn reload(&mut self) -> iced::Task<Message> {
        let (generation, cancellation) = self.search.begin();
        let cancelled = cancellation.cancelled_owned();
        let events = self
            .library
            .search(self.query.clone())
            .map(move |entry| Message::Entry { generation, entry })
            .take_until(cancelled);
        let finished = stream::once(async move { Message::Loaded(generation) });

        iced::Task::run(events.chain(finished), std::convert::identity)
    }

    pub fn update(&mut self, message: Message) -> (iced::Task<Message>, Option<Output>) {
        match message {
            Message::QueryChanged(query) => {
                self.query = query;
                return (iced::Task::none(), Some(Output::Reload));
            }
            Message::Entry { generation, entry } => self.search.push_entry(generation, entry),
            Message::Loaded(generation) => self.search.finish(generation),
            Message::Launch(item) => {
                if self.launching {
                    return (iced::Task::none(), None);
                }
                self.launching = true;
                return (
                    iced::Task::perform(
                        async move { item.launch().await.map_err(Arc::new) },
                        Message::Launched,
                    ),
                    None,
                );
            }
            Message::Launched(Err(error)) => {
                self.launching = false;
                self.last_error = Some(error.to_string());
            }
            Message::Launched(Ok(_)) => {
                self.launching = false;
                self.last_error = None;
            }
            Message::Noop => {}
        }

        (iced::Task::none(), None)
    }

    pub fn has_active_operation(&self) -> bool {
        self.launching || self.search.is_active()
    }

    pub fn cancel_active_operations(&self) {
        self.search.cancel();
    }

    pub fn view(&self, width: f32, mode: PaneMode) -> Element<'_, Message> {
        let search = text_input("Search library", &self.query)
            .on_input(Message::QueryChanged)
            .padding(12);

        match self.search.state {
            LibraryState::Idle => {
                return container(
                    column![
                        search,
                        InfoCard::new(
                            Kind::Hint,
                            "No active profile",
                            "Sign in to a profile to see its library.",
                        )
                    ]
                    .spacing(12),
                )
                .max_width(1150)
                .into();
            }
            LibraryState::Loading => {
                return container(
                    column![
                        search,
                        InfoCard::new(
                            Kind::Hint,
                            "Loading library",
                            "Loading games from this profile's linked storefronts.",
                        )
                    ]
                    .spacing(12),
                )
                .max_width(1150)
                .into();
            }
            LibraryState::Loaded => {}
        }

        let mut content = column![search].spacing(12);
        if let Some(error) = &self.last_error {
            content = content.push(InfoCard::new(Kind::Error, "Program launch failed", error));
        }
        if self.search.entries.is_empty() {
            return container(content.push(InfoCard::new(
                Kind::Hint,
                "Nothing here yet",
                "Registered programs and linked storefront games will show up here.",
            )))
            .max_width(1150)
            .into();
        }

        let columns = usize::from(mode == PaneMode::Single && width >= CONTENT_GRID_BREAKPOINT) + 1;
        let rows =
            self.search
                .entries
                .iter()
                .fold(RowGroup::new().columns(columns), |rows, entry| {
                    rows.add(
                        ActionRow::new(entry.title(), ActionRowState::Ready(entry_action(entry)))
                            .description(entry.source_name())
                            .icon(source_icon(entry.source())),
                    )
                });

        container(content.push(rows)).max_width(1150).into()
    }
}

fn entry_action(entry: &SearchEntry) -> Message {
    match entry.source() {
        SearchSource::Installed(item) => Message::Launch(item.clone()),
        _ => Message::Noop,
    }
}

fn source_icon(source: &SearchSource) -> Icon {
    match source {
        SearchSource::Installed(_) => Icon::Controller,
        SearchSource::Storefront { .. } => Icon::Disk,
        _ => Icon::Warning,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_search_completions_are_ignored() {
        let mut search = Search::default();
        let (stale, _) = search.begin();
        let (current, _) = search.begin();

        search.finish(stale);

        assert!(search.state == LibraryState::Loading);

        search.finish(current);
        assert!(search.state == LibraryState::Loaded);
    }

    #[test]
    fn cancelled_searches_remain_active_until_their_terminal_message() {
        let mut search = Search::default();
        let (generation, cancellation) = search.begin();

        search.cancel();

        assert!(cancellation.is_cancelled());
        assert!(search.is_active());

        search.finish(generation);
        assert!(!search.is_active());
    }
}
