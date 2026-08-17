use next_ui::{features::onboarding::State, theme};

fn main() -> iced::Result {
    iced::application(State::new, State::update, State::view)
        .title("Welcome to Bottles Next")
        .theme(State::theme)
        .style(|_, current_theme| theme::application(current_theme))
        .window_size((820.0, 640.0))
        .centered()
        .decorations(false)
        .transparent(true)
        .run()
}
