use gpui::{prelude::*, *};
use std::path::PathBuf;

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: u32,
    pub panel: u32,
    pub raised: u32,
    pub line: u32,
    pub text: u32,
    pub muted: u32,
    pub accent: u32,
    pub accent_bg: u32,
}
impl Palette {
    pub(crate) fn from_name(name: crate::settings::PaletteName, light: bool) -> Self {
        let d = name.definition(light);
        Self {
            bg: d.bg,
            panel: d.panel,
            raised: d.raised,
            line: d.line,
            text: d.text,
            muted: d.muted,
            accent: d.accent,
            accent_bg: d.accent_bg,
        }
    }
}
pub fn icon(name: &'static str, size: f32, color: u32) -> impl IntoElement {
    svg()
        .path(name)
        .size(px(size))
        .flex_shrink_0()
        .text_color(rgb(color))
}
pub struct Tip(pub String, pub Palette);
impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_2()
            .rounded_md()
            .text_xs()
            .bg(rgb(self.1.raised))
            .text_color(rgb(self.1.text))
            .border_1()
            .border_color(rgb(self.1.line))
            .child(self.0.clone())
    }
}
pub fn artwork(path: Option<PathBuf>, label: String, size: f32, p: Palette) -> AnyElement {
    match path {
        Some(path) => img(path)
            .size(px(size))
            .flex_shrink_0()
            .rounded_md()
            .object_fit(ObjectFit::Cover)
            .with_loading(move || placeholder(size, p).into_any_element())
            .with_fallback(move || placeholder(size, p).into_any_element())
            .into_any_element(),
        None => {
            let _ = label;
            placeholder(size, p).into_any_element()
        }
    }
}
pub(crate) fn artist_cover(
    path: Option<PathBuf>,
    label: String,
    size: f32,
    p: Palette,
) -> AnyElement {
    match path {
        Some(path) => img(path)
            .size(px(size))
            .flex_shrink_0()
            .rounded_full()
            .object_fit(ObjectFit::Cover)
            .with_loading(move || placeholder(size, p).into_any_element())
            .with_fallback(move || placeholder(size, p).into_any_element())
            .into_any_element(),
        None => {
            let _ = label;
            placeholder(size, p).into_any_element()
        }
    }
}
fn placeholder(size: f32, p: Palette) -> impl IntoElement {
    div()
        .size(px(size))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(rgb(p.line))
        .bg(rgb(p.raised))
        .text_color(rgb(p.muted))
        .child(icon("album", size * 0.35, p.muted))
}
pub fn label(text: impl Into<String>, p: Palette) -> impl IntoElement {
    div()
        .text_size(px(12.))
        .text_color(rgb(p.muted))
        .child(text.into())
}
pub fn duration(ms: u64) -> String {
    let seconds = ms / 1000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
