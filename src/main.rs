use iced::{
    Element, Fill, Task, Theme,
    widget::{column, container},
};
use next_ui::components::{header_bar, window_frame};

fn main() -> iced::Result {
    iced::application(|| (), update, view)
        .title("Bottles Next")
        .theme(theme)
        .style(|_, theme| next_ui::theme::application(theme))
        .window_size((1100.0, 720.0))
        .centered()
        .decorations(false)
        .transparent(true)
        .run()
}

fn theme(_: &()) -> Theme {
    next_ui::theme::theme()
}

#[derive(Debug, Clone)]
enum Message {
    Window(window_frame::Action),
}

fn update(_: &mut (), message: Message) -> Task<Message> {
    match message {
        Message::Window(action) => action.task(),
    }
}

fn view(_: &()) -> Element<'_, Message> {
    let header = header_bar::HeaderBar::new(Message::Window);

    window_frame::WindowFrame::new(
        container(column![header, container("").width(Fill).height(Fill)])
            .width(Fill)
            .height(Fill)
            .padding(1)
            .style(next_ui::theme::window)
            .clip(true),
        Message::Window,
    )
    .into()
}
