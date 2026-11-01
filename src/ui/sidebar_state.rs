use super::*;
use gpui::prelude::*;
use std::time::Duration;

impl AppView {
    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        let start = if self.sidebar_width > 0. {
            self.sidebar_width
        } else {
            184.
        };
        let target = if self.sidebar_collapsed { 64. } else { 184. };
        self.sidebar_animation = Some(cx.spawn(async move |view, cx| {
            let started = std::time::Instant::now();
            loop {
                let t = (started.elapsed().as_secs_f32() / 0.18).min(1.);
                let eased = 1. - (1. - t).powi(3);
                if view
                    .update(cx, |this, cx| {
                        this.sidebar_width = start + (target - start) * eased;
                        cx.notify();
                    })
                    .is_err()
                    || t >= 1.
                {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
            }
        }));
        cx.notify();
    }
}
