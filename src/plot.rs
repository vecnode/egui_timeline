use crate::context::TimelineCtx;
use egui_plot as plot;
use std::{hash::Hash, ops::RangeInclusive};

/// Short-hand for drawing a plot within the timeline UI.
///
/// The same as `egui::plot::Plot::new`, but sets some useful defaults before returning.
pub fn plot_ticks(timeline: &TimelineCtx, id_source: impl Hash, y: RangeInclusive<f32>) -> plot::Plot<'_> {
    let h = 72.0;
    plot::Plot::new(id_source)
        .set_margin_fraction(egui::Vec2::ZERO)
        .show_grid(egui::Vec2b::FALSE)
        .allow_zoom(false)
        .allow_boxed_zoom(false)
        .allow_drag(false)
        .allow_scroll(false)
        .allow_boxed_zoom(false)
        .include_x(0.0)
        .include_x(timeline.visible_ticks)
        .include_y(*y.start())
        .include_y(*y.end())
        .show_x(false)
        .show_y(false)
        .legend(plot::Legend::default().position(plot::Corner::LeftTop))
        .show_background(false)
        .show_axes([false; 2])
        .height(h)
}
