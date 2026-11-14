use crate::Element;

use super::inert::Inert;

/// Shows `content` as usual while `enabled`, and otherwise disabled as a
/// whole: it takes no input or focus, and draws with
/// [`Theme::disabled`](crate::Theme::disabled).
///
/// One switch for a group of controls, so none of them needs telling. The
/// content's tree is kept as is, so switching `enabled` loses no state.
pub fn sensitive<'a, Message: 'a>(
    enabled: bool,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    if enabled {
        content.into()
    } else {
        Element::new(Inert::new(content, true))
    }
}
