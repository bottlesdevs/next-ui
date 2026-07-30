use iced::{
    Element, Fill, Task, Theme,
    widget::{Space, button, column, container, mouse_area, row},
    window,
};

fn main() -> iced::Result {
    iced::application(|| (), update, view)
        .title("Bottles Next")
        .theme(theme)
        .window_size((1100.0, 720.0))
        .centered()
        .decorations(false)
        .run()
}

fn theme(_: &()) -> Theme {
    Theme::Dracula
}

#[derive(Debug, Clone)]
enum Message {
    Drag,
    Close,
}

fn update(_: &mut (), message: Message) -> Task<Message> {
    match message {
        Message::Drag => window::latest().and_then(window::drag),
        Message::Close => iced::exit(),
    }
}

fn view(_: &()) -> Element<'_, Message> {
    let title_bar = mouse_area(row![
        Space::new().width(Fill),
        button("×").style(button::text).on_press(Message::Close),
    ])
    .on_press(Message::Drag);

    column![title_bar, container("").width(Fill).height(Fill)]
        .padding(16)
        .into()
}
