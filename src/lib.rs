mod app;
mod classic;
pub mod icons;
mod onboarding;
mod operation;
pub mod theme;
pub mod ui;
pub mod widgets;
mod window_modal;

pub(crate) use app::Experience;

pub fn run() -> iced::Result {
    iced::application(app::App::new, app::App::update, app::App::view)
        .title("Bottles Next")
        .theme(app::App::theme)
        .subscription(app::App::subscription)
        .style(|_, theme| theme::application(theme))
        .window_size((1600.0, 1000.0))
        .centered()
        .decorations(false)
        .transparent(true)
        .exit_on_close_request(false)
        .run()
}
