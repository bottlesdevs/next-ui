use next_ui::app::App;
use next_ui::theme;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Bottles Next")
        .theme(App::theme)
        .subscription(App::subscription)
        .style(|_, theme| theme::application(theme))
        .window_size((1600.0, 1000.0))
        .centered()
        .decorations(false)
        .transparent(true)
        .run()
}
