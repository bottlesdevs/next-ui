use iced::{
    Alignment, Background, Border, Element, Event, Fill, Length, Padding, Rectangle, Shadow, Size,
    Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    widget::{Row as IcedRow, button, column, container, mouse_area, text},
};

pub struct ListRow<'a, Message, Renderer = iced::Renderer>
where
    Renderer: renderer::Renderer,
{
    body: Element<'a, Message, Theme, Renderer>,
    leading: Vec<Element<'a, Message, Theme, Renderer>>,
    trailing: Vec<Element<'a, Message, Theme, Renderer>>,
    content: Option<Element<'a, Message, Theme, Renderer>>,
    content_enabled: bool,
    on_press: Option<Message>,
    press_area: bool,
    raised: bool,
    enabled: bool,
    padding: Padding,
    height: Length,
    spacing: f32,
}

pub(crate) fn labels<'a, Message: 'a>(
    title: &'a str,
    description: &'a str,
) -> Element<'a, Message> {
    column![
        text(title).size(18).style(text::base),
        text(description)
            .size(16)
            .style(crate::components::style::muted_text),
    ]
    .spacing(4)
    .into()
}

impl<'a, Message, Renderer> ListRow<'a, Message, Renderer>
where
    Renderer: renderer::Renderer,
{
    pub fn new(body: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            body: body.into(),
            leading: Vec::new(),
            trailing: Vec::new(),
            content: None,
            content_enabled: true,
            on_press: None,
            press_area: false,
            raised: false,
            enabled: true,
            padding: Padding::from([18, 24]),
            height: Length::Shrink,
            spacing: 16.0,
        }
    }

    pub fn leading(mut self, control: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        self.leading.push(control.into());
        self
    }

    pub fn trailing(mut self, control: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        self.trailing.push(control.into());
        self
    }

    pub fn prepend_trailing(
        mut self,
        control: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        self.trailing.insert(0, control.into());
        self
    }

    pub fn content(mut self, content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn content_enabled(mut self, enabled: bool) -> Self {
        self.content_enabled = enabled;
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub(crate) fn on_press_area(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self.press_area = true;
        self
    }

    pub fn raised(mut self, raised: bool) -> Self {
        self.raised = raised;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}

impl<'a, Message: Clone + 'a, Renderer: renderer::Renderer + 'a>
    From<ListRow<'a, Message, Renderer>> for Element<'a, Message, Theme, Renderer>
{
    fn from(base: ListRow<'a, Message, Renderer>) -> Self {
        let header = IcedRow::new()
            .spacing(base.spacing)
            .align_y(Alignment::Center)
            .extend(base.leading)
            .push(container(base.body).width(Fill))
            .extend(base.trailing);

        let header: Element<'a, Message, Theme, Renderer> = match (base.on_press, base.press_area) {
            (Some(message), false) => button(header)
                .width(Fill)
                .height(base.height)
                .padding(base.padding)
                .on_press(message)
                .style(header_style)
                .into(),
            (Some(message), true) => mouse_area(
                container(header)
                    .width(Fill)
                    .height(base.height)
                    .padding(base.padding)
                    .align_y(Alignment::Center),
            )
            .on_press(message)
            .into(),
            (None, _) => container(header)
                .width(Fill)
                .height(base.height)
                .padding(base.padding)
                .align_y(Alignment::Center)
                .into(),
        };

        let mut contents = column![header].width(Fill);

        if let Some(content) = base.content {
            contents = contents.push(
                Surface::new(content)
                    .background(false)
                    .enabled(base.content_enabled),
            );
        }

        Surface::new(container(contents).width(Fill).clip(true))
            .raised(base.raised)
            .enabled(base.enabled)
            .into()
    }
}

fn header_style(theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: matches!(status, button::Status::Pressed).then_some(Background::Color(
            theme.extended_palette().background.stronger.color,
        )),
        text_color: theme.palette().text,
        border: Border::default().rounded(8),
        ..button::Style::default()
    }
}

struct Surface<'a, Message, Renderer>
where
    Renderer: renderer::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    background: bool,
    raised: bool,
    enabled: bool,
}

impl<'a, Message, Renderer> Surface<'a, Message, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            content: content.into(),
            background: true,
            raised: false,
            enabled: true,
        }
    }

    fn background(mut self, background: bool) -> Self {
        self.background = background;
        self
    }

    fn raised(mut self, raised: bool) -> Self {
        self.raised = raised;
        self
    }

    fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

#[derive(Default)]
struct State {
    hovered: bool,
}

impl<Message, Renderer> Widget<Message, Theme, Renderer> for Surface<'_, Message, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if self.enabled {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                operation,
            );
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if self.enabled {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }

        let state = tree.state.downcast_mut::<State>();
        let hovered = self.enabled && cursor.is_over(layout.bounds());

        if state.hovered != hovered {
            state.hovered = hovered;
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.enabled {
            self.content.as_widget().mouse_interaction(
                &tree.children[0],
                layout,
                cursor,
                viewport,
                renderer,
            )
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if self.background {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border::default().rounded(8),
                    shadow: Shadow::default(),
                    snap: true,
                },
                Background::Color(surface_color(
                    theme,
                    self.raised,
                    tree.state.downcast_ref::<State>().hovered,
                )),
            );
        }

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            layout,
            cursor,
            viewport,
        );

        if !self.enabled {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border::default().rounded(8),
                    shadow: Shadow::default(),
                    snap: true,
                },
                Background::Color(crate::theme::SCRIM),
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.enabled.then(|| {
            self.content.as_widget_mut().overlay(
                &mut tree.children[0],
                layout,
                renderer,
                viewport,
                translation,
            )
        })?
    }
}

impl<'a, Message: 'a, Renderer: renderer::Renderer + 'a> From<Surface<'a, Message, Renderer>>
    for Element<'a, Message, Theme, Renderer>
{
    fn from(surface: Surface<'a, Message, Renderer>) -> Self {
        Element::new(surface)
    }
}

fn surface_color(theme: &Theme, raised: bool, hovered: bool) -> iced::Color {
    if raised || hovered {
        theme.extended_palette().background.neutral.color
    } else {
        theme.extended_palette().background.weak.color
    }
}

#[cfg(test)]
mod tests {
    use crate::theme;

    use super::surface_color;

    #[test]
    fn hover_uses_the_raised_row_color() {
        let theme = theme::theme();

        assert_eq!(surface_color(&theme, false, false), theme::SURFACE);
        assert_eq!(surface_color(&theme, false, true), theme::BORDER);
        assert_eq!(surface_color(&theme, true, false), theme::BORDER);
    }
}
