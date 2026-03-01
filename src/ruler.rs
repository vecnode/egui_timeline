use crate::types::Bar;

pub trait MusicalInfo {
    /// The number of ticks per beat, also known as PPQN (parts per quarter note).
    fn ticks_per_beat(&self) -> u32;
    /// The bar at the given tick offset starting from the beginning (left) of the timeline view.
    fn bar_at_ticks(&self, tick: f32) -> Bar;
    /// Affects how "zoomed" the timeline is. By default, uses 16 points per beat.
    fn ticks_per_point(&self) -> f32 {
        self.ticks_per_beat() as f32 / 16.0
    }
    /// Get the current timeline start position in ticks (for calculating absolute bar numbers).
    /// Returns None if not available.
    fn timeline_start(&self) -> Option<f32> {
        None
    }
}

/// Respond to when the user clicks on the ruler.
pub trait MusicalInteract {
    /// The given tick location was clicked
    fn click_at_tick(&mut self, tick: f32);
}

/// The required API for the musical ruler widget.
pub trait MusicalRuler {
    fn info(&self) -> &dyn MusicalInfo;
    fn interact(&mut self) -> &mut dyn MusicalInteract;
}

pub fn musical(ui: &mut egui::Ui, api: &mut dyn MusicalRuler) -> egui::Response {
    // Use fixed height to match track height and prevent overflow
    const RULER_HEIGHT: f32 = 20.0;
    let w = ui.available_rect_before_wrap().width();
    let desired_size = egui::Vec2::new(w, RULER_HEIGHT);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

    let w = rect.width();
    let ticks_per_point = api.info().ticks_per_point();
    let visible_ticks = w * ticks_per_point;
    let pointer_pressed = ui.input(|i| i.pointer.primary_pressed());
    let pointer_over = ui.input(|i| {
        i.pointer.hover_pos()
            .map(|pos| rect.contains(pos))
            .unwrap_or(false)
    });
    if (pointer_pressed && pointer_over) || response.dragged() {
        if let Some(pt) = response.interact_pointer_pos() {
            let tick = (((pt.x - rect.min.x) / w) * visible_ticks).max(0.0);
            api.interact().click_at_tick(tick);
            response.mark_changed();
        }
    }

    let vis = ui.style().noninteractive();
    // Note: Pink border is drawn by the track's show() method to include header + timeline
    // No need to draw border here as it would only cover the timeline area

    let mut stroke = vis.fg_stroke;
    let bar_color = stroke.color.linear_multiply(0.5);
    let step_color = stroke.color.linear_multiply(0.125);
    let bar_y = rect.center().y;
    let step_even_y = rect.top() + rect.height() * 0.25;
    let step_odd_y = rect.top() + rect.height() * 0.125;

    let visible_len = w;
    let info = api.info();
    let mut steps = Steps::new(info, visible_len, crate::types::MIN_STEP_GAP);
    let mut last_bar_start: Option<f32> = None;
    
    while let Some(step) = steps.next(info) {
        let (y, color) = match step.index_in_bar {
            0 => (bar_y, bar_color),
            n if n % 2 == 0 => (step_even_y, step_color),
            _ => (step_odd_y, step_color),
        };
        stroke.color = color;
        let x = rect.left() + step.x;
        let a = egui::Pos2::new(x, rect.top());
        let b = egui::Pos2::new(x, y);
        ui.painter().line_segment([a, b], stroke);
        
        if step.index_in_bar == 0 {
            let bar = info.bar_at_ticks(step.ticks);
            let is_new_bar = match last_bar_start {
                Some(last_start) => (bar.tick_range.start - last_start).abs() > 0.1,
                None => true,
            };
            
            if is_new_bar {
                last_bar_start = Some(bar.tick_range.start);
                let ticks_per_bar = bar.tick_range.end - bar.tick_range.start;
                if ticks_per_bar > 0.0 {
                    let bar_number = if let Some(timeline_start) = info.timeline_start() {
                        let absolute_tick = timeline_start + step.ticks;
                        (absolute_tick / ticks_per_bar).floor() as u32
                    } else {
                        let estimated_bar = (step.ticks / ticks_per_bar).floor();
                        let estimated_timeline_start = (estimated_bar * ticks_per_bar) - bar.tick_range.start;
                        let absolute_tick = estimated_timeline_start + step.ticks;
                        (absolute_tick / ticks_per_bar).floor() as u32
                    };
                    let bar_number = bar_number.min(500);
                    
                    const MIN_LEFT_MARGIN: f32 = 20.0;
                    const MIN_RIGHT_MARGIN: f32 = 30.0;
                    let text = format!("{}", bar_number);
                    let estimated_text_width = text.len() as f32 * 6.0;
                    let fits_left = x >= rect.left() + MIN_LEFT_MARGIN;
                    let fits_right = x + estimated_text_width <= rect.right() - MIN_RIGHT_MARGIN;
                    
                    if fits_left && fits_right {
                        let text_color = vis.fg_stroke.color;
                        let text_pos = egui::Pos2::new(x + 2.0, rect.center().y);
                        let default_font_size = ui.style().text_styles.get(&egui::TextStyle::Body)
                            .map(|f| f.size)
                            .unwrap_or(14.0);
                        let small_font = egui::FontId::new(default_font_size * 0.75, egui::FontFamily::Proportional);
                        ui.painter().text(text_pos, egui::Align2::LEFT_CENTER, text, small_font, text_color);
                    }
                }
            }
        }
    }

    response
}

#[derive(Copy, Clone, Debug)]
pub struct Step {
    /// The index of the step within the bar.
    ///
    /// The first step always indicates the start of the bar.
    pub index_in_bar: usize,
    /// The position of the step in ticks from the beginning of the start of the visible area.
    pub ticks: f32,
    /// The location of the step along the x axis from the start of the ruler.
    pub x: f32,
}

#[derive(Clone, Debug)]
pub struct Steps {
    ticks_per_beat: f32,
    ticks_per_point: f32,
    visible_ticks: f32,
    min_step_ticks: f32,
    index_in_bar: usize,
    step_ticks: f32,
    bar: Bar,
    ticks: f32,
}

impl Steps {
    /// Create a new `Steps`.
    pub fn new(api: &dyn MusicalInfo, visible_len: f32, min_step_gap: f32) -> Self {
        let ticks_per_beat = api.ticks_per_beat() as f32;
        let ticks_per_point = api.ticks_per_point();
        let visible_ticks = ticks_per_point * visible_len;
        let min_step_ticks = ticks_per_point * min_step_gap;
        Self {
            ticks_per_beat,
            ticks_per_point,
            visible_ticks,
            min_step_ticks,
            index_in_bar: 0,
            step_ticks: 0.0,
            bar: api.bar_at_ticks(0.0),
            ticks: 0.0,
        }
    }

    /// Produce the next `Step`.
    pub fn next(&mut self, api: &dyn MusicalInfo) -> Option<Step> {
        'bars: loop {
            // If this is the first step of the bar, update step interval.
            if self.index_in_bar == 0 {
                self.ticks = self.bar.tick_range.start;
                let mut beat_subdivs = self.bar.time_sig.bottom / 4;
                self.step_ticks = self.ticks_per_beat as f32 / beat_subdivs as f32;
                if self.step_ticks >= self.min_step_ticks {
                    loop {
                        let new_beat_subdivs = beat_subdivs * 2;
                        let new_step_ticks = self.ticks_per_beat as f32 / new_beat_subdivs as f32;
                        if new_step_ticks <= self.min_step_ticks {
                            break;
                        }
                        beat_subdivs = new_beat_subdivs;
                        self.step_ticks = new_step_ticks;
                    }
                } else {
                    self.step_ticks = self.bar.tick_range.end - self.bar.tick_range.start;
                }
            }

            'ticks: loop {
                if self.ticks > self.visible_ticks {
                    return None;
                }
                if self.ticks >= self.bar.tick_range.end {
                    self.index_in_bar = 0;
                    self.bar = api.bar_at_ticks(self.bar.tick_range.end + 0.5);
                    continue 'bars;
                }
                let index_in_bar = self.index_in_bar;
                let ticks = self.ticks;
                self.index_in_bar += 1;
                self.ticks += self.step_ticks;
                if ticks < 0.0 {
                    continue 'ticks;
                }
                let x = ticks / self.ticks_per_point;
                let step = Step {
                    index_in_bar,
                    ticks,
                    x,
                };
                return Some(step);
            }
        }
    }
}
