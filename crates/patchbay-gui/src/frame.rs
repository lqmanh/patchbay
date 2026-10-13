use gpui_kit::*;
use serde::{Deserialize, Serialize};

use patchbay_core::json::write_json;
use patchbay_core::paths::Paths;

/// Where the main window sits. gpui gives window bounds relative to their
/// display, so the display goes with them, by UUID: display ids can change
/// across reboots and reconnects.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowFrame {
    bounds: WindowBounds,
    display: String,
}

/// `WindowFrame` as saved in the window state file.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Saved {
    display: String,
    state: State,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum State {
    Windowed,
    Maximized,
    Fullscreen,
}

impl WindowFrame {
    pub fn of(window: &Window, cx: &App) -> Option<Self> {
        let display = window.display(cx)?.uuid().ok()?.to_string();
        Some(Self {
            bounds: window.window_bounds(),
            display,
        })
    }

    /// The bounds and display to open a window with, unless the display is
    /// gone or would no longer show the title bar.
    pub fn placement(&self, cx: &App) -> Option<(WindowBounds, DisplayId)> {
        let display = cx.displays().into_iter().find(|display| {
            display
                .uuid()
                .is_ok_and(|uuid| uuid.to_string() == self.display)
        })?;
        display
            .visible_bounds()
            .contains(&self.bounds.get_bounds().top_center())
            .then_some((self.bounds, display.id()))
    }

    pub fn set_bounds(&mut self, bounds: WindowBounds) {
        self.bounds = bounds;
    }

    pub fn load(paths: &Paths) -> Option<Self> {
        let text = std::fs::read_to_string(paths.window_state_file()).ok()?;
        let saved: Saved = serde_json::from_str(&text).ok()?;
        if !(saved.width > 0. && saved.height > 0.) {
            return None;
        }
        let bounds = Bounds::new(
            point(px(saved.x), px(saved.y)),
            size(px(saved.width), px(saved.height)),
        );
        let bounds = match saved.state {
            State::Windowed => WindowBounds::Windowed(bounds),
            State::Maximized => WindowBounds::Maximized(bounds),
            State::Fullscreen => WindowBounds::Fullscreen(bounds),
        };
        Some(Self {
            bounds,
            display: saved.display,
        })
    }

    pub fn save(&self, paths: &Paths) -> anyhow::Result<()> {
        let (state, bounds) = match self.bounds {
            WindowBounds::Windowed(bounds) => (State::Windowed, bounds),
            WindowBounds::Maximized(bounds) => (State::Maximized, bounds),
            WindowBounds::Fullscreen(bounds) => (State::Fullscreen, bounds),
        };
        let saved = Saved {
            display: self.display.clone(),
            state,
            x: bounds.origin.x.as_f32(),
            y: bounds.origin.y.as_f32(),
            width: bounds.size.width.as_f32(),
            height: bounds.size.height.as_f32(),
        };
        write_json(&paths.window_state_file(), &saved)
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Bounds, WindowBounds, point, px, size};
    use patchbay_core::paths::Paths;

    use super::WindowFrame;

    #[test]
    fn saves_and_loads() {
        let home = tempfile::tempdir().unwrap();
        let paths = Paths::new(home.path(), |_| None);
        assert_eq!(WindowFrame::load(&paths), None);

        let frame = WindowFrame {
            bounds: WindowBounds::Maximized(Bounds::new(
                point(px(12.), px(34.)),
                size(px(1000.), px(680.)),
            )),
            display: "69CE91EC-085A-0B18-0857-D6964E3302DB".into(),
        };
        frame.save(&paths).unwrap();
        assert_eq!(WindowFrame::load(&paths), Some(frame));
    }
}
