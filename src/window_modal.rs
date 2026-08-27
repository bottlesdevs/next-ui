use iced::{Background, Element, Fill};

use crate::{theme, widgets::Control};

const DIALOG_MAX_WIDTH: f32 = 560.0;

/// Presents one dialog above a retained, insensitive application window.
pub(crate) fn view<'a, Message: Clone + 'a>(
    base: impl Into<Element<'a, Message>>,
    modal: Option<(Element<'a, Message>, Message)>,
    on_interaction: Message,
) -> Element<'a, Message> {
    use iced::widget::{center, container, mouse_area, opaque, stack};

    let modal_open = modal.is_some();
    let base = Control::new(base)
        .width(Fill)
        .height(Fill)
        .sensitive(!modal_open);
    let mut layers = stack![base].width(Fill).height(Fill);

    if let Some((content, on_dismiss)) = modal {
        let dialog = mouse_area(opaque(
            container(content)
                .max_width(DIALOG_MAX_WIDTH)
                .padding(24)
                .style(theme::panel),
        ))
        .on_press(on_interaction);
        let scrim = center(dialog).style(|_theme| container::Style {
            background: Some(Background::Color(theme::SCRIM)),
            ..container::Style::default()
        });
        let modal = opaque(mouse_area(scrim).on_press(on_dismiss));

        layers = layers.push(modal);
    }

    layers.into()
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
        let modal: Option<(Element<'static, Message>, Message)> =
            open.then(|| (Space::new().into(), Message::Dismiss));

        view(base, modal, Message::Interaction)
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
}
