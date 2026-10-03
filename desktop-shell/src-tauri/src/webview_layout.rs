//! Geometry for every movable child WebView. Never compose set_position + set_size:
//! Wry 0.55.1 retains the original parent for child bounds() after reparent on Windows.
//! A size-only write can therefore restore coordinates relative to the former window.
use tauri::{PhysicalPosition, PhysicalSize, Position, Rect, Size, Webview, Window};

pub(crate) fn place(
    view: &Webview,
    position: impl Into<Position>,
    size: impl Into<Size>,
) -> Result<(), String> {
    view.set_bounds(Rect {
        position: position.into(),
        size: size.into(),
    })
    .map_err(|error| error.to_string())
}

pub(crate) fn fill_host(view: &Webview, host: &Window) -> Result<(), String> {
    resize_hosted(
        view,
        host.label(),
        host.inner_size().map_err(|error| error.to_string())?,
    )
}

pub(crate) fn resize_hosted(
    view: &Webview,
    source_host: &str,
    size: PhysicalSize<u32>,
) -> Result<(), String> {
    // Former host events must not move a docked/parked page. A minimized host can
    // report an empty client area; retain the last usable viewport until restore.
    if !accept_resize(view.window().label(), source_host, size) {
        return Ok(());
    }
    place(view, PhysicalPosition::new(0, 0), size)
}

fn accept_resize(current: &str, source: &str, size: PhysicalSize<u32>) -> bool {
    current == source && size.width > 0 && size.height > 0
}

pub(crate) fn park(view: &Webview, x: i32, y: i32) -> Result<(), String> {
    // size() is independent of the old parent's coordinate origin.
    place(
        view,
        PhysicalPosition::new(x, y),
        view.size().map_err(|error| error.to_string())?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn former_host_and_minimized_events_cannot_replace_the_viewport() {
        let normal = PhysicalSize::new(1080, 760);
        assert!(accept_resize("popout", "popout", normal));
        assert!(!accept_resize("main", "popout", normal));
        assert!(!accept_resize("popout", "main", normal));
        assert!(!accept_resize("popout", "popout", PhysicalSize::new(0, 0)));
    }
}
