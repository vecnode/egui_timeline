use egui_timeline::{
    playhead::{Info, Interaction, Playhead, PlayheadApi},
    ruler::{musical, MusicalInfo, MusicalInteract, MusicalRuler},
    Bar, TimeSig, Timeline, TimelineApi, TrackSelectionApi,
};
use std::ops::Range;
use std::collections::HashMap;
use std::cell::RefCell;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egui_timeline Demo",
        options,
        Box::new(|_cc| Ok(Box::new(TimelineApp::default()) as Box<dyn eframe::App>)),
    )
}

struct TimelineApp {
    timeline_start: f32,
    zoom_level: f32,
    playhead_pos: RefCell<f32>,
    ticks_per_beat: u32,
    global_panel_visible: bool,
    track_selections: RefCell<HashMap<String, (f32, f32)>>, // track_id -> (start_tick, end_tick)
    drag_start_tick: RefCell<Option<(String, f32)>>, // (track_id, start_tick) when dragging
    track_names: RefCell<HashMap<String, String>>, // track_id -> track_name
}

impl TimelineApp {
    /// Total number of bars (0-500 inclusive = 501 bars)
    const TOTAL_BARS: u32 = 501;
    
    /// Calculate ticks per bar
    fn ticks_per_bar(&self) -> f32 {
        let beats_per_bar = 4.0; // 4/4 time signature
        self.ticks_per_beat as f32 * beats_per_bar
    }
}

impl Default for TimelineApp {
    fn default() -> Self {
        Self {
            timeline_start: 0.0,
            zoom_level: 1.0,
            playhead_pos: RefCell::new(0.0),
            ticks_per_beat: 960, // Standard MIDI PPQN
            global_panel_visible: false,
            track_selections: RefCell::new(HashMap::new()),
            drag_start_tick: RefCell::new(None),
            track_names: RefCell::new({
                let mut names = HashMap::new();
                names.insert("track1".to_string(), "Track 1".to_string());
                names.insert("track2".to_string(), "Track 2".to_string());
                names
            }),
        }
    }
}

impl TimelineApi for TimelineApp {
    fn musical_ruler_info(&self) -> &dyn MusicalInfo {
        self
    }

    fn timeline_start(&self) -> f32 {
        self.timeline_start
    }

    fn shift_timeline_start(&mut self, ticks: f32) {
        // Apply the shift - clamping is handled in the interaction handler
        // where we have access to the visible width to calculate proper max
        self.timeline_start += ticks;
    }

    fn zoom(&mut self, y_delta: f32) {
        self.zoom_level = (self.zoom_level * (1.0 + y_delta * 0.01)).max(0.1).min(10.0);
    }
}

impl MusicalInfo for TimelineApp {
    fn ticks_per_beat(&self) -> u32 {
        self.ticks_per_beat
    }

    fn timeline_start(&self) -> Option<f32> {
        Some(self.timeline_start)
    }

    fn bar_at_ticks(&self, tick: f32) -> Bar {
        let absolute_tick = self.timeline_start + tick;
        let ticks_per_bar = self.ticks_per_bar();
        let mut bar_number = (absolute_tick / ticks_per_bar).floor() as u32;
        
        // Clamp bar number to 0-500
        bar_number = bar_number.min(Self::TOTAL_BARS - 1);
        
        let bar_start = bar_number as f32 * ticks_per_bar;
        let bar_end = bar_start + ticks_per_bar;
        Bar {
            tick_range: Range {
                start: bar_start - self.timeline_start,
                end: bar_end - self.timeline_start,
            },
            time_sig: TimeSig { top: 4, bottom: 4 },
        }
    }

    fn ticks_per_point(&self) -> f32 {
        (self.ticks_per_beat as f32 / 16.0) * self.zoom_level
    }
}

impl MusicalInteract for TimelineApp {
    fn click_at_tick(&mut self, tick: f32) {
        *self.playhead_pos.borrow_mut() = self.timeline_start + tick;
    }
}

impl MusicalRuler for TimelineApp {
    fn info(&self) -> &dyn MusicalInfo {
        self
    }

    fn interact(&mut self) -> &mut dyn MusicalInteract {
        self
    }
}

impl Info for TimelineApp {
    fn playhead_ticks(&self) -> f32 {
        *self.playhead_pos.borrow() - self.timeline_start
    }
}

impl Interaction for TimelineApp {
    fn set_playhead_ticks(&self, ticks: f32) {
        *self.playhead_pos.borrow_mut() = self.timeline_start + ticks;
    }
}

impl TrackSelectionApi for TimelineApp {
    fn ticks_per_point(&self) -> f32 {
        (self.ticks_per_beat as f32 / 16.0) * self.zoom_level
    }

    fn timeline_start(&self) -> f32 {
        self.timeline_start
    }

    fn start_selection_drag(&self, track_id: &str, start_tick: f32) {
        *self.drag_start_tick.borrow_mut() = Some((track_id.to_string(), start_tick));
    }

    fn update_selection_drag(&self, track_id: &str, end_tick: f32) {
        if let Some((drag_track_id, start_tick)) = self.drag_start_tick.borrow().as_ref() {
            if drag_track_id == track_id {
                let start = start_tick.min(end_tick);
                let end = start_tick.max(end_tick);
                self.track_selections.borrow_mut().insert(track_id.to_string(), (start, end));
            }
        }
    }

    fn get_drag_start(&self) -> Option<(String, f32)> {
        self.drag_start_tick.borrow().clone()
    }

    fn end_selection_drag(&self) {
        *self.drag_start_tick.borrow_mut() = None;
    }

    fn set_selection(&self, track_id: &str, start_tick: f32, end_tick: f32) {
        self.track_selections.borrow_mut().insert(track_id.to_string(), (start_tick, end_tick));
    }

    fn clear_selection(&self, track_id: &str) {
        self.track_selections.borrow_mut().remove(track_id);
    }

    fn clear_all_selections(&self) {
        self.track_selections.borrow_mut().clear();
    }

    fn get_selection(&self, track_id: &str) -> Option<(f32, f32)> {
        self.track_selections.borrow().get(track_id).copied()
    }

    fn get_selected_track_id(&self) -> Option<String> {
        self.track_selections.borrow().keys().next().cloned()
    }
}

impl eframe::App for TimelineApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("egui_timeline Demo");
                ui.separator();
            });

            ui.add_space(10.0);

            // Create and show the timeline
            let timeline = Timeline::new().header(150.0);
            let show = timeline.show(ui, self);

            show.paint_grid(self)
                .pinned_tracks(|tracks, ui| {
                    // Ruler track
                    tracks.next(ui).header(|ui| {
                        ui.label("Ruler");
                    }).show(
                        |_timeline, ui| {
                            musical(ui, self);
                        },
                        None,
                        None,
                    );
                })
                .tracks(
                    |tracks, _viewport, ui, playhead_api, selection_api| {
                    // Example track 1
                    let track1_name = self.track_names.borrow().get("track1").cloned().unwrap_or_else(|| "Track 1".to_string());
                    tracks.next(ui)
                        .with_id("track1")
                        .header(|ui| {
                            ui.add_space(2.0); // Top padding
                            let available_width = ui.available_width();
                            let mut name = track1_name.clone();
                            
                            // Create TextEdit with frame disabled so it doesn't draw its own background
                            let mut text_edit = egui::TextEdit::singleline(&mut name);
                            text_edit = text_edit.desired_width(available_width * 0.5);
                            text_edit = text_edit.frame(false); // Disable TextEdit's frame/background
                            
                            // Get the natural height that TextEdit would use
                            let text_height = ui.text_style_height(&egui::TextStyle::Body);
                            let input_size = egui::Vec2::new(available_width * 0.5, text_height + 4.0);
                            
                            // Allocate space and draw background (no border radius - 0.0)
                            let (rect, _response) = ui.allocate_exact_size(input_size, egui::Sense::click());
                            let dark_grey = egui::Color32::from_rgb(50, 50, 50);
                            ui.painter().rect_filled(rect, 3.0, dark_grey);
                            
                            // Add TextEdit on top
                            let text_response = ui.put(rect, text_edit);
                            
                            if text_response.changed() {
                                self.track_names.borrow_mut().insert("track1".to_string(), name);
                            }
                        })
                            .show(
                                |timeline, ui| {
                                    let plot = timeline.plot_ticks("track1", 0.0..=1.0);
                                    plot.show(ui, |plot_ui| {
                                        // Add some example points
                                        let points: Vec<[f64; 2]> = (0..10)
                                            .map(|i| {
                                                [
                                                    (i as f64 * timeline.visible_ticks() as f64 / 10.0),
                                                    (i as f64 % 3.0) / 3.0,
                                                ]
                                            })
                                            .collect();
                                        let line = egui_plot::Line::new(points);
                                        plot_ui.line(line);
                                    });
                                },
                                playhead_api,
                                selection_api,
                            );

                    // Example track 2
                    let track2_name = self.track_names.borrow().get("track2").cloned().unwrap_or_else(|| "Track 2".to_string());
                    tracks.next(ui)
                        .with_id("track2")
                        .header(|ui| {
                            ui.add_space(2.0); // Top padding
                            let available_width = ui.available_width();
                            let mut name = track2_name.clone();
                            
                            // Create TextEdit with frame disabled so it doesn't draw its own background
                            let mut text_edit = egui::TextEdit::singleline(&mut name);
                            text_edit = text_edit.desired_width(available_width * 0.5);
                            text_edit = text_edit.frame(false); // Disable TextEdit's frame/background
                            
                            // Get the natural height that TextEdit would use
                            let text_height = ui.text_style_height(&egui::TextStyle::Body);
                            let input_size = egui::Vec2::new(available_width * 0.5, text_height + 4.0);
                            
                            // Allocate space and draw background (no border radius - 0.0)
                            let (rect, _response) = ui.allocate_exact_size(input_size, egui::Sense::click());
                            let dark_grey = egui::Color32::from_rgb(50, 50, 50);
                            ui.painter().rect_filled(rect, 3.0, dark_grey);
                            
                            // Add TextEdit on top
                            let text_response = ui.put(rect, text_edit);
                            
                            if text_response.changed() {
                                self.track_names.borrow_mut().insert("track2".to_string(), name);
                            }
                        })
                            .show(
                                |timeline, ui| {
                                    let plot = timeline.plot_ticks("track2", 0.0..=1.0);
                                    plot.show(ui, |plot_ui| {
                                        let points: Vec<[f64; 2]> = (0..15)
                                            .map(|i| {
                                                [
                                                    (i as f64 * timeline.visible_ticks() as f64 / 15.0),
                                                    ((i as f64 * 0.7) % 1.0),
                                                ]
                                            })
                                            .collect();
                                        let line = egui_plot::Line::new(points);
                                        plot_ui.line(line);
                                    });
                                },
                                playhead_api,
                                selection_api,
                            );
                    },
                    Some(self as &dyn PlayheadApi),
                    Some(self as &dyn TrackSelectionApi),
                )
                .playhead(ui, self, Playhead::new())
                .top_panel_time(ui, Some(self as &dyn PlayheadApi))
                .bottom_bar(ui, &mut self.global_panel_visible);

            ui.add_space(10.0);
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(format!("Timeline Start: {:.1} ticks", self.timeline_start));
                ui.label(format!("Zoom: {:.2}x", self.zoom_level));
                ui.label(format!("Playhead: {:.1} ticks", *self.playhead_pos.borrow()));
            });
            ui.label("Scroll horizontally to move timeline, Ctrl+Scroll to zoom, Click ruler to set playhead");
        });
    }
}

