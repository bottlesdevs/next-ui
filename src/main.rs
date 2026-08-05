use iced::{
    Element, Fill, Subscription, Task, Theme,
    keyboard::{self, key},
    widget::{column, container},
};
use next_ui::components::{header_bar, window_frame};

fn main() -> iced::Result {
    iced::application(|| (), update, view)
        .title("Bottles Next")
        .theme(theme)
        .subscription(subscription)
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
    MoveFocus(bool),
}

fn update(_: &mut (), message: Message) -> Task<Message> {
    match message {
        Message::Window(action) => action.task(),
        Message::MoveFocus(previous) => {
            if previous {
                iced::widget::operation::focus_previous()
            } else {
                iced::widget::operation::focus_next()
            }
        }
    }
}

fn subscription(_: &()) -> Subscription<Message> {
    keyboard::listen().filter_map(|event| match event {
        keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(key::Named::Tab),
            modifiers,
            repeat: false,
            ..
        } => Some(Message::MoveFocus(modifiers.shift())),
        _ => None,
    })
}

fn view(_: &()) -> Element<'_, Message> {
    let header = header_bar::HeaderBar::new(Message::Window);

    window_frame::WindowFrame::new(
        column![header, container("").width(Fill).height(Fill)],
        Message::Window,
    )
    .into()
}
