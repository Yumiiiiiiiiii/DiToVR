//! DiToVR's React control geometry, drawn with GPUI (no browser).
use crate::{
    motion::{Fade, Spring},
    theme::Palette,
};
use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme, Icon, StyledExt};
use std::{rc::Rc, time::Instant};

#[derive(Clone, Copy, Default)]
pub struct MotionSettings(pub bool);
impl Global for MotionSettings {}

type Click = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type Hover = Rc<dyn Fn(&bool, &mut Window, &mut App)>;

struct ButtonState {
    focus: FocusHandle,
    hover: Fade,
    press: Spring,
}

/// Geometry stays stable while the painted face and its label depress together.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    style: StyleRefinement,
    label: Option<SharedString>,
    icon: Option<Icon>,
    children: Vec<AnyElement>,
    overlays: Vec<AnyElement>,
    variant: u8,
    disabled: bool,
    font_size: f32,
    on_click: Option<Click>,
    on_hover: Option<Hover>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            label: None,
            icon: None,
            children: vec![],
            overlays: vec![],
            variant: 0,
            disabled: false,
            font_size: 14.,
            on_click: None,
            on_hover: None,
        }
    }
    pub fn ghost(mut self) -> Self {
        self.variant = 1;
        self
    }
    pub fn primary(mut self) -> Self {
        self.variant = 2;
        self
    }
    pub fn small(mut self) -> Self {
        self.font_size = 13.;
        self
    }
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }
    pub fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }
    pub fn label(mut self, value: impl Into<SharedString>) -> Self {
        self.label = Some(value.into());
        self
    }
    pub fn icon(mut self, value: impl Into<Icon>) -> Self {
        self.icon = Some(value.into());
        self
    }
    pub fn overlay(mut self, value: impl IntoElement) -> Self {
        self.overlays.push(value.into_any_element());
        self
    }
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
    pub fn on_hover(mut self, f: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Rc::new(f));
        self
    }
}
impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let reduced = cx.global::<MotionSettings>().0;
        let p = Palette::new(cx.theme().mode.is_dark());
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| ButtonState {
            focus: cx.focus_handle(),
            hover: Fade::new(0.),
            press: Spring::new(0., 500., 30.),
        });
        let now = Instant::now();
        let s = state.read(cx);
        if s.hover.active(now) || s.press.active(now) {
            window.request_animation_frame();
        }
        let h = s.hover.value(now);
        let press = if reduced {
            0.
        } else {
            s.press.value(now).clamp(0., 1.)
        };
        let focus = s.focus.clone();
        let focused = focus.is_focused(window);
        let disabled = self.disabled;
        let bg = match self.variant {
            1 => motion_color(p.canvas, p.hover, h),
            2 => motion_color(p.blue, crate::motion::mix(p.blue, p.canvas, 0.12), h),
            _ => motion_color(p.control, p.hover, h),
        };
        let fg = if self.variant == 2 { p.on_blue } else { p.text };
        let stroke = if self.variant == 0 { p.input } else { bg };
        let fill = if self.variant == 1 {
            Hsla::from(rgb(p.hover)).opacity(h * 0.65)
        } else {
            rgb(bg).into()
        };
        let border = if self.variant == 1 {
            Hsla::transparent_black()
        } else {
            rgb(stroke).into()
        };
        let hover_callback = self.on_hover;
        let click = self.on_click;
        let enter = state.clone();
        let down = state.clone();
        let up = state.clone();
        let out = state.clone();
        let key_down = state.clone();
        let key_up = state;
        div()
            .id(self.id)
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .flex_shrink_0()
            .h(px(36.))
            .px(px(if self.icon.is_some() { 12. } else { 16. }))
            .rounded(px(8.))
            .text_color(rgb(fg))
            .text_size(px(self.font_size))
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .when(disabled, |e| e.opacity(0.45).cursor_default())
            .when(!disabled, |e| {
                e.track_focus(&focus.tab_index(0).tab_stop(true))
            })
            .refine_style(&self.style)
            // A separate painted ring keeps both the control's dimensions and
            // its normal outline stable, including primary and ghost buttons.
            .when(focused && !disabled, |e| {
                e.child(
                    div()
                        .absolute()
                        .inset(px(-3.))
                        .rounded(px(10.))
                        .border_2()
                        .border_color(rgb(p.blue)),
                )
            })
            .child(
                div()
                    .absolute()
                    .inset(px(press * 1.1))
                    .rounded(px(8.))
                    .bg(fill)
                    .border_1()
                    .border_color(border),
            )
            .child(
                div()
                    .relative()
                    .top(px(press * 0.7))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(8.))
                    .text_size(px(self.font_size * (1. - press * 0.025)))
                    .when_some(self.icon, |e, icon| {
                        e.child(icon.size(px(16. * (1. - press * 0.025))))
                    })
                    .when_some(self.label, |e, label| e.child(label))
                    .children(self.children),
            )
            .children(self.overlays)
            .on_hover(move |hovered, window, cx| {
                if disabled {
                    return;
                }
                enter.update(cx, |s, cx| {
                    s.hover.set(
                        *hovered as u8 as f32,
                        if reduced { 0. } else { 0.15 },
                        Instant::now(),
                    );
                    if !hovered {
                        s.press.set(0., Instant::now(), reduced);
                    }
                    cx.notify();
                });
                if let Some(f) = &hover_callback {
                    f(hovered, window, cx);
                }
            })
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                if !disabled {
                    down.update(cx, |s, cx| {
                        s.press.set(1., Instant::now(), reduced);
                        cx.notify();
                    });
                }
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                up.update(cx, |s, cx| {
                    s.press.set(0., Instant::now(), reduced);
                    cx.notify();
                });
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                out.update(cx, |s, cx| {
                    s.press.set(0., Instant::now(), reduced);
                    cx.notify();
                });
            })
            .on_click(move |event, window, cx| {
                if !disabled && let Some(f) = &click {
                    f(event, window, cx);
                }
            })
            .on_key_down(move |e, _, cx| {
                if !disabled && ["space", "enter"].contains(&e.keystroke.key.as_str()) {
                    key_down.update(cx, |s, cx| {
                        s.press.set(1., Instant::now(), reduced);
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            })
            .on_key_up(move |e, _, cx| {
                if !disabled && ["space", "enter"].contains(&e.keystroke.key.as_str()) {
                    key_up.update(cx, |s, cx| {
                        s.press.set(0., Instant::now(), reduced);
                        cx.notify();
                    });
                    // GPUI's focused Div emits one KeyboardClickEvent itself.
                    // Only release the visual press here; invoking the action
                    // again would toggle a switch/disclosure twice.
                }
            })
    }
}

fn motion_color(a: u32, b: u32, t: f32) -> u32 {
    crate::motion::mix(a, b, t)
}

pub struct Choice {
    options: Vec<&'static str>,
    pub selected: usize,
    open: bool,
    disabled: bool,
    hot: usize,
    progress: Spring,
    value_fade: Fade,
    focus: FocusHandle,
    bounds: Bounds<Pixels>,
}
#[derive(Clone, Copy)]
pub struct ChoiceEvent(pub usize);
impl EventEmitter<ChoiceEvent> for Choice {}
impl Choice {
    pub fn new(options: Vec<&'static str>, selected: usize, cx: &mut Context<Self>) -> Self {
        Self {
            options,
            selected,
            open: false,
            disabled: false,
            hot: selected,
            progress: Spring::new(0., 500., 30.),
            value_fade: Fade::new(1.),
            focus: cx.focus_handle(),
            bounds: Bounds::default(),
        }
    }
    fn show(&mut self, open: bool, cx: &mut Context<Self>) {
        self.open = open;
        self.hot = self.selected;
        self.progress.set(
            open as u8 as f32,
            Instant::now(),
            cx.global::<MotionSettings>().0,
        );
        cx.notify();
    }
    fn choose(&mut self, index: usize, cx: &mut Context<Self>) {
        self.selected = index;
        let now = Instant::now();
        self.value_fade.set(0., 0., now);
        self.value_fade.set(
            1.,
            if cx.global::<MotionSettings>().0 {
                0.
            } else {
                0.15
            },
            now,
        );
        self.show(false, cx);
        cx.emit(ChoiceEvent(index));
    }
    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.show(false, cx);
    }
}

#[derive(IntoElement)]
pub struct Select {
    state: Entity<Choice>,
    disabled: bool,
}
impl Select {
    pub fn new(state: &Entity<Choice>) -> Self {
        Self {
            state: state.clone(),
            disabled: false,
        }
    }
    pub fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }
    pub fn w_full(self) -> Self {
        self
    }
}
impl RenderOnce for Select {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.clone();
        state.update(cx, |s, cx| {
            if s.disabled != self.disabled {
                s.disabled = self.disabled;
                if self.disabled {
                    s.close(cx);
                }
            }
        });
        div()
            .w_full()
            .when(self.disabled, |e| e.opacity(0.45))
            .when(self.disabled, |e| {
                e.capture_any_mouse_down(|_, _, cx| cx.stop_propagation())
            })
            .child(state)
    }
}
impl Render for Choice {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::new(cx.theme().mode.is_dark());
        let now = Instant::now();
        if cx.global::<MotionSettings>().0 {
            self.progress.finish(now);
            self.value_fade.finish();
        }
        let t = self.progress.value(now).clamp(0., 1.);
        if self.progress.active(now) || self.value_fade.active(now) {
            window.request_animation_frame();
        }
        let focus = self.focus.clone();
        div()
            .id("choice")
            .relative()
            .w_full()
            .when(!self.disabled, |e| {
                e.track_focus(&focus.clone().tab_index(0).tab_stop(true))
            })
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                if this.disabled {
                    return;
                }
                match e.keystroke.key.as_str() {
                    "escape" => this.show(false, cx),
                    "down" | "up" => {
                        if !this.open {
                            this.show(true, cx);
                        } else {
                            let count = this.options.len();
                            this.hot = (this.hot
                                + if e.keystroke.key == "down" {
                                    1
                                } else {
                                    count - 1
                                })
                                % count;
                            cx.notify();
                        }
                    }
                    "enter" | "space" => {
                        if this.open {
                            this.choose(this.hot, cx);
                        } else {
                            this.show(true, cx);
                        }
                    }
                    "tab" => {
                        this.show(false, cx);
                        return;
                    }
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .when(focus.is_focused(window) && !self.disabled, |e| {
                e.child(
                    div()
                        .absolute()
                        .inset(px(-3.))
                        .rounded(px(10.))
                        .border_2()
                        .border_color(rgb(p.blue)),
                )
            })
            .child(
                div()
                    .id("trigger")
                    .h(px(40.))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.))
                    .text_size(px(13.))
                    .text_color(rgb(p.text))
                    .bg(rgb(p.canvas))
                    .border_1()
                    .rounded(px(8.))
                    .border_color(rgb(if self.open || focus.is_focused(window) {
                        p.blue
                    } else {
                        p.input
                    }))
                    .cursor_pointer()
                    .child(
                        div()
                            .opacity(self.value_fade.value(now))
                            .child(self.options[self.selected]),
                    )
                    .child(
                        Icon::new(gpui_component::IconName::ChevronDown)
                            .size(px(14.))
                            .text_color(rgb(p.muted))
                            .rotate(radians(t * std::f32::consts::PI)),
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        if !this.disabled {
                            window.focus(&this.focus);
                            this.show(!this.open, cx);
                        }
                    }))
                    .child(
                        canvas(
                            {
                                let state = cx.entity();
                                move |bounds, _, cx| {
                                    state.update(cx, |s, _| s.bounds = bounds);
                                }
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    ),
            )
            .when(self.open || self.progress.active(now), |e| {
                e.child(
                    deferred(
                        anchored()
                            .position(point(self.bounds.left(), self.bounds.bottom() + px(6.)))
                            .snap_to_window_with_margin(px(8.))
                            .child(
                                div()
                                    .id("popup")
                                    .occlude()
                                    .w(self.bounds.size.width)
                                    .rounded(px(8.))
                                    .border_1()
                                    .border_color(rgb(p.line))
                                    .bg(rgb(p.popover))
                                    .shadow_md()
                                    .overflow_hidden()
                                    .opacity(t)
                                    .h(px((self.options.len() as f32 * 34. + 8.) * t))
                                    .on_mouse_down_out(cx.listener(
                                        |this, event: &MouseDownEvent, _, cx| {
                                            if this.open && !this.bounds.contains(&event.position) {
                                                this.show(false, cx);
                                            }
                                        },
                                    ))
                                    .child(div().p(px(4.)).children(
                                        self.options.iter().enumerate().map(|(i, name)| {
                                            div()
                                                .id(("option", i))
                                                .flex()
                                                .h(px(34.))
                                                .px(px(8.))
                                                .gap(px(8.))
                                                .items_center()
                                                .rounded(px(5.))
                                                .text_size(px(13.))
                                                .text_color(rgb(p.text))
                                                .cursor_pointer()
                                                .bg(rgb(if self.hot == i {
                                                    p.hover
                                                } else {
                                                    p.popover
                                                }))
                                                .child(div().w(px(16.)).when(
                                                    self.selected == i,
                                                    |e| {
                                                        e.child(
                                                            Icon::new(
                                                                gpui_component::IconName::Check,
                                                            )
                                                            .size(px(14.))
                                                            .text_color(rgb(p.blue)),
                                                        )
                                                    },
                                                ))
                                                .child(*name)
                                                .on_hover(cx.listener(move |this, hover, _, cx| {
                                                    if *hover && this.hot != i {
                                                        this.hot = i;
                                                        cx.notify();
                                                    }
                                                }))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    if this.open {
                                                        this.choose(i, cx);
                                                    }
                                                }))
                                        }),
                                    )),
                            ),
                    )
                    .with_priority(2),
                )
            })
    }
}
