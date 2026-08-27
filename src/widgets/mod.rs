mod anchored_panel;
mod control;
mod style;
mod surface;

pub(crate) use control::{Control, event_cursor};

use iced::{ContentFit, Point, Rectangle, Size, advanced::svg::Renderer as _};

pub(crate) mod spacing {
    pub(crate) const XS: f32 = 6.0;
    pub(crate) const SM: f32 = 12.0;
    pub(crate) const MD: f32 = 18.0;
    pub(crate) const LG: f32 = 24.0;
    pub(crate) const XG: f32 = 32.0;
}

fn reconcile_index<T: PartialEq>(
    old_keys: &[T],
    old_index: Option<usize>,
    new_keys: &[T],
) -> Option<usize> {
    old_index
        .and_then(|index| old_keys.get(index))
        .and_then(|key| new_keys.iter().position(|candidate| candidate == key))
}

fn draw_caret(renderer: &mut iced::Renderer, slot: Rectangle, expansion: f32) {
    let handle = crate::icons::Icon::DownCaret.handle();
    let Size { width, height } = renderer.measure_svg(&handle);
    let size = ContentFit::Contain.fit(Size::new(width as f32, height as f32), slot.size());
    let bounds = Rectangle::new(
        Point::new(
            slot.center_x() - size.width / 2.0,
            slot.center_y() - size.height / 2.0,
        ),
        size,
    );

    renderer.draw_svg(
        iced::advanced::svg::Svg {
            handle,
            color: None,
            rotation: (std::f32::consts::PI * expansion).into(),
            opacity: 1.0,
        },
        bounds,
        slot,
    );
}

pub mod action_row;
pub mod artwork_card;
pub mod button;
pub mod card;
pub mod cycle_row;
pub mod expander_row;
pub mod header_bar;
pub mod info_card;
pub mod info_row;
pub mod list_row;
pub mod picker_row;
pub mod popover;
pub mod progress_ring;
pub mod row_group;
pub mod search;
pub mod selector_row;
pub mod status_bar;
pub mod switcher;
pub mod switcher_row;
pub mod tabs;
pub mod text;
pub mod text_row;
pub mod title;
