mod app;
mod chrome;
mod classic;
mod context;
mod jobs;
mod layout;
#[cfg(feature = "fvs")]
mod next;
mod notifications;
mod onboarding;
mod settings;
mod store;
mod views;

pub(crate) use next_ui::{Icon, theme, widget};
pub(crate) use settings::Experience;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    iced::application(app::App::new, app::App::update, app::App::view)
        .title("Bottles Next")
        .theme(app::App::theme)
        .subscription(app::App::subscription)
        .style(|_, theme| theme::application(theme))
        .window(iced::window::Settings {
            size: iced::Size::new(1600.0, 1000.0),
            position: iced::window::Position::Centered,
            min_size: Some(iced::Size::new(720.0, 600.0)),
            decorations: false,
            transparent: true,
            exit_on_close_request: false,
            ..Default::default()
        })
        .run()
}
