//! The toolkit's theme: its semantic colors, and how iced widgets take them.

use crate::widgets::radius;
use iced::{
    Background, Border, Color, Shadow, Vector,
    theme::{
        Base, Mode, Palette, Style as ApplicationStyle,
        palette::{self, Pair},
    },
    widget::{button, container, progress_bar, rule, scrollable, svg, text, text_input},
};

/// The theme every toolkit widget is drawn with: a palette, and the state
/// inherited from the widgets above the one being drawn.
///
/// Disabled is a property of widgets. iced does not propagate widget state
/// down the tree, but the theme reaches every widget it draws, so it carries
/// that state. [`Theme::colors`] then resolves the palette for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub mode: Mode,
    palette: Colors,
    disabled: bool,
}

impl Theme {
    /// The theme for `mode`; with no preference, dark.
    pub fn new(mode: Mode) -> Self {
        let (mode, palette) = match mode {
            Mode::Light => (mode, Colors::LIGHT),
            Mode::Dark | Mode::None => (Mode::Dark, Colors::DARK),
        };

        Self {
            mode,
            palette,
            disabled: false,
        }
    }

    /// The theme for drawing a disabled widget and everything inside it.
    /// Widgets draw as usual with it, so none needs a disabled style of its
    /// own, and a disabled widget inside a disabled group looks the same as
    /// one on its own.
    pub fn disabled(self) -> Self {
        Self {
            disabled: true,
            ..self
        }
    }

    /// The state layer every action draws over its resting look while the
    /// pointer is on it or it is pressed: its label's `content` color,
    /// faintly, so it suits any surface.
    pub(crate) fn state_layer(
        &self,
        content: Color,
        hovered: bool,
        pressed: bool,
    ) -> Option<Color> {
        if pressed {
            Some(content.scale_alpha(0.16))
        } else if hovered {
            Some(content.scale_alpha(0.08))
        } else {
            None
        }
    }

    /// The palette's colors for the inherited state.
    pub fn colors(&self) -> Colors {
        if self.disabled {
            self.palette.disabled()
        } else {
            self.palette
        }
    }
}

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
    /// Set below the page, e.g. a progress track.
    pub sunken: Color,
    /// A button set back into a row or card.
    pub control: Color,
    /// The outline around the window.
    pub chrome_border: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_muted: Color,
    /// Dims what a dialog covers.
    pub scrim: Color,
    pub info: Color,
    pub danger: Color,
    pub warning: Color,
    pub success: Color,
    /// Text on the `info`, `danger`, `warning` and `success` fills.
    pub on_status: Color,
}

const SCRIM_ALPHA: f32 = 171.0 / 255.0;
const INFO: Color = Color::from_rgb8(51, 53, 71);
const DANGER: Color = Color::from_rgb8(92, 63, 63);
const WARNING: Color = Color::from_rgb8(107, 93, 71);
const SUCCESS: Color = Color::from_rgb8(53, 71, 51);

impl Colors {
    /// The disabled color group: every color faded halfway into the page,
    /// except the page itself and the scrim.
    fn disabled(self) -> Self {
        let fade = |color| palette::mix(color, self.page, 0.5);

        Self {
            chrome: fade(self.chrome),
            page: self.page,
            card: fade(self.card),
            raised: fade(self.raised),
            selected: fade(self.selected),
            hovered: fade(self.hovered),
            sunken: fade(self.sunken),
            control: fade(self.control),
            chrome_border: fade(self.chrome_border),
            text: fade(self.text),
            muted: fade(self.muted),
            accent: fade(self.accent),
            accent_muted: fade(self.accent_muted),
            scrim: self.scrim,
            info: fade(self.info),
            danger: fade(self.danger),
            warning: fade(self.warning),
            success: fade(self.success),
            on_status: fade(self.on_status),
        }
    }

    const DARK: Self = Self {
        chrome: Color::from_rgb8(41, 34, 37),
        page: Color::from_rgb8(58, 50, 53),
        card: Color::from_rgb8(65, 57, 60),
        raised: Color::from_rgb8(76, 66, 70),
        selected: Color::from_rgb8(89, 78, 82),
        hovered: Color::from_rgb8(96, 85, 89),
        sunken: Color::from_rgb8(27, 25, 26),
        control: Color::from_rgb8(47, 40, 43),
        chrome_border: Color::from_rgb8(54, 44, 49),
        text: Color::from_rgb8(250, 236, 241),
        muted: Color::from_rgb8(166, 147, 154),
        accent: Color::from_rgb8(250, 230, 237),
        accent_muted: Color::from_rgb8(199, 172, 182),
        scrim: Color::from_rgba8(27, 25, 26, SCRIM_ALPHA),
        info: INFO,
        danger: DANGER,
        warning: WARNING,
        success: SUCCESS,
        on_status: Color::WHITE,
    };

    const LIGHT: Self = Self {
        chrome: Color::from_rgb8(255, 253, 254),
        page: Color::from_rgb8(247, 242, 243),
        card: Color::from_rgb8(238, 231, 233),
        raised: Color::from_rgb8(223, 212, 216),
        selected: Color::from_rgb8(216, 201, 206),
        hovered: Color::from_rgb8(206, 190, 195),
        sunken: Color::from_rgb8(251, 248, 249),
        control: Color::from_rgb8(230, 221, 224),
        chrome_border: Color::from_rgb8(226, 215, 219),
        text: Color::from_rgb8(36, 28, 31),
        muted: Color::from_rgb8(135, 116, 122),
        accent: Color::from_rgb8(168, 76, 104),
        accent_muted: Color::from_rgb8(196, 150, 164),
        scrim: Color::from_rgba8(36, 28, 31, SCRIM_ALPHA),
        info: INFO,
        danger: DANGER,
        warning: WARNING,
        success: SUCCESS,
        on_status: Color::WHITE,
    };
}

impl Base for Theme {
    fn default(preference: Mode) -> Self {
        Self::new(preference)
    }

    fn mode(&self) -> Mode {
        self.mode
    }

    /// The application root is transparent, so the window frame can round
    /// its corners.
    fn base(&self) -> ApplicationStyle {
        ApplicationStyle {
            background_color: Color::TRANSPARENT,
            text_color: self.colors().text,
        }
    }

    fn palette(&self) -> Option<Palette> {
        None
    }

    fn name(&self) -> &str {
        match self.mode {
            Mode::Light => "Bottles Next Light",
            Mode::Dark | Mode::None => "Bottles Next",
        }
    }
}

/// The filled, rounded roles a container takes. The design is flat: a fill
/// further up the ramp, never a shadow or an outline, sets a surface apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// A page or panel.
    Page,
    /// A row or card on a page.
    Card,
    /// A block on a card, e.g. a tag or a field.
    Raised,
    /// A raised box that holds content of its own, e.g. a card in a popover.
    Boxed,
    /// A panel floating over the page, e.g. a menu: a step above the rows it
    /// covers.
    Overlay,
    /// A banner, filled by what it reports.
    Banner(Tone),
}

/// What a banner reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Hint,
    Info,
    Error,
    Warning,
    Success,
}

impl Surface {
    /// The fill and the text color on it.
    pub fn colors(self, theme: &Theme) -> Pair {
        let colors = theme.colors();
        let on = |color| Pair {
            color,
            text: colors.text,
        };
        let status = |color| Pair {
            color,
            text: colors.on_status,
        };

        match self {
            Self::Page => on(colors.page),
            Self::Card => on(colors.card),
            Self::Raised | Self::Boxed => on(colors.raised),
            Self::Overlay => on(colors.selected),
            Self::Banner(Tone::Hint) => on(colors.chrome),
            Self::Banner(Tone::Info) => status(colors.info),
            Self::Banner(Tone::Error) => status(colors.danger),
            Self::Banner(Tone::Warning) => status(colors.warning),
            Self::Banner(Tone::Success) => status(colors.success),
        }
    }

    pub fn style(self, theme: &Theme) -> container::Style {
        let colors = self.colors(theme);

        container::Style::default()
            .color(colors.text)
            .background(colors.color)
            .border(Border::default().rounded(match self {
                Self::Boxed => radius::BOX,
                _ => radius::ROW,
            }))
    }
}

/// Styles a container as `role`, for `container(..).style(surface(..))`.
pub fn surface(role: Surface) -> impl Fn(&Theme) -> container::Style {
    move |theme| role.style(theme)
}

// Every iced widget the toolkit draws takes its style as a closure over this
// theme. The defaults are the toolkit's look for a widget given no style.

impl button::Catalog for Theme {
    type Class<'a> = button::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme, _| button::Style {
            text_color: theme.colors().text,
            ..button::Style::default()
        })
    }

    fn style(&self, class: &Self::Class<'_>, status: button::Status) -> button::Style {
        class(self, status)
    }
}

impl container::Catalog for Theme {
    type Class<'a> = container::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(container::transparent)
    }

    fn style(&self, class: &Self::Class<'_>) -> container::Style {
        class(self)
    }
}

impl text::Catalog for Theme {
    type Class<'a> = text::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|_| text::Style::default())
    }

    fn style(&self, class: &Self::Class<'_>) -> text::Style {
        class(self)
    }
}

impl text_input::Catalog for Theme {
    type Class<'a> = text_input::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme, _| text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            icon: theme.colors().muted,
            placeholder: theme.colors().muted,
            value: theme.colors().text,
            selection: theme.colors().accent,
        })
    }

    fn style(&self, class: &Self::Class<'_>, status: text_input::Status) -> text_input::Style {
        class(self, status)
    }
}

impl scrollable::Catalog for Theme {
    type Class<'a> = scrollable::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme, _| {
            let colors = theme.colors();
            let rail = scrollable::Rail {
                background: None,
                border: Border::default(),
                scroller: scrollable::Scroller {
                    background: Background::Color(colors.muted),
                    border: Border::default().rounded(radius::PILL),
                },
            };

            scrollable::Style {
                container: container::Style::default(),
                vertical_rail: rail,
                horizontal_rail: rail,
                gap: None,
                auto_scroll: scrollable::AutoScroll {
                    background: Background::Color(colors.page.scale_alpha(0.9)),
                    border: Border::default()
                        .rounded(radius::PILL)
                        .width(1)
                        .color(colors.text.scale_alpha(0.8)),
                    shadow: Shadow {
                        color: Color::BLACK.scale_alpha(0.7),
                        offset: Vector::ZERO,
                        blur_radius: 2.0,
                    },
                    icon: colors.text.scale_alpha(0.8),
                },
            }
        })
    }

    fn style(&self, class: &Self::Class<'_>, status: scrollable::Status) -> scrollable::Style {
        class(self, status)
    }
}

impl svg::Catalog for Theme {
    type Class<'a> = svg::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|_, _| svg::Style::default())
    }

    fn style(&self, class: &Self::Class<'_>, status: svg::Status) -> svg::Style {
        class(self, status)
    }
}

impl progress_bar::Catalog for Theme {
    type Class<'a> = progress_bar::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme| progress_bar::Style {
            background: Background::Color(theme.colors().sunken),
            bar: Background::Color(theme.colors().accent),
            border: Border::default().rounded(radius::PILL),
        })
    }

    fn style(&self, class: &Self::Class<'_>) -> progress_bar::Style {
        class(self)
    }
}

impl rule::Catalog for Theme {
    type Class<'a> = rule::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme| rule::Style {
            color: theme.colors().raised,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
    }

    fn style(&self, class: &Self::Class<'_>) -> rule::Style {
        class(self)
    }
}
