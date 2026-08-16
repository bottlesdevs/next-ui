//! Profile switcher + management screen, driven directly by
//! `bottles_core::profile::ProfileManager` (in-process, no RPC) for
//! everything except storefront login, which genuinely requires it: the
//! login challenge (a URL to open, a code to hand back) is served by a
//! Store plugin process (`next-plugin-egs`/`next-plugin-gog`) reached via
//! the plugin registry, and completed through `next-server`'s
//! `ProfileService`. `ProfileManager` deliberately doesn't own that
//! multi-process dance (see its module docs) — this is exactly the case
//! flagged as "prepare for RPC as a fallback."
//!
//! Only Epic Games and GOG are wired up (both plugins exist and expose a
//! browser/OAuth-redirect challenge with a URL to open and a code to paste
//! back). Every other storefront shows "Coming soon" instead of a Link
//! action, since no plugin exists for it yet.

use std::sync::Arc;

use bottles_core::profile::ProfileManager;
use iced::{
    Background, ContentFit, Element, Fill, Subscription, Task, Theme,
    futures::StreamExt as _,
    widget::{center, column, container, mouse_area, opaque, scrollable, stack, svg, text},
};
use next_proto::bottles::{
    common::v1::{AuthState, Storefront},
    profiles::v1::{LinkAccountRequest, SteamLink, UserProfile, profile_client::ProfileClient, profile_event},
    registry::v1::{ResolvePluginRequest, registry_client::RegistryClient},
    store::v1::{BeginLoginRequest, login_challenge, store_client::StoreClient},
};
use next_ui::{
    components::{
        button::{Button, ButtonKind},
        header_bar::HeaderBar,
        info_card::{self, InfoCard},
        list_row::ListRow,
        picker_row::PickerRow,
        popover::{Popover, PopoverItem},
        row_group::RowGroup,
        text::TextExt as _,
        text_row::TextRow,
        title::Title,
        window_frame,
    },
    icons::Icon,
    theme,
};

const STOREFRONTS: &[Storefront] = &[
    Storefront::Steam,
    Storefront::EpicGames,
    Storefront::Gog,
    Storefront::AmazonGames,
    Storefront::EaApp,
    Storefront::UbisoftConnect,
    Storefront::BattleNet,
];
const LOGIN_STOREFRONTS: &[Storefront] = &[Storefront::EpicGames, Storefront::Gog];
const REGISTRY_ENDPOINT: &str = "http://127.0.0.1:50250";
const SERVER_ENDPOINT: &str = "http://127.0.0.1:50052";

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Bottles Next profiles")
        .theme(App::theme)
        .subscription(App::subscription)
        .style(|_, current_theme| theme::application(current_theme))
        .window_size((900.0, 760.0))
        .centered()
        .decorations(false)
        .transparent(true)
        .run()
}

#[derive(Clone)]
struct ManagerHandle(Arc<ProfileManager>);

impl std::hash::Hash for ManagerHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.0) as usize).hash(state);
    }
}

struct App {
    manager: Option<ManagerHandle>,
    profiles: Vec<UserProfile>,
    active: Option<UserProfile>,
    name_draft: String,
    switcher_open: bool,
    link_open: bool,
    steam_open: bool,
    steam_candidates: Vec<bottles_core::steam::SteamUser>,
    login: Option<LoginChallenge>,
    error: Option<String>,
}

struct LoginChallenge {
    storefront: Storefront,
    challenge_id: String,
    url: String,
    code_draft: String,
    submitting: bool,
}

#[derive(Clone)]
enum Message {
    ManagerLoaded(Result<Arc<ProfileManager>, String>),
    ProfilesLoaded(Vec<UserProfile>),
    ProfileEvent(profile_event::Event),
    ToggleSwitcher,
    ToggleLink,
    ToggleSteam,
    SteamCandidatesDetected(Vec<bottles_core::steam::SteamUser>),
    Dismiss,
    ActivateProfile(String),
    CreateProfile,
    NameChanged(String),
    RenameSubmit,
    UnlinkAccount(i32),
    BeginLogin(Storefront),
    LoginChallengeReceived(Result<(Storefront, String, String), String>),
    LoginCodeChanged(String),
    OpenLoginUrl,
    CopyLoginUrl,
    DismissError,
    SubmitLoginCode,
    CancelLogin,
    UnlinkSteam,
    LinkSteam(String, String),
    DeleteProfile(String),
    ProfileUpdated(Result<UserProfile, String>),
    ProfileDeleted(Result<(), String>),
    Window(window_frame::Action),
    Noop,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let state = Self {
            manager: None,
            profiles: Vec::new(),
            active: None,
            name_draft: String::new(),
            switcher_open: false,
            link_open: false,
            steam_open: false,
            steam_candidates: Vec::new(),
            login: None,
            error: None,
        };
        let boot = Task::perform(
            async {
                ProfileManager::load()
                    .await
                    .map(Arc::new)
                    .map_err(|err| err.to_string())
            },
            Message::ManagerLoaded,
        );

        (state, boot)
    }

    fn theme(&self) -> Theme {
        theme::theme()
    }

    fn subscription(&self) -> Subscription<Message> {
        let Some(handle) = self.manager.clone() else {
            return Subscription::none();
        };

        Subscription::run_with(handle, profile_events)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ManagerLoaded(Ok(manager)) => {
                self.manager = Some(ManagerHandle(manager.clone()));

                return Task::perform(
                    async move {
                        let profiles = manager.list().await;

                        if profiles.is_empty() {
                            let profile = manager
                                .create("Player".into(), "person".into())
                                .await
                                .map_err(|err| err.to_string())?;
                            manager
                                .apply_activation(&profile.id, Default::default())
                                .await
                                .map_err(|err| err.to_string())?;
                            Ok(manager.list().await)
                        } else {
                            Ok(profiles)
                        }
                    },
                    |result: Result<Vec<UserProfile>, String>| match result {
                        Ok(profiles) => Message::ProfilesLoaded(profiles),
                        Err(_) => Message::ProfilesLoaded(Vec::new()),
                    },
                );
            }
            Message::ManagerLoaded(Err(err)) => self.error = Some(err),
            Message::ProfilesLoaded(profiles) => {
                self.active = profiles
                    .iter()
                    .find(|profile| Some(&profile.id) == self.active.as_ref().map(|p| &p.id))
                    .or_else(|| profiles.first())
                    .cloned();
                self.name_draft = self
                    .active
                    .as_ref()
                    .map(|profile| profile.name.clone())
                    .unwrap_or_default();
                self.profiles = profiles;
            }
            Message::ProfileEvent(profile_event::Event::Activated(profile)) => {
                upsert_profile(&mut self.profiles, profile.clone());
                self.name_draft = profile.name.clone();
                self.active = Some(profile);
            }
            Message::ProfileEvent(profile_event::Event::Updated(profile)) => {
                upsert_profile(&mut self.profiles, profile.clone());

                if self.active.as_ref().is_some_and(|active| active.id == profile.id) {
                    self.name_draft = profile.name.clone();
                    self.active = Some(profile);
                }
            }
            Message::ProfileEvent(profile_event::Event::DeletedProfileId(id)) => {
                self.profiles.retain(|profile| profile.id != id);
                if self.active.as_ref().is_some_and(|active| active.id == id) {
                    self.active = self.profiles.first().cloned();
                }
            }
            Message::ToggleSwitcher => {
                self.switcher_open = !self.switcher_open;
                self.link_open = false;
                self.steam_open = false;
            }
            Message::ToggleLink => {
                self.link_open = !self.link_open;
                self.switcher_open = false;
                self.steam_open = false;
            }
            Message::ToggleSteam => {
                self.steam_open = !self.steam_open;
                self.switcher_open = false;
                self.link_open = false;

                if self.steam_open {
                    return Task::perform(
                        async { detect_steam_users() },
                        Message::SteamCandidatesDetected,
                    );
                }
            }
            Message::SteamCandidatesDetected(candidates) => self.steam_candidates = candidates,
            Message::Dismiss => {
                self.switcher_open = false;
                self.link_open = false;
                self.steam_open = false;
            }
            Message::ActivateProfile(id) => {
                self.switcher_open = false;

                if let Some(handle) = self.manager.clone() {
                    return Task::perform(
                        async move {
                            handle
                                .0
                                .apply_activation(&id, Default::default())
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::CreateProfile => {
                self.switcher_open = false;

                if let Some(handle) = self.manager.clone() {
                    return Task::perform(
                        async move {
                            let profile = handle
                                .0
                                .create("New profile".into(), "person".into())
                                .await
                                .map_err(|err| err.to_string())?;
                            handle
                                .0
                                .apply_activation(&profile.id, Default::default())
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::NameChanged(name) => self.name_draft = name,
            Message::RenameSubmit => {
                if let (Some(handle), Some(active)) = (self.manager.clone(), self.active.clone())
                {
                    let name = self.name_draft.clone();
                    return Task::perform(
                        async move {
                            handle
                                .0
                                .rename(&active.id, name)
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::DeleteProfile(id) => {
                if let Some(handle) = self.manager.clone() {
                    return Task::perform(
                        async move { handle.0.delete(&id).await.map_err(|err| err.to_string()) },
                        Message::ProfileDeleted,
                    );
                }
            }
            Message::ProfileDeleted(Err(err)) => self.error = Some(err),
            // Success is a no-op here: `delete()` already broadcasts a
            // `DeletedProfileId` event, which the `ProfileEvent` arm below
            // uses to update `self.profiles`/`self.active`.
            Message::ProfileDeleted(Ok(())) => {}
            Message::UnlinkAccount(storefront) => {
                if let (Some(handle), Some(active)) = (self.manager.clone(), self.active.clone())
                {
                    return Task::perform(
                        async move {
                            handle
                                .0
                                .unlink_account(&active.id, storefront)
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::BeginLogin(storefront) => {
                self.link_open = false;

                if let Some(active) = self.active.clone() {
                    return Task::perform(
                        async move {
                            begin_login(active.id, storefront)
                                .await
                                .map(|(challenge_id, url)| (storefront, challenge_id, url))
                        },
                        Message::LoginChallengeReceived,
                    );
                }
            }
            Message::LoginChallengeReceived(Ok((storefront, challenge_id, url))) => {
                self.login = Some(LoginChallenge {
                    storefront,
                    challenge_id,
                    url,
                    code_draft: String::new(),
                    submitting: false,
                });
            }
            Message::LoginChallengeReceived(Err(err)) => self.error = Some(err),
            Message::LoginCodeChanged(code) => {
                if let Some(login) = &mut self.login {
                    login.code_draft = code;
                }
            }
            Message::OpenLoginUrl => {
                if let Some(login) = &self.login {
                    open_url(&login.url);
                }
            }
            Message::CopyLoginUrl => {
                if let Some(login) = &self.login {
                    return iced::clipboard::write(login.url.clone());
                }
            }
            Message::CancelLogin => self.login = None,
            Message::SubmitLoginCode => {
                if let (Some(active), Some(login)) = (self.active.clone(), &mut self.login) {
                    login.submitting = true;

                    let profile_id = active.id;
                    let challenge_id = login.challenge_id.clone();
                    let storefront = login.storefront;
                    let user_input = login.code_draft.clone();

                    return Task::perform(
                        async move {
                            complete_login(profile_id, challenge_id, storefront, user_input)
                                .await
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::UnlinkSteam => {
                if let (Some(handle), Some(active)) = (self.manager.clone(), self.active.clone())
                {
                    return Task::perform(
                        async move {
                            handle
                                .0
                                .unlink_steam(&active.id)
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::LinkSteam(steam_id64, account_name) => {
                self.steam_open = false;

                if let (Some(handle), Some(active)) = (self.manager.clone(), self.active.clone())
                {
                    return Task::perform(
                        async move {
                            handle
                                .0
                                .link_steam(
                                    &active.id,
                                    SteamLink {
                                        steam_id64,
                                        account_name,
                                    },
                                )
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::ProfileUpdated(Ok(profile)) => {
                upsert_profile(&mut self.profiles, profile.clone());
                self.name_draft = profile.name.clone();
                self.active = Some(profile);
                self.login = None;
                self.error = None;
            }
            Message::DismissError => self.error = None,
            Message::ProfileUpdated(Err(err)) => {
                self.error = Some(err);

                if let Some(login) = &mut self.login {
                    login.submitting = false;
                }
            }
            Message::Window(action) => return action.task(),
            Message::Noop => {}
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let active_label = self
            .active
            .as_ref()
            .map(|profile| profile.name.as_str())
            .unwrap_or("No profile");
        let switcher_trigger = Button::new(active_label)
            .icon(Icon::Person)
            .kind(ButtonKind::Surface)
            .on_press(Message::ToggleSwitcher);
        let mut switcher = Popover::new(switcher_trigger, self.switcher_open)
            .on_dismiss(Message::Dismiss)
            .footer("New profile", Message::CreateProfile);

        for profile in &self.profiles {
            let mut item = PopoverItem::new(&profile.name)
                .icon(Icon::Person)
                .on_select(Message::ActivateProfile(profile.id.clone()));

            if let Some(account) = profile.accounts.first() {
                item = item.subtitle(&account.account_display_name);
            }

            switcher = switcher.add(item);
        }

        let header = HeaderBar::new(Message::Window)
            .show_window_controls(true)
            .middle(Title::new("Profiles").subtitle("Manage local profiles and linked accounts"))
            .end(switcher);

        let content: Element<'_, Message> = if let Some(active) = &self.active {
            let mut accounts = RowGroup::new().title("Linked accounts");

            for account in &active.accounts {
                let storefront = Storefront::try_from(account.storefront).unwrap_or_default();
                accounts = accounts.add(account_row(
                    storefront_icon(storefront),
                    format!(
                        "{} on {}",
                        account.account_display_name,
                        storefront_label(storefront)
                    ),
                    auth_state_label(account.auth_state),
                    Message::UnlinkAccount(account.storefront),
                ));
            }

            let link_trigger = PickerRow::new("Link a storefront account")
                .description("Choose the account to connect")
                .on_press(Message::ToggleLink);
            let mut link_popover = Popover::new(link_trigger, self.link_open)
                .on_dismiss(Message::Dismiss)
                .footer("Not listed, install manually", Message::Noop);

            for storefront in STOREFRONTS {
                if active
                    .accounts
                    .iter()
                    .any(|account| account.storefront == *storefront as i32)
                {
                    continue;
                }

                let mut item = PopoverItem::new(storefront_label(*storefront))
                    .icon(storefront_icon(*storefront));

                item = if LOGIN_STOREFRONTS.contains(storefront) {
                    item.action("Link", Message::BeginLogin(*storefront))
                } else {
                    item.subtitle("Coming soon")
                };

                link_popover = link_popover.add(item);
            }

            let steam_row: Element<'_, Message> = if let Some(link) = &active.steam_link {
                account_row(
                    Icon::Computer,
                    &link.account_name,
                    "Linked Steam account",
                    Message::UnlinkSteam,
                )
                .into()
            } else {
                let steam_trigger = PickerRow::new("Link Steam account")
                    .description("Detected from your local Steam installation")
                    .on_press(Message::ToggleSteam);
                let mut steam_popover =
                    Popover::new(steam_trigger, self.steam_open).on_dismiss(Message::Dismiss);

                for user in &self.steam_candidates {
                    steam_popover = steam_popover.add(
                        PopoverItem::new(&user.account_name).icon(Icon::Computer).action(
                            "Link",
                            Message::LinkSteam(user.steam_id64.clone(), user.account_name.clone()),
                        ),
                    );
                }

                steam_popover.into()
            };

            column![
                RowGroup::new()
                    .title("Profile")
                    .add(
                        TextRow::new("Profile name", &self.name_draft)
                            .icon(Icon::Person)
                            .on_input(Message::NameChanged)
                            .on_submit(Message::RenameSubmit),
                    )
                    .add(action_button_row(
                        Icon::Cross,
                        "Delete profile",
                        "Removes this profile and its linked accounts from this device",
                        "Delete",
                        Message::DeleteProfile(active.id.clone()),
                    )),
                accounts,
                container(link_popover).width(Fill),
                container(steam_row).width(Fill),
            ]
            .spacing(18)
            .into()
        } else {
            column![].into()
        };

        let mut page = column![].spacing(18);

        if let Some(error) = &self.error {
            page = page.push(error_banner(error));
        }

        let body = scroll_panel(page.push(content));
        let window = window_frame::WindowFrame::new(
            column![header, body].width(Fill).height(Fill),
            Message::Window,
        );

        if let Some(login) = &self.login {
            modal(window, login_dialog(login), Message::CancelLogin)
        } else {
            window.into()
        }
    }
}

fn login_dialog(login: &LoginChallenge) -> Element<'_, Message> {
    let submit_label = if login.submitting { "Submitting…" } else { "Submit" };

    container(
        column![
            Title::new("Sign in").subtitle(storefront_label(login.storefront)),
            RowGroup::new()
                .add(
                    label_row(storefront_icon(login.storefront), "Sign-in link (click to copy)", &login.url)
                        .on_press(Message::CopyLoginUrl),
                )
                .add(action_button_row(
                    Icon::Arrow,
                    "Open in your browser",
                    "Sign in there, then paste the code you're given below.",
                    "Open",
                    Message::OpenLoginUrl,
                ))
                .add(
                    TextRow::new("Authorization code", &login.code_draft)
                        .icon(Icon::Checkmark)
                        .on_input(Message::LoginCodeChanged)
                        .on_submit(Message::SubmitLoginCode),
                ),
            iced::widget::row![
                Button::new(submit_label)
                    .kind(ButtonKind::Primary)
                    .on_press_maybe((!login.submitting).then_some(Message::SubmitLoginCode)),
                Button::new("Cancel")
                    .kind(ButtonKind::Transparent)
                    .on_press(Message::CancelLogin),
            ]
            .spacing(12),
        ]
        .spacing(18),
    )
    .width(560)
    .padding(24)
    .style(theme::panel)
    .into()
}

/// A centered dialog over a dimmed, click-to-dismiss backdrop — `content`
/// is wrapped in its own `opaque` so a click on the dialog itself doesn't
/// also fall through to the backdrop's dismiss handler.
fn modal<'a>(
    base: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    on_dismiss: Message,
) -> Element<'a, Message> {
    stack![
        base.into(),
        opaque(
            mouse_area(center(opaque(content)).style(|_theme| container::Style {
                background: Some(Background::Color(theme::SCRIM)),
                ..container::Style::default()
            }))
            .on_press(on_dismiss)
        ),
    ]
    .into()
}

fn scroll_panel<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    let content = container(content).width(Fill).padding(24).center_x(Fill);

    container(
        scrollable(content)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4).margin(12),
            ))
            .style(theme::scrollbar)
            .width(Fill)
            .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .into()
}

fn profile_events(
    handle: &ManagerHandle,
) -> std::pin::Pin<Box<dyn iced::futures::Stream<Item = Message> + Send>> {
    let manager = handle.0.clone();

    Box::pin(
        manager
            .watch()
            .filter_map(|event| async move { event.event.map(Message::ProfileEvent) }),
    )
}

/// A linked-account row where only the trailing "Unlink" button is
/// clickable — unlike `ActionRow`, whose entire row acts as a single press
/// target, which would make a destructive action a one-tap accident.
fn account_row<'a>(
    icon: Icon,
    title: impl text::IntoFragment<'a>,
    description: &'a str,
    on_unlink: Message,
) -> ListRow<'a, Message> {
    action_button_row(icon, title, description, "Unlink", on_unlink)
}

fn error_banner(message: &str) -> Element<'_, Message> {
    column![
        InfoCard::new(info_card::Kind::Error, "Something went wrong", message),
        Button::new("Dismiss")
            .kind(ButtonKind::Transparent)
            .on_press(Message::DismissError),
    ]
    .spacing(6)
    .into()
}

/// A plain, non-interactive info row — no trailing button, so a long
/// description (e.g. a full URL, which can't word-wrap) never has
/// anything next to it to overlap.
fn label_row<'a>(
    icon: Icon,
    title: &'a str,
    description: impl text::IntoFragment<'a>,
) -> ListRow<'a, Message> {
    let labels = column![text(title).label(), text(description).detail().muted()].spacing(6);

    ListRow::new(labels).leading(
        svg(icon.handle())
            .width(24)
            .height(24)
            .content_fit(ContentFit::Contain),
    )
}

fn action_button_row<'a>(
    icon: Icon,
    title: impl text::IntoFragment<'a>,
    description: &'a str,
    button_label: &'a str,
    on_press: Message,
) -> ListRow<'a, Message> {
    let labels = column![text(title).label(), text(description).detail().muted()].spacing(6);

    ListRow::new(labels)
        .leading(
            svg(icon.handle())
                .width(24)
                .height(24)
                .content_fit(ContentFit::Contain),
        )
        .trailing(
            Button::new(button_label)
                .kind(ButtonKind::Surface)
                .on_press(on_press),
        )
}

fn upsert_profile(profiles: &mut Vec<UserProfile>, profile: UserProfile) {
    if let Some(existing) = profiles.iter_mut().find(|existing| existing.id == profile.id) {
        *existing = profile;
    } else {
        profiles.push(profile);
    }
}

/// Resolves `storefront` to its owning plugin via the registry, dials it,
/// and starts an interactive login. Returns the challenge id (needed to
/// complete the login later) and the URL the user needs to open — this
/// mirrors `next-server`'s own `store_client_for` + `BeginLogin` dance
/// (`crates/next-server/src/profile.rs`), since that resolution isn't
/// something `next-server`'s `Profile` service proxies for callers.
async fn begin_login(profile_id: String, storefront: Storefront) -> Result<(String, String), String> {
    let mut registry = RegistryClient::connect(REGISTRY_ENDPOINT)
        .await
        .map_err(|err| format!("plugin registry unavailable: {err}"))?;
    let resolved = registry
        .resolve_plugin(ResolvePluginRequest {
            storefront: storefront as i32,
        })
        .await
        .map_err(|err| err.to_string())?
        .into_inner();
    let endpoint = resolved
        .endpoint
        .ok_or_else(|| format!("no {} plugin is running", storefront_label(storefront)))?;
    let mut store = StoreClient::connect(endpoint)
        .await
        .map_err(|err| err.to_string())?;
    let challenge = store
        .begin_login(BeginLoginRequest {
            profile_id,
            storefront: storefront as i32,
        })
        .await
        .map_err(|err| err.to_string())?
        .into_inner();

    if let Some(error) = challenge.error {
        return Err(error);
    }

    let url = match challenge.kind {
        Some(login_challenge::Kind::BrowserRedirect(challenge)) => challenge.url,
        Some(login_challenge::Kind::OauthRedirect(challenge)) => challenge.auth_url,
        _ => return Err("this storefront's login flow isn't supported yet".into()),
    };

    Ok((challenge.challenge_id, url))
}

/// Completes the login started by [`begin_login`] against `next-server`'s
/// `Profile` service, which resolves the same plugin again to exchange
/// `user_input` and persists the resulting linked account.
async fn complete_login(
    profile_id: String,
    challenge_id: String,
    storefront: Storefront,
    user_input: String,
) -> Result<UserProfile, String> {
    let mut client = ProfileClient::connect(SERVER_ENDPOINT)
        .await
        .map_err(|err| format!("next-server unavailable: {err}"))?;

    client
        .link_account(LinkAccountRequest {
            profile_id,
            challenge_id,
            storefront: storefront as i32,
            user_input,
        })
        .await
        .map(|response| response.into_inner())
        .map_err(|err| err.to_string())
}

fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

fn detect_steam_users() -> Vec<bottles_core::steam::SteamUser> {
    bottles_core::steam::loginusers_vdf_path()
        .and_then(|path| bottles_core::steam::parse_loginusers(&path).ok())
        .unwrap_or_default()
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

fn auth_state_label(state: i32) -> &'static str {
    match AuthState::try_from(state).unwrap_or_default() {
        AuthState::Active => "Connected",
        AuthState::Stale => "Needs re-authentication",
        AuthState::Inactive => "Signed out",
        AuthState::Unspecified => "Unknown",
    }
}
