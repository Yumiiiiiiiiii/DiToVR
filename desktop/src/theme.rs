use gpui::{App, Window, rgb};
use gpui_component::{Theme, ThemeMode};
use std::sync::atomic::{AtomicBool, Ordering};

/// 案B（ステータス）の純黒／純白パレットを使うか。起動時に一度だけ設定する。
pub static B_LOOK: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy)]
pub struct Palette {
    pub canvas: u32,
    pub surface: u32,
    pub hover: u32,
    pub text: u32,
    pub muted: u32,
    pub line: u32,
    pub input: u32,
    pub control: u32,
    pub popover: u32,
    pub blue: u32,
    pub slider: u32,
    pub on_blue: u32,
    pub brand: u32,
    pub brand_edge: u32,
    pub good: u32,
}

impl Palette {
    pub fn new(dark: bool) -> Self {
        if B_LOOK.load(Ordering::Relaxed) {
            return if dark {
                Self {
                    canvas: 0x000000,
                    surface: 0x0d0d0d,
                    hover: 0x1a1a1a,
                    text: 0xf2f2f2,
                    muted: 0xa6a6a6,
                    line: 0x262626,
                    input: 0x6a6a6a,
                    control: 0x141414,
                    popover: 0x111111,
                    blue: 0x7fb0ff,
                    slider: 0x5f8fdf,
                    on_blue: 0x000000,
                    brand: 0x3564df,
                    brand_edge: 0x5078e4,
                    good: 0x7fd4a6,
                }
            } else {
                Self {
                    canvas: 0xffffff,
                    surface: 0xffffff,
                    hover: 0xf0f0f0,
                    text: 0x111111,
                    muted: 0x595959,
                    line: 0xe0e0e0,
                    input: 0x8b8b8b,
                    control: 0xffffff,
                    popover: 0xffffff,
                    blue: 0x2456c9,
                    slider: 0x2456c9,
                    on_blue: 0xffffff,
                    brand: 0x3564df,
                    brand_edge: 0x2c56c3,
                    good: 0x23724e,
                }
            };
        }
        if dark {
            Self {
                canvas: 0x101010,
                surface: 0x191919,
                hover: 0x252525,
                text: 0xededed,
                muted: 0xaaaaaa,
                line: 0x303030,
                input: 0x707070,
                control: 0x2d2d2d,
                popover: 0x222222,
                blue: 0x8aaaff,
                slider: 0x6282d4,
                on_blue: 0x111111,
                brand: 0x3564df,
                brand_edge: 0x5078e4,
                good: 0x8bcba9,
            }
        } else {
            Self {
                canvas: 0xfafafa,
                surface: 0xffffff,
                hover: 0xededed,
                text: 0x272727,
                muted: 0x626262,
                line: 0xdedede,
                input: 0x8b8b8b,
                control: 0xffffff,
                popover: 0xffffff,
                blue: 0x3564df,
                slider: 0x3564df,
                on_blue: 0xffffff,
                brand: 0x3564df,
                brand_edge: 0x2c56c3,
                good: 0x257553,
            }
        }
    }
}

pub fn apply(dark: bool, window: &mut Window, cx: &mut App) {
    Theme::change(
        if dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        None,
        cx,
    );
    let p = Palette::new(dark);
    let t = Theme::global_mut(cx);
    t.font_family = "Yu Gothic UI".into();
    t.font_size = gpui::px(14.);
    t.radius = gpui::px(7.);
    t.shadow = false;
    t.background = rgb(p.canvas).into();
    t.foreground = rgb(p.text).into();
    t.muted = rgb(p.hover).into();
    t.muted_foreground = rgb(p.muted).into();
    t.border = rgb(p.line).into();
    t.title_bar = rgb(p.canvas).into();
    t.title_bar_border = rgb(p.line).into();
    t.input = rgb(p.input).into();
    t.accent = rgb(p.hover).into();
    t.accent_foreground = rgb(p.text).into();
    t.primary = rgb(p.blue).into();
    t.primary_hover = gpui::Hsla::from(rgb(p.blue)).opacity(0.85);
    t.primary_active = gpui::Hsla::from(rgb(p.blue)).opacity(0.75);
    t.primary_foreground = rgb(p.on_blue).into();
    t.secondary = rgb(p.surface).into();
    t.secondary_hover = rgb(p.hover).into();
    t.secondary_active = rgb(p.hover).into();
    t.secondary_foreground = rgb(p.text).into();
    t.popover = rgb(p.surface).into();
    t.popover_foreground = rgb(p.text).into();
    t.list = rgb(p.surface).into();
    t.list_hover = rgb(p.hover).into();
    t.list_active = rgb(p.hover).into();
    t.ring = rgb(p.blue).into();
    t.slider_bar = rgb(p.slider).into();
    t.slider_thumb = rgb(0xffffff).into();
    t.scrollbar = rgb(p.canvas).into();
    t.scrollbar_thumb = rgb(p.line).into();
    window.refresh();
}

// Read only. Do not change the user's Windows animation setting.
#[cfg(windows)]
pub fn windows_reduces_motion() -> bool {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SystemParametersInfoW(action: u32, param: u32, value: *mut i32, flags: u32) -> i32;
    }
    let mut enabled = 1;
    let ok = unsafe { SystemParametersInfoW(0x1042, 0, &mut enabled, 0) };
    ok != 0 && enabled == 0
}

#[cfg(not(windows))]
pub fn windows_reduces_motion() -> bool {
    false
}
