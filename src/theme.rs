//! Semantic colors for the toolkit.

use crate::widgets::radius;
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

// The fill ramp, in order; see `Colors`.
const CHROME: Color = Color::from_rgb8(41, 34, 37);
const PAGE: Color = Color::from_rgb8(58, 50, 53);
const CARD: Color = Color::from_rgb8(65, 57, 60);
const RAISED: Color = Color::from_rgb8(76, 66, 70);
const SELECTED: Color = Color::from_rgb8(89, 78, 82);
const HOVERED: Color = Color::from_rgb8(96, 85, 89);

const SUNKEN: Color = Color::from_rgb8(27, 25, 26);
const CONTROL: Color = Color::from_rgb8(47, 40, 43);
const CHROME_BORDER: Color = Color::from_rgb8(54, 44, 49);

const MUTED: Color = Color::from_rgb8(166, 147, 154);
const TEXT: Color = Color::from_rgb8(250, 236, 241);
const ACCENT: Color = Color::from_rgb8(250, 230, 237);
const ACCENT_MUTED: Color = Color::from_rgb8(199, 172, 182);

const INFO: Color = Color::from_rgb8(51, 53, 71);
const ERROR: Color = Color::from_rgb8(92, 63, 63);
const WARNING: Color = Color::from_rgb8(107, 93, 71);
const SUCCESS: Color = Color::from_rgb8(53, 71, 51);
const WHITE: Color = Color::WHITE;

const CHROME_LIGHT: Color = Color::from_rgb8(255, 253, 254);
const PAGE_LIGHT: Color = Color::from_rgb8(247, 242, 243);
const CARD_LIGHT: Color = Color::from_rgb8(238, 231, 233);
const RAISED_LIGHT: Color = Color::from_rgb8(223, 212, 216);
const SELECTED_LIGHT: Color = Color::from_rgb8(216, 201, 206);
const HOVERED_LIGHT: Color = Color::from_rgb8(206, 190, 195);

const SUNKEN_LIGHT: Color = Color::from_rgb8(251, 248, 249);
const CONTROL_LIGHT: Color = Color::from_rgb8(230, 221, 224);
const CHROME_BORDER_LIGHT: Color = Color::from_rgb8(226, 215, 219);
const CHROME_HOVERED_LIGHT: Color = Color::from_rgb8(214, 200, 204);

const MUTED_LIGHT: Color = Color::from_rgb8(135, 116, 122);
const TEXT_LIGHT: Color = Color::from_rgb8(36, 28, 31);
const ACCENT_LIGHT: Color = Color::from_rgb8(168, 76, 104);
const ACCENT_MUTED_LIGHT: Color = Color::from_rgb8(196, 150, 164);

/// Semantic colors used by toolkit widgets.
///
/// Fills climb one ramp, each step further from the window: `chrome` (the
/// window frame and header bars), `page`, `card` (rows and cards on a page),
/// `raised` (blocks on a card, and lines drawn on any level), `selected`,
/// `hovered`. Dark steps get lighter, light ones darker. The other fills are
/// roles off the ramp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    pub chrome: Color,
    pub page: Color,
    pub card: Color,
    pub raised: Color,
    pub selected: Color,
    pub hovered: Color,
    /// Set below the page, e.g. a progress track or a pressed chrome button.
    pub sunken: Color,
    /// A button set back into a row or card.
    pub control: Color,
    /// The outline around the window.
    pub chrome_border: Color,
    /// A chrome-filled button under the pointer.
    pub chrome_hovered: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_muted: Color,
    pub scrim: Color,
    pub info: Color,
    pub danger: Color,
    pub warning: Color,
    pub success: Color,
}

/// Builds the dark Iced appearance used by the toolkit.
pub fn dark() -> IcedTheme {
    custom(
        "Bottles Next",
        Palette {
            background: PAGE,
            text: TEXT,
            primary: ACCENT,
            success: SUCCESS,
            warning: WARNING,
            danger: ERROR,
        },
        Extended {
            background: BackgroundPalette {
                base: pair(PAGE, TEXT),
                weakest: pair(SUNKEN, TEXT),
                weaker: pair(PAGE, TEXT),
                weak: pair(CARD, TEXT),
                neutral: pair(RAISED, TEXT),
                strong: pair(HOVERED, TEXT),
                stronger: pair(SELECTED, TEXT),
                strongest: pair(CHROME_BORDER, TEXT),
            },
            primary: Primary {
                base: pair(MUTED, TEXT),
                weak: pair(CHROME, TEXT),
                strong: pair(ACCENT_MUTED, TEXT),
            },
            secondary: Secondary {
                base: pair(SUNKEN, MUTED),
                weak: pair(PAGE, MUTED),
                strong: pair(CHROME_BORDER, MUTED),
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
            background: PAGE_LIGHT,
            text: TEXT_LIGHT,
            primary: ACCENT_LIGHT,
            success: SUCCESS,
            warning: WARNING,
            danger: ERROR,
        },
        Extended {
            background: BackgroundPalette {
                base: pair(PAGE_LIGHT, TEXT_LIGHT),
                weakest: pair(SUNKEN_LIGHT, TEXT_LIGHT),
                weaker: pair(PAGE_LIGHT, TEXT_LIGHT),
                weak: pair(CARD_LIGHT, TEXT_LIGHT),
                neutral: pair(RAISED_LIGHT, TEXT_LIGHT),
                strong: pair(HOVERED_LIGHT, TEXT_LIGHT),
                stronger: pair(SELECTED_LIGHT, TEXT_LIGHT),
                strongest: pair(CHROME_BORDER_LIGHT, TEXT_LIGHT),
            },
            primary: Primary {
                base: pair(MUTED_LIGHT, TEXT_LIGHT),
                weak: pair(CHROME_LIGHT, TEXT_LIGHT),
                strong: pair(ACCENT_MUTED_LIGHT, TEXT_LIGHT),
            },
            secondary: Secondary {
                base: pair(SUNKEN_LIGHT, MUTED_LIGHT),
                weak: pair(PAGE_LIGHT, MUTED_LIGHT),
                strong: pair(CHROME_HOVERED_LIGHT, MUTED_LIGHT),
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

/// Resolves semantic toolkit colors from an Iced appearance.
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
        chrome: extended.primary.weak.color,
        page: extended.background.base.color,
        card: extended.background.weak.color,
        raised: extended.background.neutral.color,
        selected: extended.background.stronger.color,
        hovered: extended.background.strong.color,
        sunken: extended.background.weakest.color,
        control: if extended.is_dark {
            CONTROL
        } else {
            CONTROL_LIGHT
        },
        chrome_border: extended.background.strongest.color,
        chrome_hovered: extended.secondary.strong.color,
        text,
        muted: extended.secondary.weak.text,
        accent: theme.palette().primary,
        accent_muted: extended.primary.strong.color,
        scrim,
        info: INFO,
        danger: theme.palette().danger,
        warning: theme.palette().warning,
        success: theme.palette().success,
    }
}

#[doc(hidden)]
pub fn for_mode(mode: Mode) -> IcedTheme {
    match mode {
        Mode::Light => light(),
        Mode::Dark | Mode::None => dark(),
    }
}

pub(crate) fn muted(theme: &IcedTheme) -> Color {
    colors(theme).muted
}

pub(crate) fn scrim(theme: &IcedTheme) -> Color {
    colors(theme).scrim
}

/// Styles the transparent Iced application root.
pub fn application(theme: &IcedTheme) -> ApplicationStyle {
    ApplicationStyle {
        background_color: Color::TRANSPARENT,
        text_color: colors(theme).text,
    }
}

/// Styles a standard application panel.
pub fn panel(theme: &IcedTheme) -> container::Style {
    let colors = colors(theme);
    surface(pair(colors.page, colors.text))
}

pub(crate) fn card(theme: &IcedTheme) -> container::Style {
    let colors = colors(theme);
    surface(pair(colors.card, colors.text))
}

/// A panel floating over the page, e.g. a menu. The design is flat, so a
/// fill a step lighter than the rows it covers, not a shadow or an outline,
/// sets it apart.
pub(crate) fn overlay(theme: &IcedTheme) -> container::Style {
    let colors = colors(theme);
    surface(pair(colors.selected, colors.text))
}

pub(crate) fn surface(colors: Pair) -> container::Style {
    container::Style::default()
        .color(colors.text)
        .background(colors.color)
        .border(Border::default().rounded(radius::ROW))
}

/// Styles a scrollbar using semantic muted colors.
pub fn scrollbar(theme: &IcedTheme, status: scrollable::Status) -> scrollable::Style {
    let mut style = scrollable::default(theme, status);
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(muted(theme)),
            border: Border::default().rounded(radius::PILL),
        },
    };

    style.vertical_rail = rail;
    style.horizontal_rail = rail;
    style
}

fn custom(name: &'static str, palette: Palette, extended: Extended) -> IcedTheme {
    IcedTheme::custom_with_fn(name, palette, move |_| extended)
}

const fn pair(color: Color, text: Color) -> Pair {
    Pair { color, text }
}
