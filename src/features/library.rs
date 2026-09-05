use std::{collections::HashMap, sync::Arc};

use bottles_core::{Library, LibraryItem, SearchEntry, error::Error as CoreError};
use iced::{
    Task,
    futures::{StreamExt as _, stream},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle,
    Loading,
    Loaded,
}

struct Search {
    entries: Vec<SearchEntry>,
    status: Status,
    generation: u64,
    active: HashMap<u64, CancellationToken>,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            status: Status::Idle,
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
        self.status = Status::Loading;

        let cancellation = CancellationToken::new();
        self.active.insert(self.generation, cancellation.clone());
        (self.generation, cancellation)
    }

    fn push_entry(&mut self, generation: u64, entry: SearchEntry) {
        if generation == self.generation {
            self.status = Status::Loaded;
            self.entries.push(entry);
        }
    }

    fn finish(&mut self, generation: u64) {
        self.active.remove(&generation);
        if generation == self.generation {
            self.status = Status::Loaded;
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

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn status(&self) -> Status {
        self.search.status
    }

    pub fn entries(&self) -> &[SearchEntry] {
        &self.search.entries
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn reload(&mut self) -> Task<Message> {
        let (generation, cancellation) = self.search.begin();
        let cancelled = cancellation.cancelled_owned();
        let events = self
            .library
            .search(self.query.clone())
            .map(move |entry| Message::Entry { generation, entry })
            .take_until(cancelled);
        let finished = stream::once(async move { Message::Loaded(generation) });

        Task::run(events.chain(finished), std::convert::identity)
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Option<Output>) {
        match message {
            Message::QueryChanged(query) => {
                self.query = query;
                return (Task::none(), Some(Output::Reload));
            }
            Message::Entry { generation, entry } => self.search.push_entry(generation, entry),
            Message::Loaded(generation) => self.search.finish(generation),
            Message::Launch(item) => {
                if self.launching {
                    return (Task::none(), None);
                }
                self.launching = true;
                return (
                    Task::perform(
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
        }

        (Task::none(), None)
    }

    pub fn has_active_operation(&self) -> bool {
        self.launching || self.search.is_active()
    }

    pub fn cancel_active_operations(&self) {
        self.search.cancel();
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

        assert!(search.status == Status::Loading);

        search.finish(current);
        assert!(search.status == Status::Loaded);
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
