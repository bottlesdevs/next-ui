use iced::{
    Alignment, Background, Border, Element, Event, Fill, Length, Point, Rectangle, Shadow, Size,
    Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    keyboard::{self, key},
    touch,
    widget::{column, container, row, scrollable, text},
};

use crate::icons::Icon;

use super::{
    draw_caret,
    focusable_action::Interaction,
    list_row::{self, ListRow},
    menu::{item as menu_item, row_content},
    spacing,
    text::TextExt as _,
};

const PANEL_PADDING: [f32; 2] = [spacing::MD, spacing::SM];
const RADIUS: f32 = 6.0;

/// An in-flow row that selects one value from a connected option panel.
pub struct SelectorRow<'a, T, Message> {
    title: &'a str,
    options: &'a [T],
    selected: Option<&'a T>,
    on_selected: Option<Box<dyn Fn(T) -> Message + 'a>>,
    placeholder: &'a str,
    icon: Option<Icon>,
}

impl<'a, T, Message> SelectorRow<'a, T, Message> {
    pub fn new(title: &'a str, options: &'a [T], selected: Option<&'a T>) -> Self {
        Self {
            title,
            options,
            selected,
            on_selected: None,
            placeholder: "",
            icon: None,
        }
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn on_selected(mut self, on_selected: impl Fn(T) -> Message + 'a) -> Self {
        self.on_selected = Some(Box::new(on_selected));
        self
    }
}

impl<'a, T, Message> From<SelectorRow<'a, T, Message>> for Element<'a, Message>
where
    T: Clone + PartialEq + ToString + 'a,
    Message: Clone + 'a,
{
    fn from(selector: SelectorRow<'a, T, Message>) -> Self {
        let selected = selector
            .selected
            .filter(|selected| selector.options.contains(selected));
        let value = selected.map_or_else(|| selector.placeholder.to_owned(), ToString::to_string);
        let enabled = selector.on_selected.is_some() && !selector.options.is_empty();
        let header = header(selector.title, value, selector.icon);
        let rows = selector.options.iter().map(|option| {
            let message = selector
                .on_selected
                .as_ref()
                .map(|on_selected| on_selected(option.clone()));

            menu_item(
                row_content(option.to_string().into(), None, None),
                message,
                selected == Some(option),
                || false,
            )
            .into()
        });
        let panel = container(
            scrollable(
                container(column(rows).width(Fill))
                    .width(Fill)
                    .padding(PANEL_PADDING),
            )
            .width(Fill)
            .style(crate::theme::scrollbar),
        )
        .width(Fill);

        Element::new(Selector {
            header,
            panel: panel.into(),
            enabled,
        })
    }
}

fn header<'a, Message: 'a>(
    title: &'a str,
    value: String,
    icon: Option<Icon>,
) -> Element<'a, Message> {
    let mut value_row = row![].spacing(spacing::SM).align_y(Alignment::Center);

    if let Some(icon) = icon {
        value_row = value_row.push(
            icon.view()
                .width(list_row::BODY_SIZE)
                .height(list_row::BODY_SIZE),
        );
    }

    value_row = value_row.push(text(value).size(list_row::BODY_SIZE).muted());

    ListRow::new(
        column![text(title).label().medium(), value_row]
            .width(Fill)
            .spacing(spacing::XS),
    )
    .into_disclosure_content()
}

struct Selector<'a, Message> {
    header: Element<'a, Message>,
    panel: Element<'a, Message>,
    enabled: bool,
}

#[derive(Debug, Default)]
struct State {
    open: bool,
    interaction: Interaction,
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for Selector<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.header), Tree::new(&self.panel)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.header, &self.panel]);

        if !self.enabled {
            tree.state.downcast_mut::<State>().open = false;
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(Length::Fill).height(Length::Shrink);
        let header = self
            .header
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &limits);
        let header_size = header.size();
        let panel = if tree.state.downcast_ref::<State>().open {
            self.panel
                .as_widget_mut()
                .layout(
                    &mut tree.children[1],
                    renderer,
                    &layout::Limits::new(
                        Size::new(header_size.width, 0.0),
                        Size::new(
                            header_size.width,
                            (limits.max().height - header_size.height).max(0.0),
                        ),
                    ),
                )
                .move_to(Point::new(0.0, header_size.height))
        } else {
            layout::Node::new(Size::ZERO)
        };
        let size = Size::new(header_size.width, header_size.height + panel.size().height);

        layout::Node::with_children(size, vec![header, panel])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let (header, panel) = layout_parts(layout);

        if self.enabled {
            operation.focusable(None, header.bounds(), &mut state.interaction);
        }

        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.header
                .as_widget_mut()
                .operate(&mut tree.children[0], header, renderer, operation);

            if state.open {
                self.panel.as_widget_mut().operate(
                    &mut tree.children[1],
                    panel,
                    renderer,
                    operation,
                );
            }
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let (header_layout, panel_layout) = layout_parts(layout);
        let state = tree.state.downcast_mut::<State>();

        if state.open {
            let mut messages = Vec::new();
            let mut panel_shell = Shell::new(&mut messages);
            self.panel.as_widget_mut().update(
                &mut tree.children[1],
                event,
                panel_layout,
                cursor,
                renderer,
                clipboard,
                &mut panel_shell,
                viewport,
            );

            if !panel_shell.is_empty() {
                state.open = false;
                panel_shell.invalidate_layout();
            }
            shell.merge(panel_shell, std::convert::identity);
        }

        self.header.as_widget_mut().update(
            &mut tree.children[0],
            event,
            header_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if state.interaction.update(
            event,
            header_layout
                .bounds()
                .intersection(viewport)
                .unwrap_or_default(),
            cursor,
            self.enabled,
            shell,
        ) {
            state.open = !state.open;
            shell.invalidate_layout();
            shell.request_redraw();
        } else if state.open && dismisses(event, cursor, layout.bounds()) {
            state.open = false;
            shell.invalidate_layout();
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        let (header, panel) = layout_parts(layout);
        let mut interaction = self.header.as_widget().mouse_interaction(
            &tree.children[0],
            header,
            cursor,
            viewport,
            renderer,
        );

        if state.open {
            interaction = interaction.max(self.panel.as_widget().mouse_interaction(
                &tree.children[1],
                panel,
                cursor,
                viewport,
                renderer,
            ));
        }

        if interaction == mouse::Interaction::default()
            && self.enabled
            && cursor.is_over(header.bounds())
        {
            mouse::Interaction::Pointer
        } else {
            interaction
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let (header, panel) = layout_parts(layout);
        let colors = crate::theme::colors(theme);

        if state.open {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: Border::default().rounded(RADIUS),
                    shadow: Shadow::default(),
                    snap: true,
                },
                Background::Color(colors.border),
            );
        }

        let mut header_state =
            state
                .interaction
                .state(self.enabled, false, header.bounds(), cursor);
        header_state.selected = state.open;
        let header_style = list_row::action_style(theme, header_state);
        renderer.fill_quad(
            renderer::Quad {
                bounds: header.bounds(),
                border: header_style.border,
                shadow: header_style.shadow,
                snap: header_style.snap,
            },
            header_style
                .background
                .unwrap_or(Background::Color(iced::Color::TRANSPARENT)),
        );
        self.header.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: if self.enabled {
                    colors.text
                } else {
                    colors.muted
                },
            },
            header,
            cursor,
            viewport,
        );
        draw_caret(
            renderer,
            theme,
            disclosure_bounds(header.bounds()),
            f32::from(state.open),
        );

        if state.open {
            self.panel.as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                style,
                panel,
                cursor,
                viewport,
            );
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        y: panel.bounds().y,
                        height: 1.0,
                        ..panel.bounds()
                    },
                    snap: true,
                    ..renderer::Quad::default()
                },
                Background::Color(colors.selection),
            );
        }
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        let state = tree.state.downcast_ref::<State>();
        let (_, panel) = layout_parts(layout);

        state.open.then(|| {
            self.panel.as_widget_mut().overlay(
                &mut tree.children[1],
                panel,
                renderer,
                viewport,
                translation,
            )
        })?
    }
}

fn layout_parts(layout: Layout<'_>) -> (Layout<'_>, Layout<'_>) {
    let mut children = layout.children();
    (
        children.next().expect("selector header"),
        children.next().expect("selector panel"),
    )
}

fn disclosure_bounds(header: Rectangle) -> Rectangle {
    const SIZE: f32 = 20.0;

    Rectangle {
        x: header.x + header.width - list_row::STANDARD_PADDING.right - SIZE,
        y: header.center_y() - SIZE / 2.0,
        width: SIZE,
        height: SIZE,
    }
}

fn dismisses(event: &Event, cursor: mouse::Cursor, bounds: Rectangle) -> bool {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(_)) => cursor
            .position()
            .is_some_and(|position| !bounds.contains(position)),
        Event::Touch(touch::Event::FingerPressed { position, .. }) => !bounds.contains(*position),
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(key::Named::Escape),
            repeat: false,
            ..
        }) => true,
        _ => false,
    }
}
