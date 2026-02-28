use crate::{context::TimelineCtx, ruler, types::MIN_STEP_GAP};

/// Paints the grid over the timeline `Rect`.
///
/// If using a custom `background`, you may wish to call this after.
///
/// The grid is positioned so that tick 0 always aligns with the left edge of the timeline area
/// (where the header ends), keeping it "glued" to the left edge.
pub fn paint_grid(ui: &mut egui::Ui, timeline: &TimelineCtx, info: &dyn ruler::MusicalInfo) {
    let vis = ui.style().noninteractive();
    let mut stroke = vis.bg_stroke;
    let bar_color = stroke.color.linear_multiply(0.5);
    let step_even_color = stroke.color.linear_multiply(0.25);
    let step_odd_color = stroke.color.linear_multiply(0.125);
    let tl_rect = timeline.full_rect;
    let visible_len = tl_rect.width();
    let mut steps = ruler::Steps::new(info, visible_len, MIN_STEP_GAP);
    while let Some(step) = steps.next(info) {
        stroke.color = match step.index_in_bar {
            0 => bar_color,
            n if n % 2 == 0 => step_even_color,
            _ => step_odd_color,
        };
        // Position grid lines relative to tl_rect.left() (timeline's left edge).
        // When step.x = 0 (tick 0), the line is at tl_rect.left(), keeping it glued to the left edge.
        let x = tl_rect.left() + step.x;
        let a = egui::Pos2::new(x, tl_rect.top());
        let b = egui::Pos2::new(x, tl_rect.bottom());
        ui.painter().line_segment([a, b], stroke);
    }
}
