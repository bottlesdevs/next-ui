use std::sync::{Arc, Mutex};

use bottles_core::{
    AccountLinkInteraction, PluginId, Profile, Profiles, StorefrontProvider,
    error::Error as CoreError,
};
use iced::{
    Task,
    futures::channel::{mpsc, oneshot},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

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

pub struct LoginDialog {
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

    pub fn code(&self) -> &str {
        &self.code_draft
    }

    pub fn url(&self) -> &str {
        &self.prompt.url
    }

    pub fn instructions(&self) -> &str {
        &self.prompt.instructions
    }

    pub fn is_submitting(&self) -> bool {
        self.submitting
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn set_code(&mut self, code: String) {
        self.code_draft = code;
    }

    fn submit(&mut self) {
        match self.prompt.submit(self.code_draft.trim()) {
            Ok(()) => self.submitting = true,
            Err(error) => self.error = Some(error.to_string()),
        }
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
    InstallProviderPlugin,
    UrlOpened(Result<(), Arc<std::io::Error>>),
    LinkFinished(Result<Profile, Arc<CoreError>>),
    ProfileUpdated(Result<Profile, Arc<CoreError>>),
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
    pub fn dialog(&self) -> Option<&LoginDialog> {
        self.login_dialog.as_ref()
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn cancel_active_operation(&mut self) {
        self.login_dialog = None;
        if let Some(cancellation) = &self.link_cancellation {
            cancellation.cancel();
        }
    }

    pub fn has_active_operation(&self) -> bool {
        self.link_cancellation.is_some() || self.mutation_pending
    }

    pub fn update(
        &mut self,
        message: Message,
        active_profile: &Profile,
        profiles: &Profiles,
    ) -> Task<Message> {
        match message {
            Message::UnlinkAccount(provider_id) => {
                if !self.mutation_pending && self.link_cancellation.is_none() {
                    let profiles = profiles.clone();
                    let profile_id = active_profile.id();
                    self.mutation_pending = true;
                    return Task::perform(
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
                    let (cancellation, task) =
                        link_account(profiles, active_profile.id(), provider.id, generation);
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
                    return Task::none();
                };
                dialog.set_code(code);
            }
            Message::OpenLoginUrl => {
                let Some(dialog) = &mut self.login_dialog else {
                    return Task::none();
                };
                let url = dialog.url().to_owned();
                dialog.error = None;
                self.last_error = None;

                return Task::perform(
                    async move { open_url(&url).map_err(Arc::new) },
                    Message::UrlOpened,
                );
            }
            Message::CopyLoginUrl => {
                return self
                    .login_dialog
                    .as_ref()
                    .map_or_else(Task::none, |dialog| {
                        iced::clipboard::write(dialog.url().to_owned())
                    });
            }
            Message::SubmitLogin => {
                if let Some(dialog) = &mut self.login_dialog {
                    dialog.submit();
                }
            }
            Message::InstallProviderPlugin => {}
            Message::UrlOpened(result) => self.finish_url_open(result),
            Message::LinkFinished(result) => self.finish_link(result),
            Message::ProfileUpdated(result) => self.finish_mutation(result),
        }

        Task::none()
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

    fn finish_url_open(&mut self, result: Result<(), Arc<std::io::Error>>) {
        if let Err(error) = result {
            let error = format!("Could not open the sign-in link: {error}");
            if let Some(dialog) = &mut self.login_dialog {
                dialog.error = Some(error);
            } else {
                self.last_error = Some(error);
            }
        }
    }
}

fn link_account(
    profiles: &Profiles,
    profile_id: Uuid,
    provider_id: PluginId,
    generation: u64,
) -> (CancellationToken, Task<Message>) {
    let (send_prompt, prompts) = mpsc::unbounded();
    let interaction = Arc::new(LoginInteraction {
        prompts: send_prompt,
    });
    let operation = profiles.link_account(profile_id, provider_id, interaction);
    let cancellation = operation.cancellation_token();
    let prompts = Task::run(prompts, move |prompt| Message::LoginRequested {
        generation,
        prompt,
    });
    let operation = Task::perform(operation, |result| {
        Message::LinkFinished(result.map_err(Arc::new))
    });
    (cancellation, Task::batch([prompts, operation]))
}

fn presentation_error(result: Result<Profile, Arc<CoreError>>) -> Option<String> {
    result.err().and_then(|error| {
        (!matches!(error.as_ref(), CoreError::Cancelled)).then(|| error.to_string())
    })
}

fn open_url(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(not(any(unix, target_os = "windows")))]
    let result = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "opening URLs is not supported on this platform",
    ));

    result.map(drop)
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
        let cancellation = CancellationToken::new();
        let (prompt, answer) = LoginPrompt::new("https://example.com".into(), "Sign in".into());
        let mut state = State {
            link_generation: 1,
            link_cancellation: Some(cancellation.clone()),
            login_dialog: Some(LoginDialog::new(prompt)),
            ..State::default()
        };

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
            let mut state = State {
                link_generation: 2,
                link_cancellation: Some(CancellationToken::new()),
                ..State::default()
            };
            let (prompt, answer) = LoginPrompt::new("https://example.com".into(), "Sign in".into());

            state.receive_prompt(1, prompt);

            assert!(state.login_dialog.is_none());
            assert!(answer.await.is_err());
        });
    }
}
