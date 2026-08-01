use iced::{
    Alignment, Background, Border, Element, Event, Fill, Length, Rectangle, Shadow, Size, Theme,
    Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, operation},
    },
    touch,
    widget::{Row as IcedRow, button, column, container, text},
};

use super::{
    pressable::{Pressable, SharedFlag, Status as PressableStatus},
    spacing,
    text::TextExt as _,
};

pub struct ListRow<'a, Message> {
    body: Element<'a, Message>,
    leading: Vec<Element<'a, Message>>,
    trailing: Vec<Element<'a, Message>>,
    enabled: bool,
    on_press: Option<Message>,
    on_activate: Option<SharedFlag>,
    raised_when: Option<SharedFlag>,
    hover_tone: HoverTone,
    focus_content_on_press: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum HoverTone {
    #[default]
    Default,
    Strong,
}

pub(crate) fn labels<'a, Message: 'a>(
    title: &'a str,
    description: &'a str,
) -> Element<'a, Message> {
    column![text(title).label(), text(description).detail().muted(),]
        .spacing(spacing::XS)
        .into()
}

impl<'a, Message> ListRow<'a, Message> {
    pub fn new(body: impl Into<Element<'a, Message>>) -> Self {
        Self {
            body: body.into(),
            leading: Vec::new(),
            trailing: Vec::new(),
            enabled: true,
            on_press: None,
            on_activate: None,
            raised_when: None,
            hover_tone: HoverTone::Default,
            focus_content_on_press: false,
        }
    }

    pub fn leading(mut self, control: impl Into<Element<'a, Message>>) -> Self {
        self.leading.push(control.into());
        self
    }

    pub fn trailing(mut self, control: impl Into<Element<'a, Message>>) -> Self {
        self.trailing.push(control.into());
        self
    }

    pub fn prepend_trailing(mut self, control: impl Into<Element<'a, Message>>) -> Self {
        self.trailing.insert(0, control.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub(crate) fn parent_enabled(mut self, enabled: bool) -> Self {
        self.enabled &= enabled;
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self.on_activate = None;
        self
    }

    pub(crate) fn on_activate(mut self, activation: SharedFlag) -> Self {
        self.on_press = None;
        self.on_activate = Some(activation);
        self
    }

    pub(crate) fn raised_when(mut self, raised: SharedFlag) -> Self {
        self.raised_when = Some(raised);
        self
    }

    pub(crate) fn set_hover_tone(&mut self, hover_tone: HoverTone) {
        self.hover_tone = hover_tone;
    }

    pub(crate) fn focus_content_on_press(mut self) -> Self {
        self.focus_content_on_press = true;
        self
    }
}

impl<'a, Message: Clone + 'a> From<ListRow<'a, Message>> for Element<'a, Message> {
    fn from(base: ListRow<'a, Message>) -> Self {
        let header = IcedRow::new()
            .spacing(spacing::MD)
            .align_y(Alignment::Center)
            .extend(base.leading)
            .push(container(base.body).width(Fill))
            .extend(base.trailing);

        let header: Element<'a, Message> = match (base.on_press, base.on_activate) {
            (Some(message), _) => Pressable::new(header)
                .width(Fill)
                .padding([spacing::MD, spacing::LG])
                .on_press(message)
                .style(header_style)
                .into(),
            (None, Some(activation)) => Pressable::new(header)
                .width(Fill)
                .padding([spacing::MD, spacing::LG])
                .on_activate(activation)
                .style(header_style)
                .into(),
            (None, None) => container(header)
                .width(Fill)
                .padding([spacing::MD, spacing::LG])
                .align_y(Alignment::Center)
                .into(),
        };

        Surface::new(container(header).width(Fill).clip(true))
            .raised_when(base.raised_when)
            .hover_tone(base.hover_tone)
            .enabled(base.enabled)
            .focus_content_on_press(base.focus_content_on_press)
            .into()
    }
}

fn header_style(theme: &Theme, status: PressableStatus) -> button::Style {
    button::Style {
        background: matches!(status, PressableStatus::Pressed).then_some(Background::Color(
            theme.extended_palette().background.stronger.color,
        )),
        text_color: theme.palette().text,
        border: Border::default().rounded(8),
        ..button::Style::default()
    }
}

struct Surface<'a, Message> {
    content: Element<'a, Message>,
    raised_when: Option<SharedFlag>,
    hover_tone: HoverTone,
    enabled: bool,
    focus_content_on_press: bool,
}

impl<'a, Message> Surface<'a, Message> {
    fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            raised_when: None,
            hover_tone: HoverTone::Default,
            enabled: true,
            focus_content_on_press: false,
        }
    }

    fn raised_when(mut self, raised: Option<SharedFlag>) -> Self {
        self.raised_when = raised;
        self
    }

    fn hover_tone(mut self, hover_tone: HoverTone) -> Self {
        self.hover_tone = hover_tone;
        self
    }

    fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    fn focus_content_on_press(mut self, focus_content_on_press: bool) -> Self {
        self.focus_content_on_press = focus_content_on_press;
        self
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Surface<'_, Message> {
    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        iced::advanced::widget::tree::Tag::of::<SurfaceState>()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(SurfaceState::default())
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
        renderer: &iced::Renderer,
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
        renderer: &iced::Renderer,
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
        renderer: &iced::Renderer,
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

            if self.focus_content_on_press
                && cursor.is_over(layout.bounds())
                && matches!(
                    event,
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                        | Event::Touch(touch::Event::FingerPressed { .. })
                )
            {
                self.content.as_widget_mut().operate(
                    &mut tree.children[0],
                    layout,
                    renderer,
                    &mut FocusFirst(false),
                );
                shell.request_redraw();
            }
        }

        let hovered = self.enabled && cursor.is_over(layout.bounds());

        if matches!(
            event,
            Event::Window(iced::window::Event::RedrawRequested(_))
        ) {
            let mut count = operation::focusable::count();
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                &mut operation::black_box(&mut count),
            );
            let focused = matches!(
                Operation::finish(&count),
                operation::Outcome::Some(count) if count.focused.is_some()
            );
            let state = tree.state.downcast_mut::<SurfaceState>();
            state.hovered = hovered;
            state.focused = focused;
        } else if tree.state.downcast_ref::<SurfaceState>().hovered != hovered {
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
        renderer: &mut iced::Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let state = tree.state.downcast_ref::<SurfaceState>();
        let hovered = state.hovered || self.enabled && cursor.is_over(bounds);

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border::default().rounded(8),
                shadow: Shadow::default(),
                snap: true,
            },
            Background::Color(surface_color(
                theme,
                self.raised_when.as_ref().is_some_and(SharedFlag::get) || state.focused,
                hovered,
                self.hover_tone,
            )),
        );

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
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
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

#[derive(Debug, Default)]
struct SurfaceState {
    hovered: bool,
    focused: bool,
}

struct FocusFirst(bool);

impl Operation for FocusFirst {
    fn focusable(
        &mut self,
        _id: Option<&iced::widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn operation::Focusable,
    ) {
        if !self.0 {
            state.focus();
            self.0 = true;
        }
    }

    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }
}

impl<'a, Message: 'a> From<Surface<'a, Message>> for Element<'a, Message> {
    fn from(surface: Surface<'a, Message>) -> Self {
        Element::new(surface)
    }
}

fn surface_color(theme: &Theme, raised: bool, hovered: bool, hover_tone: HoverTone) -> iced::Color {
    if hovered && hover_tone == HoverTone::Strong {
        crate::theme::ROW_HOVER_STRONG
    } else if raised || hovered {
        theme.extended_palette().background.neutral.color
    } else {
        theme.extended_palette().background.weak.color
    }
}
