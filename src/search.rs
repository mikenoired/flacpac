use gpui::{prelude::*, *};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

pub struct SearchChanged(pub String);
pub struct SearchInput {
    pub focus: FocusHandle,
    pub light: bool,
    pub palette: crate::settings::PaletteName,
    text: String,
    anchor: usize,
    cursor: usize,
    marked: Option<Range<usize>>,
    bounds: Bounds<Pixels>,
    line: Option<ShapedLine>,
    offset: Pixels,
    dragging: bool,
}
impl EventEmitter<SearchChanged> for SearchInput {}
impl SearchInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            light: false,
            palette: Default::default(),
            text: String::new(),
            anchor: 0,
            cursor: 0,
            marked: None,
            bounds: Bounds::default(),
            line: None,
            offset: px(0.),
            dragging: false,
        }
    }
    fn selection(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.text.clear();
        self.cursor = 0;
        self.anchor = 0;
        self.marked = None;
        self.changed(cx);
    }
    fn changed(&self, cx: &mut Context<Self>) {
        cx.emit(SearchChanged(self.text.clone()));
        cx.notify();
    }
    fn replace(&mut self, range: Range<usize>, text: &str, cx: &mut Context<Self>) {
        let text: String = text.chars().filter(|c| !c.is_control()).collect();
        self.text.replace_range(range.clone(), &text);
        self.cursor = range.start + text.len();
        self.anchor = self.cursor;
        self.changed(cx);
    }
    fn hit(&self, position: Point<Pixels>) -> usize {
        self.line.as_ref().map_or(0, |line| {
            line.closest_index_for_x(position.x - self.bounds.left() + self.offset)
                .min(self.text.len())
        })
    }
    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke.key;
        let modifiers = event.keystroke.modifiers;
        let command = modifiers.platform || modifiers.control;
        let selected = self.selection();
        match (command, key.as_str()) {
            (true, "a") => {
                self.anchor = 0;
                self.cursor = self.text.len();
            }
            (true, "c" | "x") => {
                if !selected.is_empty() {
                    cx.write_to_clipboard(ClipboardItem::new_string(
                        self.text[selected.clone()].to_owned(),
                    ));
                }
                if key == "x" {
                    self.replace(selected, "", cx);
                }
            }
            (true, "v") => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    self.replace(selected, &text, cx);
                }
            }
            (_, "backspace" | "delete") => {
                let range = if !selected.is_empty() {
                    selected
                } else if key == "backspace" {
                    let previous = self.text[..self.cursor]
                        .grapheme_indices(true)
                        .next_back()
                        .map_or(0, |(index, _)| index);
                    (if command { 0 } else { previous })..self.cursor
                } else {
                    let next = self.text[self.cursor..]
                        .graphemes(true)
                        .next()
                        .map_or(self.cursor, |text| self.cursor + text.len());
                    self.cursor..(if command { self.text.len() } else { next })
                };
                self.marked = None;
                self.replace(range, "", cx);
            }
            (_, "left" | "right" | "home" | "end") => {
                let target = match key.as_str() {
                    "home" => 0,
                    "end" => self.text.len(),
                    "left" if command => 0,
                    "right" if command => self.text.len(),
                    "left" if !modifiers.shift && !selected.is_empty() => selected.start,
                    "right" if !modifiers.shift && !selected.is_empty() => selected.end,
                    "left" => self.text[..self.cursor]
                        .grapheme_indices(true)
                        .next_back()
                        .map_or(0, |(index, _)| index),
                    _ => self.text[self.cursor..]
                        .graphemes(true)
                        .next()
                        .map_or(self.cursor, |text| self.cursor + text.len()),
                };
                self.cursor = target;
                if !modifiers.shift {
                    self.anchor = target;
                }
            }
            (_, "escape") => {
                self.clear(cx);
                window.blur();
            }
            (_, "enter") => window.blur(),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }
}
fn to_utf16(text: &str, byte: usize) -> usize {
    text[..byte].encode_utf16().count()
}
fn from_utf16(text: &str, offset: usize) -> usize {
    let mut count = 0;
    for (index, c) in text.char_indices() {
        if count + c.len_utf16() > offset {
            return index;
        }
        count += c.len_utf16();
    }
    text.len()
}
fn bytes(text: &str, range: Range<usize>) -> Range<usize> {
    from_utf16(text, range.start)..from_utf16(text, range.end)
}
impl EntityInputHandler for SearchInput {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = bytes(&self.text, range);
        *adjusted = Some(to_utf16(&self.text, range.start)..to_utf16(&self.text, range.end));
        Some(self.text[range].to_owned())
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let range = self.selection();
        Some(UTF16Selection {
            range: to_utf16(&self.text, range.start)..to_utf16(&self.text, range.end),
            reversed: self.cursor < self.anchor,
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked
            .as_ref()
            .map(|r| to_utf16(&self.text, r.start)..to_utf16(&self.text, r.end))
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        cx.notify();
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .map(|r| bytes(&self.text, r))
            .or_else(|| self.marked.clone())
            .unwrap_or_else(|| self.selection());
        self.marked = None;
        self.replace(range, text, cx);
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selection: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .map(|r| bytes(&self.text, r))
            .or_else(|| self.marked.clone())
            .unwrap_or_else(|| self.selection());
        let start = range.start;
        self.replace(range, text, cx);
        self.marked = Some(start..self.cursor);
        if let Some(selection) = selection {
            let range = bytes(&self.text[start..self.cursor], selection);
            self.anchor = start + range.start;
            self.cursor = start + range.end;
        }
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = bytes(&self.text, range);
        let line = self.line.as_ref()?;
        Some(Bounds::new(
            point(
                self.bounds.left() + line.x_for_index(range.start) - self.offset,
                self.bounds.top(),
            ),
            size(
                (line.x_for_index(range.end) - line.x_for_index(range.start)).max(px(1.)),
                self.bounds.size.height,
            ),
        ))
    }
    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        Some(to_utf16(&self.text, self.hit(point)))
    }
    fn set_selected_text_range(
        &mut self,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = bytes(&self.text, range);
        self.anchor = range.start;
        self.cursor = range.end;
        cx.notify();
    }
}
impl Render for SearchInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);
        let entity = cx.entity();
        let paint_entity = entity.clone();
        let palette = self.palette.definition(self.light);
        let border = if focused {
            palette.accent
        } else {
            palette.line
        };
        div()
            .id("search-input")
            .debug_selector(|| "search-field".into())
            .role(Role::TextInput)
            .aria_label("Search library")
            .tab_index(0)
            .track_focus(&self.focus)
            .cursor(CursorStyle::IBeam)
            .flex()
            .items_center()
            .gap_2()
            .h(px(36.))
            .w(px(280.))
            .px_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(border))
            .bg(rgb(palette.panel))
            .text_color(rgb(palette.text))
            .on_key_down(cx.listener(Self::key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    let index = this.hit(event.position);
                    this.cursor = index;
                    if !event.modifiers.shift {
                        this.anchor = index;
                    }
                    if event.click_count > 1 {
                        this.anchor = 0;
                        this.cursor = this.text.len();
                    }
                    this.dragging = true;
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if this.dragging && event.pressed_button == Some(MouseButton::Left) {
                    this.cursor = this.hit(event.position);
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = false),
            )
            .child(
                svg()
                    .path("search")
                    .size(px(16.))
                    .text_color(rgb(palette.muted)),
            )
            .child(
                div().flex_1().min_w_0().overflow_hidden().child(
                    canvas(
                        move |bounds, window, cx| {
                            entity.update(cx, |this, _| {
                                let style = window.text_style();
                                let placeholder = this.text.is_empty();
                                let text: SharedString = if placeholder {
                                    "Search your library…".into()
                                } else {
                                    this.text.clone().into()
                                };
                                let color = if placeholder {
                                    rgb_to_hsla(rgb(palette.muted))
                                } else {
                                    style.color
                                };
                                let run = TextRun {
                                    len: text.len(),
                                    font: style.font(),
                                    color,
                                    background_color: None,
                                    letter_spacing: None,
                                    underline: None,
                                    strikethrough: None,
                                };
                                let line =
                                    window.text_system().shape_line(text, px(13.), &[run], None);
                                let caret = if placeholder {
                                    px(0.)
                                } else {
                                    line.x_for_index(this.cursor)
                                };
                                this.offset = this
                                    .offset
                                    .min(caret)
                                    .max((caret - bounds.size.width + px(2.)).max(px(0.)));
                                this.bounds = bounds;
                                this.line = Some(line.clone());
                                (
                                    line,
                                    this.offset,
                                    this.selection(),
                                    this.cursor,
                                    placeholder,
                                    style.color,
                                )
                            })
                        },
                        move |bounds,
                              (line, offset, selection, cursor, placeholder, color),
                              window,
                              cx| {
                            let focus = paint_entity.read(cx).focus.clone();
                            window.handle_input(
                                &focus,
                                ElementInputHandler::new(bounds, paint_entity),
                                cx,
                            );
                            window.with_content_mask(Some(ContentMask { bounds }), |window| {
                                let origin = point(bounds.left() - offset, bounds.top());
                                if focused && !selection.is_empty() && !placeholder {
                                    window.paint_quad(fill(
                                        Bounds::new(
                                            point(
                                                origin.x + line.x_for_index(selection.start),
                                                bounds.top(),
                                            ),
                                            size(
                                                line.x_for_index(selection.end)
                                                    - line.x_for_index(selection.start),
                                                bounds.size.height,
                                            ),
                                        ),
                                        rgba((palette.accent << 8) | 0x55),
                                    ));
                                }
                                let _ = line.paint(
                                    origin,
                                    bounds.size.height,
                                    TextAlign::Left,
                                    None,
                                    window,
                                    cx,
                                );
                                if focused {
                                    let x = if placeholder {
                                        px(0.)
                                    } else {
                                        line.x_for_index(cursor)
                                    };
                                    window.paint_quad(fill(
                                        Bounds::new(
                                            point(origin.x + x, bounds.top() + px(2.)),
                                            size(px(1.5), bounds.size.height - px(4.)),
                                        ),
                                        color,
                                    ));
                                }
                            });
                        },
                    )
                    .w_full()
                    .h(px(22.)),
                ),
            )
            .when(!self.text.is_empty(), |this| {
                this.child(
                    div()
                        .id("clear-search")
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| this.clear(cx)))
                        .child(
                            svg()
                                .path("close")
                                .size(px(14.))
                                .text_color(rgb(palette.muted)),
                        ),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn native_selection_ranges_preserve_cyrillic_and_surrogate_pairs() {
        let text = "Трек 🎵 Björk";
        for (byte, _) in text
            .char_indices()
            .chain(std::iter::once((text.len(), ' ')))
        {
            assert_eq!(from_utf16(text, to_utf16(text, byte)), byte);
        }
        let start = text.find('🎵').unwrap();
        let range = to_utf16(text, start)..to_utf16(text, start + "🎵".len());
        assert_eq!(&text[bytes(text, range)], "🎵");
    }
}

#[cfg(test)]
mod native_input_tests {
    use super::SearchInput;
    use gpui::{AppContext, ClipboardItem, TestAppContext};

    #[gpui::test]
    fn typing_selection_and_paste_use_the_native_input_handler(cx: &mut TestAppContext) {
        let (input, cx) = cx.add_window_view(|_, cx| SearchInput::new(cx));
        cx.update(|window, cx| {
            let focus = input.read(cx).focus.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_input("Bonobo");
        cx.read_entity(&input, |input, _| assert_eq!(input.text, "Bonobo"));
        cx.simulate_keystrokes("left left");
        cx.simulate_input(" 🎵 ");
        cx.read_entity(&input, |input, _| assert_eq!(input.text, "Bono 🎵 bo"));
        cx.simulate_keystrokes("cmd-a");
        cx.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string("Бьорк".into())));
        cx.simulate_keystrokes("cmd-v");
        cx.read_entity(&input, |input, _| assert_eq!(input.text, "Бьорк"));
        cx.simulate_keystrokes("backspace");
        cx.read_entity(&input, |input, _| assert_eq!(input.text, "Бьор"));
        cx.simulate_keystrokes("cmd-a");
        cx.simulate_input("e\u{301}");
        cx.simulate_keystrokes("backspace");
        cx.read_entity(&input, |input, _| assert!(input.text.is_empty()));
    }
}
