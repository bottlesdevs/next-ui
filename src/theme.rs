//! Semantic colors for the toolkit.
use iced::{
    Background, Border, Color, Theme as IcedTheme,
    theme::{
        Mode, Palette, Style as ApplicationStyle,
        palette::{
            Background as BackgroundPalette, Danger, Extended, Pair, Primary, Secondary, Success,
            Warning,
        },
    },
    widget::{container, scrollable},
};

const BACKGROUND: Color = Color::from_rgb8(58, 50, 53);
const WINDOW: Color = Color::from_rgb8(41, 34, 37);
const WINDOW_BORDER: Color = Color::from_rgb8(54, 44, 49);
const DEEP_BACKGROUND: Color = Color::from_rgb8(27, 25, 26);
const SURFACE: Color = Color::from_rgb8(65, 57, 60);
const BORDER: Color = Color::from_rgb8(76, 66, 70);
const ROW_HOVER: Color = Color::from_rgb8(96, 85, 89);
const SURFACE_SELECTED: Color = Color::from_rgb8(89, 78, 82);

const MUTED: Color = Color::from_rgb8(166, 147, 154);
const TEXT: Color = Color::from_rgb8(250, 236, 241);
const ACCENT: Color = Color::from_rgb8(250, 230, 237);
const ACCENT_MUTED: Color = Color::from_rgb8(199, 172, 182);

const INFO: Color = Color::from_rgb8(51, 53, 71);
const ERROR: Color = Color::from_rgb8(92, 63, 63);
const WARNING: Color = Color::from_rgb8(107, 93, 71);
const SUCCESS: Color = Color::from_rgb8(53, 71, 51);
const WHITE: Color = Color::WHITE;

const BACKGROUND_LIGHT: Color = Color::from_rgb8(247, 242, 243);
const WINDOW_LIGHT: Color = Color::from_rgb8(255, 253, 254);
const WINDOW_BORDER_LIGHT: Color = Color::from_rgb8(226, 215, 219);
const DEEP_BACKGROUND_LIGHT: Color = Color::from_rgb8(251, 248, 249);
const SURFACE_DEEP_LIGHT: Color = Color::from_rgb8(214, 200, 204);
const SURFACE_LIGHT: Color = Color::from_rgb8(238, 231, 233);
const BORDER_LIGHT: Color = Color::from_rgb8(223, 212, 216);
const ROW_HOVER_LIGHT: Color = Color::from_rgb8(206, 190, 195);
const SURFACE_SELECTED_LIGHT: Color = Color::from_rgb8(216, 201, 206);

const MUTED_LIGHT: Color = Color::from_rgb8(135, 116, 122);
const TEXT_LIGHT: Color = Color::from_rgb8(36, 28, 31);
const ACCENT_LIGHT: Color = Color::from_rgb8(168, 76, 104);
const ACCENT_MUTED_LIGHT: Color = Color::from_rgb8(196, 150, 164);

/// Semantic colors used by toolkit widgets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    pub background: Color,
    pub deep_background: Color,
    pub surface: Color,
    pub panel: Color,
    pub border: Color,
    pub hover: Color,
    pub selection: Color,
    pub window: Color,
    pub window_border: Color,
    pub surface_deep: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_muted: Color,
    pub focus: Color,
    pub scrim: Color,
    pub danger: Color,
    pub danger_surface: Color,
    pub warning_surface: Color,
    pub success_surface: Color,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Motion {
    #[default]
    Full,
    Reduced,
}

impl Motion {
    pub const fn is_reduced(self) -> bool {
        matches!(self, Self::Reduced)
    }
}

impl From<bool> for Motion {
    fn from(reduced: bool) -> Self {
        if reduced { Self::Reduced } else { Self::Full }
    }
}

/// Builds the dark Iced appearance used by the toolkit.
pub fn dark() -> IcedTheme {
    custom(
        "Bottles Next",
        Palette {
            background: BACKGROUND,
            text: TEXT,
            primary: ACCENT,
            success: SUCCESS,
            warning: WARNING,
            danger: ERROR,
        },
        Extended {
            background: BackgroundPalette {
                base: pair(BACKGROUND, TEXT),
                weakest: pair(DEEP_BACKGROUND, TEXT),
                weaker: pair(BACKGROUND, TEXT),
                weak: pair(SURFACE, TEXT),
                neutral: pair(BORDER, TEXT),
                strong: pair(ROW_HOVER, TEXT),
                stronger: pair(SURFACE_SELECTED, TEXT),
                strongest: pair(WINDOW_BORDER, TEXT),
            },
            primary: Primary {
                base: pair(MUTED, TEXT),
                weak: pair(WINDOW, TEXT),
                strong: pair(ACCENT_MUTED, TEXT),
            },
            secondary: Secondary {
                base: pair(DEEP_BACKGROUND, MUTED),
                weak: pair(BACKGROUND, MUTED),
                strong: pair(WINDOW_BORDER, MUTED),
            },
            success: Success {
                base: pair(SUCCESS, WHITE),
                weak: pair(SUCCESS, WHITE),
                strong: pair(SUCCESS, WHITE),
            },
            warning: Warning {
                base: pair(WARNING, WHITE),
                weak: pair(WARNING, WHITE),
                strong: pair(WARNING, WHITE),
            },
            danger: Danger {
                base: pair(ERROR, WHITE),
                weak: pair(ERROR, WHITE),
                strong: pair(ERROR, WHITE),
            },
            is_dark: true,
        },
    )
}

/// Builds the light Iced appearance used by the toolkit.
pub fn light() -> IcedTheme {
    custom(
        "Bottles Next Light",
        Palette {
            background: BACKGROUND_LIGHT,
            text: TEXT_LIGHT,
            primary: ACCENT_LIGHT,
            success: SUCCESS,
            warning: WARNING,
            danger: ERROR,
        },
        Extended {
            background: BackgroundPalette {
                base: pair(BACKGROUND_LIGHT, TEXT_LIGHT),
                weakest: pair(DEEP_BACKGROUND_LIGHT, TEXT_LIGHT),
                weaker: pair(BACKGROUND_LIGHT, TEXT_LIGHT),
                weak: pair(SURFACE_LIGHT, TEXT_LIGHT),
                neutral: pair(BORDER_LIGHT, TEXT_LIGHT),
                strong: pair(ROW_HOVER_LIGHT, TEXT_LIGHT),
                stronger: pair(SURFACE_SELECTED_LIGHT, TEXT_LIGHT),
                strongest: pair(WINDOW_BORDER_LIGHT, TEXT_LIGHT),
            },
            primary: Primary {
                base: pair(MUTED_LIGHT, TEXT_LIGHT),
                weak: pair(WINDOW_LIGHT, TEXT_LIGHT),
                strong: pair(ACCENT_MUTED_LIGHT, TEXT_LIGHT),
            },
            secondary: Secondary {
                base: pair(DEEP_BACKGROUND_LIGHT, MUTED_LIGHT),
                weak: pair(BACKGROUND_LIGHT, MUTED_LIGHT),
                strong: pair(SURFACE_DEEP_LIGHT, MUTED_LIGHT),
            },
            success: Success {
                base: pair(SUCCESS, WHITE),
                weak: pair(SUCCESS, WHITE),
                strong: pair(SUCCESS, WHITE),
            },
            warning: Warning {
                base: pair(WARNING, WHITE),
                weak: pair(WARNING, WHITE),
                strong: pair(WARNING, WHITE),
            },
            danger: Danger {
                base: pair(ERROR, WHITE),
                weak: pair(ERROR, WHITE),
                strong: pair(ERROR, WHITE),
            },
            is_dark: false,
        },
    )
}

pub fn colors(theme: &IcedTheme) -> Colors {
    let extended = theme.extended_palette();
    let text = theme.palette().text;
    let scrim = if extended.is_dark {
        extended.background.weakest.color
    } else {
        text
    }
    .scale_alpha(171.0 / 255.0);

    Colors {
        background: extended.background.base.color,
        deep_background: extended.background.weakest.color,
        surface: extended.background.weak.color,
        panel: extended.background.weaker.color,
        border: extended.background.neutral.color,
        hover: extended.background.strong.color,
        selection: extended.background.stronger.color,
        window: extended.primary.weak.color,
        window_border: extended.background.strongest.color,
        surface_deep: extended.secondary.strong.color,
        text,
        muted: extended.secondary.weak.text,
        accent: theme.palette().primary,
        accent_muted: extended.primary.strong.color,
        focus: theme.palette().primary,
        scrim,
        danger: theme.palette().danger,
        danger_surface: theme.palette().danger,
        warning_surface: theme.palette().warning,
        success_surface: theme.palette().success,
    }
}

#[doc(hidden)]
pub fn for_mode(mode: Mode) -> IcedTheme {
    match mode {
        Mode::Light => light(),
        Mode::Dark | Mode::None => dark(),
    }
}

pub(crate) fn hint(theme: &IcedTheme) -> Pair {
    let colors = colors(theme);
    pair(colors.window, colors.text)
}

pub(crate) fn deep_surface(theme: &IcedTheme) -> Pair {
    let colors = colors(theme);
    pair(colors.surface_deep, colors.text)
}

pub(crate) fn muted(theme: &IcedTheme) -> Color {
    colors(theme).muted
}

#[doc(hidden)]
pub fn window_color(theme: &IcedTheme) -> Color {
    colors(theme).window
}

pub(crate) fn window(theme: &IcedTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(window_color(theme))),
        border: Border::default()
            .rounded(6)
            .color(colors(theme).window_border)
            .width(1),
        ..container::Style::default()
    }
}

pub(crate) fn scrim(theme: &IcedTheme) -> Color {
    colors(theme).scrim
}

pub fn application(theme: &IcedTheme) -> ApplicationStyle {
    ApplicationStyle {
        background_color: Color::TRANSPARENT,
        text_color: colors(theme).text,
    }
}

pub fn panel(theme: &IcedTheme) -> container::Style {
    let colors = colors(theme);
    surface(pair(colors.background, colors.text))
}

pub(crate) fn surface(colors: Pair) -> container::Style {
    container::Style::default()
        .color(colors.text)
        .background(colors.color)
        .border(Border::default().rounded(6))
}

pub fn scrollbar(theme: &IcedTheme, status: scrollable::Status) -> scrollable::Style {
    let mut style = scrollable::default(theme, status);
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(muted(theme)),
            border: Border::default().rounded(999),
        },
    };

    style.vertical_rail = rail;
    style.horizontal_rail = rail;
    style
}

pub(crate) const fn info() -> Pair {
    pair(INFO, WHITE)
}

fn custom(name: &'static str, palette: Palette, extended: Extended) -> IcedTheme {
    IcedTheme::custom_with_fn(name, palette, move |_| extended)
}

const fn pair(color: Color, text: Color) -> Pair {
    Pair { color, text }
}
