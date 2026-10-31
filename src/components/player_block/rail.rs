use crate::ui::{Palette, SliderValue};
use gpui::{prelude::*, *};

pub fn rail(
    id: &'static str,
    value: SliderValue,
    enabled: bool,
    p: Palette,
    tooltip_text: Option<String>,
    dragging: std::rc::Rc<std::cell::Cell<bool>>,
    callback: impl Fn(f32, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    use std::{cell::Cell, rc::Rc};
    let SliderValue {
        fraction: value,
        maximum,
        step,
    } = value;
    let fraction_step = if maximum > 0. { step / maximum } else { 0. };
    let bounds = Rc::new(Cell::new(Bounds::<Pixels>::default()));
    let painting = bounds.clone();
    let callback = Rc::new(callback);
    let click_callback = callback.clone();
    let click_dragging = dragging.clone();
    let is_dragging = dragging.get();
    let key_callback = callback.clone();
    div()
        .id(id)
        .debug_selector(move || format!("rail-{id}"))
        .role(Role::Slider)
        .aria_label(if id == "seek" {
            "Playback position"
        } else {
            "Volume"
        })
        .aria_min_numeric_value(0.)
        .aria_max_numeric_value(maximum as f64)
        .aria_numeric_value((value.clamp(0., 1.) * maximum) as f64)
        .aria_numeric_value_step(step as f64)
        .aria_orientation(Orientation::Horizontal)
        .aria_value(
            tooltip_text
                .clone()
                .unwrap_or_else(|| format!("{:.0}%", value * 100.)),
        )
        .relative()
        .group(format!("rail-{id}"))
        .h(px(16.))
        .w_full()
        .when(enabled, |el| {
            el.tab_index(0)
                .focus_visible(move |s| s.bg(rgb(p.raised)))
                .on_key_down(move |event: &KeyDownEvent, window, cx| {
                    let target = match event.keystroke.key.as_str() {
                        "left" | "down" => value - fraction_step,
                        "right" | "up" => value + fraction_step,
                        "home" => 0.,
                        "end" => 1.,
                        _ => return,
                    };
                    key_callback(target.clamp(0., 1.), window, cx);
                    cx.stop_propagation();
                })
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    let b = bounds.get();
                    click_dragging.set(true);
                    if b.size.width > px(0.) {
                        click_callback(
                            ((event.position.x - b.left()) / b.size.width).clamp(0., 1.),
                            window,
                            cx,
                        );
                    }
                    cx.stop_propagation();
                })
        })
        .child(
            canvas(
                move |b, _, _| {
                    painting.set(b);
                },
                move |b, _, window, _| {
                    let y = b.top() + b.size.height / 2. - px(1.5);
                    window.paint_quad(fill(
                        Bounds::new(point(b.left(), y), size(b.size.width, px(3.))),
                        rgb(p.line),
                    ));
                    if enabled && value > 0. {
                        window.paint_quad(fill(
                            Bounds::new(
                                point(b.left(), y),
                                size(b.size.width * value.clamp(0., 1.), px(3.)),
                            ),
                            rgb(p.accent),
                        ));
                    }
                    if enabled {
                        let moving = dragging.clone();
                        let moving_callback = callback.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                            if phase == DispatchPhase::Bubble
                                && moving.get()
                                && b.size.width > px(0.)
                            {
                                moving_callback(
                                    ((event.position.x - b.left()) / b.size.width).clamp(0., 1.),
                                    window,
                                    cx,
                                );
                            }
                        });
                        let released = dragging.clone();
                        let release_callback = callback.clone();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                            if phase == DispatchPhase::Bubble
                                && event.button == MouseButton::Left
                                && released.replace(false)
                                && b.size.width > px(0.)
                            {
                                release_callback(
                                    ((event.position.x - b.left()) / b.size.width).clamp(0., 1.),
                                    window,
                                    cx,
                                );
                                window.refresh();
                            }
                        });
                    }
                },
            )
            .size_full(),
        )
        .when(id == "seek" && enabled, |el| {
            el.child(
                div()
                    .absolute()
                    .left(relative(value.clamp(0., 1.)))
                    .ml(px(-5.))
                    .top(px(3.))
                    .size(px(10.))
                    .group("seek-thumb")
                    .rounded_full()
                    .bg(rgb(p.text))
                    .opacity(if is_dragging { 1. } else { 0. })
                    .group_hover(format!("rail-{id}"), |s| s.opacity(1.))
                    .when_some(tooltip_text, |el, text| {
                        el.child(
                            div()
                                .absolute()
                                .debug_selector(|| "seek-time-tooltip".into())
                                .w(px(54.))
                                .left(px(-22.))
                                .bottom(px(18.))
                                .py_1()
                                .rounded_md()
                                .text_center()
                                .whitespace_nowrap()
                                .bg(rgb(p.raised))
                                .text_color(rgb(p.text))
                                .text_size(px(11.))
                                .opacity(if is_dragging { 1. } else { 0. })
                                .group_hover("seek-thumb", |s| s.opacity(1.))
                                .child(text),
                        )
                    }),
            )
        })
}
