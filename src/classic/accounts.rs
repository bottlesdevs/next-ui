//! Storefront account linking backed directly by `next-core` providers.

use std::sync::{Arc, Mutex};

use bottles_core::{
    AccountLinkInteraction, PluginId, Profile, Profiles, StorefrontProvider,
    error::Error as CoreError,
};
use iced::futures::channel::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    icons::Icon,
    widgets::{
        button::{Button, ButtonKind},
        dialog::Dialog,
        info_row::InfoRow,
        list_row::ListRow,
        picker_row::PickerRow,
        popover::{Popover, PopoverItem},
        row_group::RowGroup,
        text_row::TextRow,
        title::Title,
    },
};

pub struct Context<'a> {
    pub active_profile: &'a Profile,
    pub profiles: &'a Profiles,
}

#[derive(Clone)]
pub struct LoginPrompt {
    url: String,
    instructions: String,
    response: Arc<Mutex<Option<oneshot::Sender<String>>>>,
}

impl LoginPrompt {
    fn new(url: String, instructions: String) -> (Self, oneshot::Receiver<String>) {
        let (response, answer) = oneshot::channel();
        (
            Self {
                url,
                instructions,
                response: Arc::new(Mutex::new(Some(response))),
            },
            answer,
        )
    }

    fn submit(&self, input: impl Into<String>) -> Result<(), &'static str> {
        let response = self
            .response
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
            .ok_or("account-link prompt was already answered")?;
        response
            .send(input.into())
            .map_err(|_| "account-link session is closed")
    }

    fn cancel(&self) {
        let _ = self
            .response
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
    }
}

struct LoginInteraction {
    prompts: mpsc::UnboundedSender<LoginPrompt>,
}

#[async_trait::async_trait]
impl AccountLinkInteraction for LoginInteraction {
    async fn request_input(&self, url: url::Url, instructions: String) -> Result<String, String> {
        let (prompt, answer) = LoginPrompt::new(url.to_string(), instructions);
        self.prompts
            .unbounded_send(prompt)
            .map_err(|_| "account-link UI is closed".to_owned())?;
        answer
            .await
            .map_err(|_| "account linking was cancelled".to_owned())
    }
}

struct LoginDialog {
    code_draft: String,
    prompt: LoginPrompt,
    submitting: bool,
    error: Option<String>,
}

impl LoginDialog {
    fn new(prompt: LoginPrompt) -> Self {
        Self {
            code_draft: String::new(),
            prompt,
            submitting: false,
            error: None,
        }
    }

    fn set_code(&mut self, code: String) {
        self.code_draft = code;
    }

    fn url(&self) -> &str {
        &self.prompt.url
    }

    fn submit(&mut self) {
        match self.prompt.submit(self.code_draft.trim()) {
            Ok(()) => self.submitting = true,
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn view(&self) -> iced::widget::Column<'_, Message> {
        use iced::widget::{column, container, row};

        let submit_label = if self.submitting {
            "Submitting…"
        } else {
            "Submit"
        };

        let mut content = column![
            container(Title::new("Sign in").subtitle(&self.prompt.instructions))
                .center_x(iced::Fill),
            RowGroup::new()
                .row(
                    ListRow::from(
                        InfoRow::new("Sign-in link (click to copy)")
                            .description(&self.prompt.url)
                            .icon(Icon::Controller),
                    )
                    .on_press(Message::CopyLoginUrl),
                )
                .row(action_button_row(
                    Icon::Arrow,
                    "Open in your browser",
                    "Sign in there, then paste the requested value below.",
                    "Open",
                    Message::OpenLoginUrl,
                ))
                .row(
                    TextRow::new("Authorization code", &self.code_draft)
                        .icon(Icon::Checkmark)
                        .on_input(Message::LoginCodeChanged)
                        .on_submit(Message::SubmitLogin),
                ),
        ]
        .spacing(18);
        if let Some(error) = &self.error {
            content = content.push(
                crate::widgets::info_card::InfoCard::new(
                    crate::widgets::info_card::Kind::Error,
                    "Could not answer the sign-in prompt",
                    error,
                )
                .width(iced::Fill),
            );
        }
        content = content.push(
            row![
                Button::new(submit_label)
                    .kind(ButtonKind::Primary)
                    .on_press_maybe((!self.submitting).then_some(Message::SubmitLogin)),
                Button::new("Cancel")
                    .kind(ButtonKind::Transparent)
                    .on_press(Message::DismissLogin),
            ]
            .spacing(12),
        );

        content
    }
}

impl Drop for LoginDialog {
    fn drop(&mut self) {
        self.prompt.cancel();
    }
}

#[derive(Clone)]
pub enum Message {
    UnlinkAccount(PluginId),
    BeginLogin(StorefrontProvider),
    LoginRequested {
        generation: u64,
        prompt: LoginPrompt,
    },
    LoginCodeChanged(String),
    OpenLoginUrl,
    CopyLoginUrl,
    SubmitLogin,
    DismissLogin,
    LinkFinished(Result<Profile, Arc<CoreError>>),
    ProfileUpdated(Result<Profile, Arc<CoreError>>),
    Noop,
}

#[derive(Default)]
pub struct State {
    link_generation: u64,
    link_cancellation: Option<CancellationToken>,
    login_dialog: Option<LoginDialog>,
    mutation_pending: bool,
    last_error: Option<String>,
}

impl State {
    /// Requests cancellation without dropping the task that drives the session.
    pub fn cancel_active_operation(&mut self) {
        self.login_dialog = None;
        if let Some(cancellation) = &self.link_cancellation {
            cancellation.cancel();
        }
    }

    pub fn has_active_operation(&self) -> bool {
        self.link_cancellation.is_some() || self.mutation_pending
    }

    pub fn update(&mut self, message: Message, ctx: &Context<'_>) -> iced::Task<Message> {
        match message {
            Message::UnlinkAccount(provider_id) => {
                if !self.mutation_pending && self.link_cancellation.is_none() {
                    let profiles = ctx.profiles.clone();
                    let profile_id = ctx.active_profile.id();
                    self.mutation_pending = true;
                    return iced::Task::perform(
                        async move {
                            profiles
                                .unlink_account(profile_id, provider_id)
                                .await
                                .map_err(Arc::new)
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::BeginLogin(provider) => {
                if self.link_cancellation.is_none() && !self.mutation_pending {
                    self.link_generation = self.link_generation.wrapping_add(1);
                    let generation = self.link_generation;
                    let (cancellation, task) = link_account(
                        ctx.profiles,
                        ctx.active_profile.id(),
                        provider.id,
                        generation,
                    );
                    self.link_cancellation = Some(cancellation);
                    self.last_error = None;
                    return task;
                }
            }
            Message::LoginRequested { generation, prompt } => {
                self.receive_prompt(generation, prompt);
            }
            Message::DismissLogin => self.cancel_active_operation(),
            Message::LoginCodeChanged(code) => {
                let Some(dialog) = &mut self.login_dialog else {
                    return iced::Task::none();
                };
                dialog.set_code(code);
            }
            Message::OpenLoginUrl => {
                if let Some(dialog) = &self.login_dialog {
                    open_url(dialog.url());
                }
            }
            Message::CopyLoginUrl => {
                return self
                    .login_dialog
                    .as_ref()
                    .map_or_else(iced::Task::none, |dialog| {
                        iced::clipboard::write(dialog.url().to_owned())
                    });
            }
            Message::SubmitLogin => {
                if let Some(dialog) = &mut self.login_dialog {
                    dialog.submit();
                }
            }
            Message::LinkFinished(result) => self.finish_link(result),
            Message::ProfileUpdated(result) => self.finish_mutation(result),
            Message::Noop => {}
        }

        iced::Task::none()
    }

    fn receive_prompt(&mut self, generation: u64, prompt: LoginPrompt) {
        if generation == self.link_generation
            && self
                .link_cancellation
                .as_ref()
                .is_some_and(|cancellation| !cancellation.is_cancelled())
        {
            self.login_dialog = Some(LoginDialog::new(prompt));
        } else {
            prompt.cancel();
        }
    }

    fn finish_link(&mut self, result: Result<Profile, Arc<CoreError>>) {
        self.login_dialog = None;
        self.link_cancellation = None;
        self.last_error = presentation_error(result);
    }

    fn finish_mutation(&mut self, result: Result<Profile, Arc<CoreError>>) {
        self.mutation_pending = false;
        self.last_error = presentation_error(result);
    }

    pub(super) fn dialog(&self) -> Option<Dialog<'_, Message>> {
        self.login_dialog
            .as_ref()
            .map(|dialog| Dialog::new(dialog.view(), Message::DismissLogin))
    }

    pub fn view_links<'a>(&'a self, ctx: &Context<'a>) -> iced::Element<'a, Message> {
        use iced::widget::{column, container};

        let active = ctx.active_profile;

        let mut accounts = RowGroup::new().title("Linked accounts");
        for account in active.accounts() {
            accounts = accounts.row(account_row(
                provider_icon(&account.provider),
                format!(
                    "{} on {}",
                    account.identity.display_name, account.provider.name
                ),
                "Connected",
                Message::UnlinkAccount(account.provider.id.clone()),
            ));
        }

        let link_trigger = PickerRow::new("Link a storefront account")
            .description("Choose the account provider to connect")
            .on_press(());
        let mut link_popover = Popover::new(link_trigger)
            .footer("Not listed, install a provider plugin", Message::Noop);

        for provider in ctx.profiles.account_providers() {
            if active
                .accounts()
                .iter()
                .any(|account| account.provider.id == provider.id)
            {
                continue;
            }

            link_popover = link_popover.add(
                PopoverItem::new(provider.name.clone())
                    .icon(provider_icon(&provider))
                    .action("Link", Message::BeginLogin(provider)),
            );
        }

        let mut content = column![accounts, container(link_popover).width(iced::Fill)].spacing(18);
        if let Some(error) = &self.last_error {
            content = content.push(
                crate::widgets::info_card::InfoCard::new(
                    crate::widgets::info_card::Kind::Error,
                    "Account update failed",
                    error,
                )
                .width(iced::Fill),
            );
        }
        content.into()
    }
}

pub fn account_row<'a>(
    icon: Icon,
    title: impl iced::widget::text::IntoFragment<'a>,
    description: &'a str,
    on_unlink: Message,
) -> ListRow<'a, Message> {
    action_button_row(icon, title, description, "Unlink", on_unlink)
}

pub fn action_button_row<'a, M: Clone + 'a>(
    icon: Icon,
    title: impl iced::widget::text::IntoFragment<'a>,
    description: &'a str,
    button_label: &'a str,
    on_press: M,
) -> ListRow<'a, M> {
    ListRow::from(InfoRow::new(title).description(description).icon(icon)).trailing(
        Button::new(button_label)
            .kind(ButtonKind::Surface)
            .on_press(on_press),
    )
}

fn link_account(
    profiles: &Profiles,
    profile_id: Uuid,
    provider_id: PluginId,
    generation: u64,
) -> (CancellationToken, iced::Task<Message>) {
    let (send_prompt, prompts) = mpsc::unbounded();
    let interaction = Arc::new(LoginInteraction {
        prompts: send_prompt,
    });
    let operation = profiles.link_account(profile_id, provider_id, interaction);
    let cancellation = operation.cancellation_token();
    let prompts = iced::Task::run(prompts, move |prompt| Message::LoginRequested {
        generation,
        prompt,
    });
    let operation = iced::Task::perform(operation, |result| {
        Message::LinkFinished(result.map_err(Arc::new))
    });
    (cancellation, iced::Task::batch([prompts, operation]))
}

fn presentation_error(result: Result<Profile, Arc<CoreError>>) -> Option<String> {
    result.err().and_then(|error| {
        (!matches!(error.as_ref(), CoreError::Cancelled)).then(|| error.to_string())
    })
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

fn provider_icon(provider: &StorefrontProvider) -> Icon {
    if provider.id.as_str() == "steam" {
        Icon::Computer
    } else {
        Icon::Controller
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_prompt_submission_is_one_shot() {
        futures_lite::future::block_on(async {
            let (prompt, answer) = LoginPrompt::new("https://example.com".into(), "Sign in".into());
            let mut dialog = LoginDialog::new(prompt);
            dialog.set_code("first".into());

            dialog.submit();
            assert!(dialog.submitting);
            dialog.submit();
            assert_eq!(
                dialog.error.as_deref(),
                Some("account-link prompt was already answered")
            );
            assert_eq!(answer.await.unwrap(), "first");
        });
    }

    #[test]
    fn cancellation_stays_active_until_the_terminal_event() {
        let mut state = State::default();
        let cancellation = CancellationToken::new();
        state.link_generation = 1;
        state.link_cancellation = Some(cancellation.clone());
        let (prompt, answer) = LoginPrompt::new("https://example.com".into(), "Sign in".into());
        state.login_dialog = Some(LoginDialog::new(prompt));

        state.cancel_active_operation();

        assert!(cancellation.is_cancelled());
        assert!(state.login_dialog.is_none());
        assert!(state.has_active_operation());
        assert!(futures_lite::future::block_on(answer).is_err());

        state.finish_link(Err(Arc::new(CoreError::Cancelled)));

        assert!(!state.has_active_operation());
        assert!(state.last_error.is_none());
    }

    #[test]
    fn stale_prompts_do_not_attach_to_a_later_link() {
        futures_lite::future::block_on(async {
            let mut state = State::default();
            state.link_generation = 2;
            state.link_cancellation = Some(CancellationToken::new());
            let (prompt, answer) = LoginPrompt::new("https://example.com".into(), "Sign in".into());

            state.receive_prompt(1, prompt);

            assert!(state.login_dialog.is_none());
            assert!(answer.await.is_err());
        });
    }
}
