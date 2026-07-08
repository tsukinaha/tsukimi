use gtk::{
    PolicyType,
    ScrolledWindow,
    graphene::Point,
    prelude::*,
};

use serde_json;

pub trait ScrolledWindowFixExt {
    fn fix(&self) -> &Self;
}

/// Scroll a child widget so it sits near the vertical center of the nearest
/// ancestor `ScrolledWindow` with a vertical scrollbar.
pub fn scroll_widget_to_row_center(widget: &impl IsA<gtk::Widget>) {
    let Some(scrolled) = widget
        .ancestor(ScrolledWindow::static_type())
        .and_downcast::<ScrolledWindow>()
    else {
        return;
    };
    if scrolled.vscrollbar_policy() == PolicyType::Never {
        return;
    }
    scroll_widget_centered_in(&scrolled, widget);
}

fn scroll_widget_centered_in(scrolled: &ScrolledWindow, widget: &impl IsA<gtk::Widget>) {
    let Some(content) = scrolled.child() else {
        return;
    };
    let point = Point::new(0.0, 0.0);
    let Some(translated) = widget.compute_point(&content, &point) else {
        return;
    };
    let y = f64::from(translated.y());
    let height = widget.height().max(1);
    let adj = scrolled.vadjustment();
    let page = adj.page_size();
    if page <= 0.0 {
        return;
    }
    let max = (adj.upper() - page).max(0.0);
    let center_target = y + f64::from(height) * 0.5 - page * 0.5;
    let target = center_target.clamp(0.0, max);
    // #region agent log
    {
        use std::io::Write;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let line = serde_json::json!({
            "sessionId": "ef5d72",
            "hypothesisId": "A",
            "location": "fix.rs:scroll_widget_centered_in",
            "message": "scroll clamp",
            "data": {
                "y": y,
                "height": height,
                "centerTarget": center_target,
                "target": target,
                "max": max,
                "adjValue": adj.value(),
                "adjUpper": adj.upper(),
                "clamped": center_target > max
            },
            "timestamp": ts,
            "runId": "post-fix"
        });
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/var/mnt/SSD/Atlas Commons/technitiumdns-api/.cursor/debug-ef5d72.log")
        {
            let _ = writeln!(f, "{line}");
        }
    }
    // #endregion
    adj.set_value(target);
}

/// Keep a widget fully visible inside the nearest vertical `ScrolledWindow`.
#[allow(dead_code)]
pub fn scroll_widget_into_viewport(widget: &impl IsA<gtk::Widget>) {
    let Some(scrolled) = widget
        .ancestor(ScrolledWindow::static_type())
        .and_downcast::<ScrolledWindow>()
    else {
        return;
    };
    if scrolled.vscrollbar_policy() == PolicyType::Never {
        return;
    }
    let Some(content) = scrolled.child() else {
        return;
    };
    let point = Point::new(0.0, 0.0);
    let Some(translated) = widget.compute_point(&content, &point) else {
        return;
    };
    let y = f64::from(translated.y());
    let height = f64::from(widget.height().max(1));
    let adj = scrolled.vadjustment();
    let page = adj.page_size();
    if page <= 0.0 {
        return;
    }
    let max = (adj.upper() - page).max(0.0);
    let margin = 56.0;
    let top = y;
    let bottom = y + height;
    let view_top = adj.value();
    let view_bottom = view_top + page;

    if bottom > view_bottom - margin {
        adj.set_value((bottom - page + margin).clamp(0.0, max));
    } else if top < view_top + margin {
        adj.set_value((top - margin).clamp(0.0, max));
    }
}

/// Scroll a child widget so it sits near the horizontal center of the nearest
/// ancestor `ScrolledWindow` with a horizontal scrollbar.
pub fn scroll_widget_to_column_center(widget: &impl IsA<gtk::Widget>) {
    let Some(scrolled) = widget
        .ancestor(ScrolledWindow::static_type())
        .and_downcast::<ScrolledWindow>()
    else {
        return;
    };
    if scrolled.hscrollbar_policy() == PolicyType::Never {
        return;
    }
    let Some(content) = scrolled.child() else {
        return;
    };
    let point = Point::new(0.0, 0.0);
    let Some(translated) = widget.compute_point(&content, &point) else {
        return;
    };
    let x = f64::from(translated.x());
    let width = f64::from(widget.width().max(1));
    let adj = scrolled.hadjustment();
    let page = adj.page_size();
    if page <= 0.0 {
        return;
    }
    let max = (adj.upper() - page).max(0.0);
    let target = (x + width * 0.5 - page * 0.5).clamp(0.0, max);
    adj.set_value(target);
}

/// fix scrolledwindow fucking up the vscroll event
impl ScrolledWindowFixExt for ScrolledWindow {
    fn fix(&self) -> &Self {
        for object in self.observe_controllers().into_iter() {
            if let Some(controller) = object.ok().and_downcast_ref::<gtk::EventControllerScroll>() {
                controller.set_flags(
                    gtk::EventControllerScrollFlags::HORIZONTAL
                        | gtk::EventControllerScrollFlags::KINETIC,
                );
            }
        }
        self
    }
}
