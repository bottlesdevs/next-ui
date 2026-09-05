use iced::{
    Event, Theme,
    advanced::{
        Layout, Widget, mouse,
        widget::{Id, Operation, Tree, operation},
    },
    touch,
};

pub(crate) fn descendant_is_focused<Message>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
) -> bool {
    matches!(
        run_operation(
            content,
            tree,
            layout,
            renderer,
            operation::focusable::count(),
        ),
        operation::Outcome::Some(operation::focusable::Count {
            focused: Some(_),
            ..
        })
    )
}

pub(crate) fn focus_first_descendant<Message>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
) {
    unfocus_descendants(content, tree, layout, renderer);
    move_descendant_focus(content, tree, layout, renderer, false);
}

pub(crate) fn focus_descendant<Message>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
    id: Id,
) {
    run_operation(
        content,
        tree,
        layout,
        renderer,
        operation::focusable::focus::<()>(id),
    );
}

pub(crate) fn unfocus_descendants<Message>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
) {
    run_operation(
        content,
        tree,
        layout,
        renderer,
        operation::focusable::unfocus::<()>(),
    );
}

pub(crate) fn cycle_descendant_focus<Message>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
    previous: bool,
) {
    move_descendant_focus(content, tree, layout, renderer, previous);

    if !descendant_is_focused(content, tree, layout, renderer) {
        move_descendant_focus(content, tree, layout, renderer, previous);
    }
}

fn move_descendant_focus<Message>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
    previous: bool,
) {
    if previous {
        run_operation(
            content,
            tree,
            layout,
            renderer,
            operation::focusable::focus_previous::<()>(),
        );
    } else {
        run_operation(
            content,
            tree,
            layout,
            renderer,
            operation::focusable::focus_next::<()>(),
        );
    }
}

fn run_operation<Message, Output: 'static>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
    operation: impl Operation<Output> + 'static,
) -> operation::Outcome<Output> {
    let mut operation: Box<dyn Operation<Output>> = Box::new(operation);

    loop {
        content.operate(
            tree,
            layout,
            renderer,
            &mut operation::black_box(operation.as_mut()),
        );

        match operation.finish() {
            operation::Outcome::Chain(next) => operation = next,
            outcome => return outcome,
        }
    }
}

pub(crate) fn event_cursor(event: &Event, cursor: mouse::Cursor) -> mouse::Cursor {
    match event {
        Event::Touch(
            touch::Event::FingerPressed { position, .. }
            | touch::Event::FingerMoved { position, .. }
            | touch::Event::FingerLifted { position, .. }
            | touch::Event::FingerLost { position, .. },
        ) => mouse::Cursor::Available(*position),
        _ => cursor,
    }
}
