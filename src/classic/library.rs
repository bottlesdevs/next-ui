//! Library tab backed directly by `next-core`.

use std::{collections::HashMap, sync::Arc};

use bottles_core::{Library, LibraryItem, error::Error as CoreError};
use iced::{
    Element, Fill, Length,
    futures::{FutureExt as _, StreamExt as _, stream},
    widget::{Grid, column, container},
};
use tokio_util::sync::CancellationToken;

use crate::{
    icons::Icon,
    widgets::{
        artwork_card::{ArtworkCard, CardAction},
        info_card::{InfoCard, Kind},
        search::Search as SearchWidget,
        spacing,
    },
};

const NARROW_CONTENT_MAX_WIDTH: f32 = 500.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum LibraryState {
    Idle,
    Loading,
    Loaded,
}

struct Search {
    entries: Vec<LibraryItem>,
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

    fn push_entries(&mut self, generation: u64, entries: Vec<LibraryItem>) {
        if generation == self.generation {
            self.state = LibraryState::Loaded;
            self.entries = entries;
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
    Listed {
        generation: u64,
        result: Result<Vec<LibraryItem>, Arc<CoreError>>,
    },
    Loaded(u64),
    Launch(LibraryItem),
    Launched(Result<(), Arc<CoreError>>),
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
        let library = self.library.clone();
        let query = self.query.trim().to_lowercase();
        let events = stream::once(async move {
            let result = library
                .list()
                .await
                .map(|items| {
                    items
                        .into_iter()
                        .filter(|item| item.entry().title.to_lowercase().contains(&query))
                        .collect()
                })
                .map_err(Arc::new);
            Message::Listed { generation, result }
        })
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
            Message::Listed {
                generation,
                result: Ok(entries),
            } => self.search.push_entries(generation, entries),
            Message::Listed {
                generation,
                result: Err(error),
            } => {
                if generation == self.search.generation {
                    self.last_error = Some(error.to_string());
                }
            }
            Message::Loaded(generation) => self.search.finish(generation),
            Message::Launch(item) => {
                if self.launching {
                    return (iced::Task::none(), None);
                }
                self.launching = true;
                return (
                    iced::Task::perform(
                        async move { item.launch()?.await }.map(|result| result.map_err(Arc::new)),
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
        }

        (iced::Task::none(), None)
    }

    pub fn has_active_operation(&self) -> bool {
        self.launching || self.search.is_active()
    }

    pub fn cancel_active_operations(&self) {
        self.search.cancel();
    }

    pub fn view(&self) -> Element<'_, Message> {
        let search = centered_narrow(SearchWidget::new(
            "Search library",
            &self.query,
            Message::QueryChanged,
        ));

        let notice = match self.search.state {
            LibraryState::Idle => Some((
                "No active profile",
                "Sign in to a profile to see its library.",
            )),
            LibraryState::Loading => Some((
                "Loading library",
                "Loading games from this profile's linked storefronts.",
            )),
            LibraryState::Loaded => None,
        };
        if let Some((title, body)) = notice {
            return column![
                search,
                centered_narrow(InfoCard::new(Kind::Hint, title, body).width(Fill))
            ]
            .spacing(12)
            .into();
        }

        let mut content = column![search].spacing(12);
        if let Some(error) = &self.last_error {
            content = content.push(centered_narrow(
                InfoCard::new(Kind::Error, "Library error", error).width(Fill),
            ));
        }
        if self.search.entries.is_empty() {
            return content
                .push(centered_narrow(
                    InfoCard::new(
                        Kind::Hint,
                        "Nothing here yet",
                        "Registered programs and linked storefront games will show up here.",
                    )
                    .width(Fill),
                ))
                .into();
        }

        let rows = Grid::with_children(self.search.entries.iter().map(entry_card))
            .fluid(400.0)
            .spacing(spacing::MD)
            .height(Length::Shrink);

        content.push(rows).into()
    }
}

fn centered_narrow<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(
        container(content)
            .width(Fill)
            .max_width(NARROW_CONTENT_MAX_WIDTH),
    )
    .center_x(Fill)
    .into()
}

fn entry_card(item: &LibraryItem) -> Element<'_, Message> {
    ArtworkCard::new(&item.entry().title, item.provider_id())
        .menu(CardAction::new("More actions", Icon::EllipsisVertical))
        .primary(CardAction::new("Play", Icon::Play).on_press(Message::Launch(item.clone())))
        .into()
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
