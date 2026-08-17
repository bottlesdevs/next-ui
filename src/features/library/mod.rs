//! Library tab: games discovered from the active profile's linked
//! storefronts, streamed in via `WatchGames`. The shell drives the
//! `library_events` subscription (keyed on the active profile id, which
//! this feature doesn't own) and maps it into `shell::Message::Library`;
//! profile switches are relayed in via [`State::reset_for_profile`].

use iced::{
    Element,
    futures::{SinkExt as _, StreamExt as _},
    widget::container,
};

use next_proto::bottles::{
    common::v1::{Game, Storefront},
    library::v1::{WatchGamesRequest, game_event, library_client::LibraryClient},
};

use crate::{
    widgets::{
        action_row::{ActionRow, State as ActionRowState},
        info_card::{InfoCard, Kind},
        row_group::RowGroup,
        split_view::PaneMode,
    },
    icons::Icon,
};

const CONTENT_GRID_BREAKPOINT: f32 = 720.0;
const SERVER_ENDPOINT: &str = "http://127.0.0.1:50052";

/// Whether the Library tab has anything to show yet for the active
/// profile — distinct from `games` being empty, since an empty list can
/// mean either "still waiting on the first `WatchGames` event" or
/// "loaded, and there's genuinely nothing linked".
#[derive(Clone, PartialEq, Eq)]
enum LibraryState {
    /// No active profile to load a library for.
    Idle,
    /// Waiting on the first event from `WatchGames`.
    Loading,
    /// At least one event has arrived (or the profile has nothing to
    /// watch in the first place), so an empty `games` list is meaningful.
    Loaded,
    Failed(String),
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct LibraryHandle(pub String);

pub fn library_events(
    handle: &LibraryHandle,
) -> std::pin::Pin<Box<dyn iced::futures::Stream<Item = Message> + Send>> {
    let profile_id = handle.0.clone();

    Box::pin(iced::stream::channel(
        16,
        move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
            let mut client = match LibraryClient::connect(SERVER_ENDPOINT).await {
                Ok(client) => client,
                Err(err) => {
                    let _ = output
                        .send(Message::Error(format!("next-server unavailable: {err}")))
                        .await;
                    return;
                }
            };
            let response = client.watch_games(WatchGamesRequest { profile_id }).await;
            let mut events = match response {
                Ok(response) => response.into_inner(),
                Err(err) => {
                    let _ = output.send(Message::Error(err.to_string())).await;
                    return;
                }
            };

            while let Some(event) = events.next().await {
                match event {
                    Ok(event) => {
                        if let Some(event) = event.event {
                            let _ = output.send(Message::Event(event)).await;
                        }
                    }
                    Err(err) => {
                        let _ = output.send(Message::Error(err.to_string())).await;
                        break;
                    }
                }
            }
        },
    ))
}

#[derive(Clone)]
pub enum Message {
    Event(game_event::Event),
    Error(String),
    Noop,
}

pub struct State {
    games: Vec<Game>,
    state: LibraryState,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            games: Vec::new(),
            state: LibraryState::Idle,
        }
    }

    /// Called by the shell whenever the active profile changes (including
    /// to `None`). `has_watchable_accounts` says whether the new profile
    /// has any linked accounts for `WatchGames` to ever report on.
    pub fn reset_for_profile(&mut self, has_watchable_accounts: Option<bool>) {
        self.games.clear();
        self.state = match has_watchable_accounts {
            None => LibraryState::Idle,
            Some(true) => LibraryState::Loading,
            // Nothing for `WatchGames` to ever report, so there's no event
            // to wait on — treat it as already loaded (empty).
            Some(false) => LibraryState::Loaded,
        };
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::Event(game_event::Event::Added(added)) => {
                self.state = LibraryState::Loaded;

                if let Some(game) = added.game {
                    upsert_game(&mut self.games, game);
                }
            }
            Message::Event(game_event::Event::Updated(updated)) => {
                self.state = LibraryState::Loaded;

                if let Some(game) = updated.game {
                    upsert_game(&mut self.games, game);
                }
            }
            Message::Event(game_event::Event::Removed(removed)) => {
                self.games.retain(|game| {
                    !(game.id == removed.game_id && game.storefront == removed.storefront)
                });
            }
            Message::Error(err) => {
                self.state = LibraryState::Failed(err.clone());
                eprintln!("failed to watch library: {err}");
            }
            Message::Noop => {}
        }

        iced::Task::none()
    }

    pub fn view(&self, width: f32, mode: PaneMode) -> Element<'_, Message> {
        match &self.state {
            LibraryState::Idle => {
                return container(InfoCard::new(
                    Kind::Hint,
                    "No active profile",
                    "Sign in to a profile to see its library.",
                ))
                .max_width(1150)
                .into();
            }
            LibraryState::Loading => {
                return container(InfoCard::new(
                    Kind::Hint,
                    "Loading library",
                    "Fetching games linked to this profile's storefronts.",
                ))
                .max_width(1150)
                .into();
            }
            LibraryState::Failed(err) => {
                return container(InfoCard::new(Kind::Error, "Couldn't load library", err.as_str()))
                    .max_width(1150)
                    .into();
            }
            LibraryState::Loaded => {}
        }

        if self.games.is_empty() {
            return container(InfoCard::new(
                Kind::Hint,
                "Nothing here yet",
                "Games linked to this profile's storefronts will show up here.",
            ))
            .max_width(1150)
            .into();
        }

        let columns = usize::from(mode == PaneMode::Single && width >= CONTENT_GRID_BREAKPOINT) + 1;
        let rows = self
            .games
            .iter()
            .fold(RowGroup::new().columns(columns), |rows, game| {
                let storefront = Storefront::try_from(game.storefront).unwrap_or_default();

                rows.add(
                    ActionRow::new(&game.title, ActionRowState::Ready(Message::Noop))
                        .description(storefront_label(storefront))
                        .icon(storefront_icon(storefront)),
                )
            });

        container(rows).max_width(1150).into()
    }
}

fn upsert_game(games: &mut Vec<Game>, game: Game) {
    if let Some(existing) = games
        .iter_mut()
        .find(|existing| existing.id == game.id && existing.storefront == game.storefront)
    {
        *existing = game;
    } else {
        games.push(game);
    }
}

fn storefront_label(storefront: Storefront) -> &'static str {
    match storefront {
        Storefront::Steam => "Steam",
        Storefront::EpicGames => "Epic Games Store",
        Storefront::Gog => "GOG",
        Storefront::AmazonGames => "Amazon Games",
        Storefront::EaApp => "EA App",
        Storefront::UbisoftConnect => "Ubisoft Connect",
        Storefront::BattleNet => "Battle.net",
        Storefront::Unspecified => "Unknown storefront",
    }
}

fn storefront_icon(storefront: Storefront) -> Icon {
    match storefront {
        Storefront::Steam => Icon::Computer,
        Storefront::EpicGames | Storefront::Gog | Storefront::AmazonGames => Icon::Disk,
        Storefront::EaApp | Storefront::UbisoftConnect | Storefront::BattleNet => Icon::Controller,
        Storefront::Unspecified => Icon::Warning,
    }
}
