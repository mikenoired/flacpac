use crate::ui::{Palette, Tip, icon};
use gpui::{prelude::*, *};

pub fn icon_button(
    id: impl Into<String>,
    name: &'static str,
    title: &'static str,
    p: Palette,
    enabled: bool,
    primary: bool,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let id = id.into();
    div()
        .id(id.clone())
        .debug_selector(move || id.clone())
        .role(Role::Button)
        .aria_label(title)
        .border_1()
        .border_color(rgba(0x00000000))
        .size(px(if primary && title == "Play / pause · Space" {
            38.
        } else {
            32.
        }))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .text_color(rgb(if !enabled {
            p.muted
        } else if primary {
            p.bg
        } else {
            p.text
        }))
        .bg(rgb(if primary && enabled {
            p.accent
        } else {
            p.panel
        }))
        .opacity(if enabled { 1. } else { 0.35 })
        .when(enabled, |el| {
            el.tab_index(0)
                .focus_visible(move |style| style.border_color(rgb(p.accent)))
                .cursor_pointer()
                .hover(move |s| s.bg(rgb(if primary { p.text } else { p.raised })))
                .active(move |s| s.bg(rgb(p.accent_bg)))
                .on_click(listener)
        })
        .tooltip(move |_, cx| cx.new(|_| Tip(title.into(), p)).into())
        .child(icon(
            name,
            if primary { 19. } else { 17. },
            if primary && enabled { p.bg } else { p.text },
        ))
}

pub fn icon_toggle_button(
    id: &'static str,
    name: &'static str,
    title: &'static str,
    p: Palette,
    enabled: bool,
    active: bool,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(title)
        .aria_toggled(if active {
            Toggled::True
        } else {
            Toggled::False
        })
        .border_1()
        .border_color(rgba(0x00000000))
        .size(px(32.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(if active { p.accent } else { p.text }))
        .bg(rgb(p.panel))
        .opacity(if enabled { 1. } else { 0.35 })
        .when(enabled, |el| {
            el.tab_index(0)
                .focus_visible(move |s| s.border_color(rgb(p.accent)))
                .cursor_pointer()
                .hover(move |s| s.text_color(rgb(p.accent)))
                .on_click(listener)
        })
        .tooltip(move |_, cx| cx.new(|_| Tip(title.into(), p)).into())
        .child(icon(name, 17., if active { p.accent } else { p.text }))
}

pub fn text_button(
    id: impl Into<String>,
    name: &'static str,
    label: impl Into<String>,
    p: Palette,
    primary: bool,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id.into())
        .role(Role::Button)
        .tab_index(0)
        .border_1()
        .border_color(rgba(0x00000000))
        .focus_visible(move |style| style.border_color(rgb(p.accent)))
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .h(px(32.))
        .px_3()
        .rounded_md()
        .cursor_pointer()
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .bg(rgb(if primary { p.accent } else { p.raised }))
        .text_color(rgb(if primary { p.bg } else { p.text }))
        .hover(move |s| s.bg(rgb(if primary { p.text } else { p.accent_bg })))
        .active(move |s| s.opacity(0.7))
        .on_click(listener)
        .child(icon(name, 14., if primary { p.bg } else { p.text }))
        .child(label.into())
}
