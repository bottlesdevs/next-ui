use iced::{Background, Element, Fill};

use crate::{theme, widgets::Control};

const DIALOG_MAX_WIDTH: f32 = 560.0;

/// The ways a window-modal dialog may be dismissed without using its actions.
pub(crate) enum Dismissal<Message> {
    #[allow(dead_code)] // Supported for workflows that must be completed explicitly.
    Explicit,
    OutsideAndEscape(Message),
}

impl<Message> Dismissal<Message> {
    pub(crate) fn map<Other>(self, map: impl FnOnce(Message) -> Other) -> Dismissal<Other> {
        match self {
            Self::Explicit => Dismissal::Explicit,
            Self::OutsideAndEscape(message) => Dismissal::OutsideAndEscape(map(message)),
        }
    }

    pub(crate) fn is_outside_and_escape(&self) -> bool {
        matches!(self, Self::OutsideAndEscape(_))
    }

    fn outside_message(&self) -> Option<&Message> {
        match self {
            Self::Explicit => None,
            Self::OutsideAndEscape(message) => Some(message),
        }
    }
}

/// Presents one dialog above a retained, insensitive application window.
pub(crate) struct WindowModalHost<'a, Message> {
    base: Element<'a, Message>,
    modal: Option<(Element<'a, Message>, Dismissal<Message>, Message)>,
}

impl<'a, Message> WindowModalHost<'a, Message> {
    pub(crate) fn new(base: impl Into<Element<'a, Message>>) -> Self {
        Self {
            base: base.into(),
            modal: None,
        }
    }

    pub(crate) fn modal(
        mut self,
        content: impl Into<Element<'a, Message>>,
        dismissal: Dismissal<Message>,
        on_interaction: Message,
    ) -> Self {
        self.modal = Some((content.into(), dismissal, on_interaction));
        self
    }
}

impl<'a, Message: Clone + 'a> From<WindowModalHost<'a, Message>> for Element<'a, Message> {
    fn from(host: WindowModalHost<'a, Message>) -> Self {
        use iced::widget::{center, container, stack};

        let modal_open = host.modal.is_some();
        let base = Control::new(host.base)
            .width(Fill)
            .height(Fill)
            .sensitive(!modal_open);
        let mut layers = stack![base].width(Fill).height(Fill);

        if let Some((content, dismissal, on_interaction)) = host.modal {
            let outside_press = dismissal
                .outside_message()
                .cloned()
                .unwrap_or_else(|| on_interaction.clone());
            let dialog = Control::new(
                container(content)
                    .max_width(DIALOG_MAX_WIDTH)
                    .padding(24)
                    .style(theme::panel),
            )
            .on_press(on_interaction)
            .focus_first_descendant();
            let scrim = center(dialog).style(|_theme| container::Style {
                background: Some(Background::Color(theme::SCRIM)),
                ..container::Style::default()
            });
            let modal = Control::new(scrim)
                .on_press(outside_press)
                .focus_first_descendant()
                .width(Fill)
                .height(Fill);

            layers = layers.push(modal);
        }

        layers.into()
    }
}

#[cfg(test)]
mod tests {
    use iced::{Element, advanced::widget::Tree, widget::Space};

    use super::*;
    use crate::widgets::Interaction;

    #[derive(Clone)]
    enum Message {
        Base,
        Dismiss,
        Interaction,
    }

    fn host(open: bool) -> Element<'static, Message> {
        let base = Control::new(Space::new()).on_press(Message::Base);
        let host = WindowModalHost::new(base);

        if open {
            host.modal(
                Space::new(),
                Dismissal::OutsideAndEscape(Message::Dismiss),
                Message::Interaction,
            )
            .into()
        } else {
            host.into()
        }
    }

    fn base_interaction(tree: &mut Tree) -> &mut Interaction {
        tree.children[0].children[0]
            .state
            .downcast_mut::<Interaction>()
    }

    #[test]
    fn opening_and_closing_retains_the_base_tree() {
        use iced::advanced::widget::operation::Focusable;

        let closed = host(false);
        let mut tree = Tree::new(&closed);
        base_interaction(&mut tree).focus();

        let open = host(true);
        tree.diff(&open);
        assert_eq!(tree.children.len(), 2);
        assert!(base_interaction(&mut tree).is_focused());

        let closed = host(false);
        tree.diff(&closed);
        assert_eq!(tree.children.len(), 1);
        assert!(base_interaction(&mut tree).is_focused());
    }

    #[test]
    fn dismissal_policy_controls_outside_and_escape() {
        let explicit = Dismissal::<u8>::Explicit;
        let dismissible = Dismissal::OutsideAndEscape(7);

        assert!(!explicit.is_outside_and_escape());
        assert_eq!(explicit.outside_message(), None);
        assert!(dismissible.is_outside_and_escape());
        assert_eq!(dismissible.outside_message(), Some(&7));
    }
}
