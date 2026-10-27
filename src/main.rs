mod cache;
mod icons;
mod library;
mod media_controls;
mod playback;
mod search;
mod settings;
mod statistics;
mod ui;

use gpui::{App, AppContext, QuitMode, WindowOptions};
use ui::AppView;

fn main() {
    gpui_platform::application()
        .with_assets(icons::Icons)
        .with_quit_mode(QuitMode::LastWindowClosed)
        .run(|cx: &mut App| {
            cx.open_window(
                WindowOptions {
                    window_min_size: Some(gpui::size(gpui::px(960.), gpui::px(640.))),
                    ..WindowOptions::default()
                },
                |_, cx| cx.new(AppView::new),
            )
            .expect("open main window");
            cx.activate(true);
        });
}
