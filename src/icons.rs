use iced::{ContentFit, Element, widget::svg};
use rust_embed::RustEmbed;

pub const SIZE: f32 = 24.0;

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "icons/*.svg"]
struct Assets;

pub fn get(name: &str) -> svg::Handle {
    let path = format!("icons/{name}.svg");
    let icon = Assets::get(&path).unwrap_or_else(|| panic!("missing embedded icon: {path}"));

    svg::Handle::from_memory(icon.data)
}

pub fn view<'a, Message: 'a>(name: &str) -> Element<'a, Message> {
    rotated(name, 0.0)
}

pub fn rotated<'a, Message: 'a>(name: &str, rotation: f32) -> Element<'a, Message> {
    svg(get(name))
        .width(SIZE)
        .height(SIZE)
        .content_fit(ContentFit::Contain)
        .rotation(rotation)
        .into()
}

pub fn play() -> svg::Handle {
    get("play")
}

pub fn settings() -> svg::Handle {
    get("gear")
}

#[cfg(test)]
mod tests {
    use super::Assets;

    #[test]
    fn embeds_every_icon() {
        assert_eq!(Assets::iter().count(), 28);
    }
}
