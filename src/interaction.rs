use crate::{context::TracksCtx, playhead::PlayheadApi};

/// Handle scroll and zoom interactions for the timeline.
pub fn handle_scroll_and_zoom(
    ui: &mut egui::Ui,
    timeline_rect: egui::Rect,
    timeline_api: &mut dyn crate::TimelineApi,
) {
    if ui.rect_contains_pointer(timeline_rect) {
        let ctrl_pressed = ui.input(|i| i.modifiers.ctrl);
        let smooth_delta = ui.input(|i| i.smooth_scroll_delta);
        let raw_delta = ui.input(|i| i.raw_scroll_delta);
        // When Ctrl is pressed, prefer raw_delta for more immediate response
        // Otherwise, prefer smooth_delta for better UX
        let delta = if ctrl_pressed {
            if raw_delta != egui::Vec2::ZERO {
                raw_delta
            } else {
                smooth_delta
            }
        } else {
            if smooth_delta != egui::Vec2::ZERO {
                smooth_delta
            } else {
                raw_delta
            }
        };
        if ctrl_pressed {
            if delta.x != 0.0 || delta.y != 0.0 {
                timeline_api.zoom(delta.y - delta.x);
            }
        } else {
            if delta.x != 0.0 {
                let ticks_per_point = timeline_api.musical_ruler_info().ticks_per_point();
                let shift_amount = delta.x * ticks_per_point;
                let current_start = timeline_api.timeline_start();
                // Only allow scrolling right (positive shift) or scrolling left (negative shift)
                // but clamp to prevent timeline_start from going below 0
                let new_start = (current_start + shift_amount).max(0.0);
                let actual_shift = new_start - current_start;
                if actual_shift != 0.0 {
                    timeline_api.shift_timeline_start(actual_shift);
                }
            }
        }
    }
}

/// Handle clicks and drags on timeline area to set playhead.
pub fn handle_track_playhead_interaction(
    ui: &mut egui::Ui,
    tracks: &TracksCtx,
    playhead_api: Option<&mut dyn PlayheadApi>,
) {
    if let Some(api) = playhead_api {
        let timeline_rect = tracks.timeline.full_rect;
        let timeline_w = timeline_rect.width();
        let ticks_per_point = api.ticks_per_point();
        let visible_ticks = ticks_per_point * timeline_w;

        // Check input state without allocating space (to avoid layout issues)
        let pointer_pressed = ui.input(|i| i.pointer.primary_pressed());
        let pointer_down = ui.input(|i| i.pointer.primary_down());
        let pointer_pos = ui.input(|i| i.pointer.interact_pos());
        let pointer_over = pointer_pos
            .map(|pos| timeline_rect.contains(pos))
            .unwrap_or(false);

        // Handle both initial click and drag
        if (pointer_pressed && pointer_over) || (pointer_down && pointer_over) {
            if let Some(pt) = pointer_pos {
                let tick = (((pt.x - timeline_rect.min.x) / timeline_w) * visible_ticks).max(0.0);
                api.set_playhead_ticks(tick);
            }
        }
    }
}
