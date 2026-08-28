mod app;
mod classic;
pub mod icons;
mod onboarding;
mod operation;
pub mod theme;
pub mod ui;
pub mod widgets;

pub(crate) use app::Experience;

pub fn run() -> iced::Result {
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
