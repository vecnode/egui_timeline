use crate::{
    context::{BackgroundCtx, SetPlayhead, TimelineCtx, TracksCtx},
    grid, interaction, playhead::PlayheadApi, ruler,
};

/// The top-level timeline widget.
pub struct Timeline {
    /// A optional side panel with track headers.
    ///
    /// Can be useful for labelling tracks or providing convenient volume, mute, solo, etc style
    /// widgets.
    header: Option<f32>,
}

/// The result of setting the timeline, ready to start laying out tracks.
pub struct Show {
    tracks: TracksCtx,
    ui: egui::Ui,
    bottom_bar_rect: Option<egui::Rect>,
    top_panel_rect: Option<egui::Rect>,
}

impl Timeline {
    /// Begin building the timeline widget.
    pub fn new() -> Self {
        Self { header: None }
    }

    /// A optional track header side panel.
    ///
    /// Can be useful for labelling tracks or providing convenient volume, mute, solo, etc style
    /// widgets.
    pub fn header(mut self, width: f32) -> Self {
        self.header = Some(width);
        self
    }

    /// Set the timeline within the currently available rect.
    pub fn show(self, ui: &mut egui::Ui, timeline: &mut dyn crate::TimelineApi) -> Show {
        // The full area including both headers and timeline.
        let full_rect = ui.available_rect_before_wrap();
        
        // Reserve 40px at the top for the top panel and 20px at the bottom for the bottom bar
        const TOP_PANEL_HEIGHT: f32 = 40.0;
        const BOTTOM_BAR_HEIGHT: f32 = 20.0;
        let mut content_rect = full_rect;
        content_rect.min.y += TOP_PANEL_HEIGHT;
        content_rect.set_height(full_rect.height() - TOP_PANEL_HEIGHT - BOTTOM_BAR_HEIGHT);
        
        // Top panel area (40px height, full width)
        let top_panel_rect = egui::Rect::from_min_max(
            egui::Pos2::new(full_rect.min.x, full_rect.min.y),
            egui::Pos2::new(full_rect.max.x, full_rect.min.y + TOP_PANEL_HEIGHT),
        );
        
        // The area occupied by the timeline (excluding top panel and bottom bar).
        let mut timeline_rect = content_rect;
        // The area occupied by track headers.
        let header_rect = self.header.map(|header_w| {
            let mut r = content_rect;
            r.set_width(header_w);
            timeline_rect.min.x = r.right();
            r
        });
        
        // Bottom bar area (20px height, full width)
        let bottom_bar_rect = egui::Rect::from_min_max(
            egui::Pos2::new(full_rect.min.x, content_rect.max.y),
            egui::Pos2::new(full_rect.max.x, full_rect.max.y),
        );

        // Handle scroll and zoom interactions
        interaction::handle_scroll_and_zoom(ui, timeline_rect, timeline);

        // Draw the background.
        let vis = ui.style().noninteractive();
        let bg_stroke = egui::Stroke {
            width: 0.0,
            ..vis.bg_stroke
        };
        ui.painter().rect(full_rect, 0.0, vis.bg_fill, bg_stroke);

        // Draw top panel background (no border)
        let vis = ui.style().noninteractive();
        ui.painter().rect(top_panel_rect, 0.0, vis.bg_fill, vis.bg_stroke);
        
        // Draw a 1px green border around the entire timeline widget (including header column, top panel, and bottom bar)
        // to visualize the complete viewport
        let green_border = egui::Stroke {
            width: 1.0,
            color: egui::Color32::from_rgb(0, 255, 0),
        };
        // full_rect includes the top panel and bottom bar area, so the border will encompass everything
        ui.painter().rect_stroke(full_rect, 0.0, green_border);

        // The child widgets (content area, excluding bottom bar).
        let layout = egui::Layout::top_down(egui::Align::Min);
        let info = timeline.musical_ruler_info();
        let visible_ticks = info.ticks_per_point() * timeline_rect.width();
        let timeline_ctx = TimelineCtx::new(timeline_rect, visible_ticks);
        let tracks = TracksCtx::new(content_rect, header_rect, timeline_ctx);
        let ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect).layout(layout));
        Show { tracks, ui, bottom_bar_rect: Some(bottom_bar_rect), top_panel_rect: Some(top_panel_rect) }
    }
}

impl Show {
    /// Allows for drawing some widgets in the background before showing the grid.
    ///
    /// Can be useful for subtly colouring different ranges, etc.
    pub fn background(mut self, background: impl FnOnce(&BackgroundCtx, &mut egui::Ui)) -> Self {
        let Show {
            ref mut ui,
            ref tracks,
            bottom_bar_rect: _,
            top_panel_rect: _,
        } = self;
        let bg = BackgroundCtx {
            header_full_rect: tracks.header_full_rect,
            timeline: &tracks.timeline,
        };
        background(&bg, ui);
        self
    }

    /// Paints the grid over the timeline `Rect`.
    ///
    /// If using a custom `background`, you may wish to call this after.
    pub fn paint_grid(mut self, info: &dyn ruler::MusicalInfo) -> Self {
        grid::paint_grid(&mut self.ui, &self.tracks.timeline, info);
        self
    }

    /// Set some tracks that should be pinned to the top.
    ///
    /// Often useful for the ruler or other tracks that should always be visible.
    pub fn pinned_tracks(mut self, tracks_fn: impl FnOnce(&TracksCtx, &mut egui::Ui)) -> Self {
        let Self {
            ref mut ui,
            ref tracks,
            bottom_bar_rect: _,
            top_panel_rect: _,
        } = self;

        // Use no spacing by default so we can get exact position for line separator.
        ui.scope(|ui| tracks_fn(tracks, ui));

        // Draw a line to mark end of the pinned tracks.
        let remaining = ui.available_rect_before_wrap();
        let a = remaining.left_top();
        let b = remaining.right_top();
        let stroke = ui.style().visuals.noninteractive().bg_stroke;
        ui.painter().line_segment([a, b], stroke);

        // Add the exact space so the UI is aware.
        ui.add_space(stroke.width);

        // Return to default spacing.
        let rect = ui.available_rect_before_wrap();
        self.ui.set_clip_rect(rect);
        self
    }

    /// Set all remaining tracks for the timeline.
    ///
    /// These tracks will become vertically scrollable in the case that there are two many to fit
    /// on the view. The given `egui::Rect` is the viewport (visible area) relative to the
    /// timeline.
    ///
    /// If `playhead_api` is provided, clicking and dragging on the timeline area of tracks will set the playhead position.
    /// If `selection_api` is provided, clicking and dragging on tracks will create selections.
    pub fn tracks(
        mut self,
        tracks_fn: impl FnOnce(&TracksCtx, egui::Rect, &mut egui::Ui, Option<&dyn PlayheadApi>, Option<&dyn crate::interaction::TrackSelectionApi>),
        playhead_api: Option<&dyn PlayheadApi>,
        selection_api: Option<&dyn crate::interaction::TrackSelectionApi>,
    ) -> SetPlayhead {
        let Self {
            ref mut ui,
            ref tracks,
            bottom_bar_rect,
            top_panel_rect: _,
        } = self;
        let rect = ui.available_rect_before_wrap();
        let enable_scrolling = !ui.input(|i| i.modifiers.ctrl);
        let res = egui::ScrollArea::vertical()
            .max_height(rect.height())
            .enable_scrolling(enable_scrolling)
            .animated(true)
            .stick_to_bottom(true) // stick to new tracks as they're added
            .show_viewport(ui, |ui, view| {
                tracks_fn(tracks, view, ui, playhead_api, selection_api);
            });
        let timeline_rect = tracks.timeline.full_rect;
        let tracks_bottom = res
            .inner_rect
            .bottom()
            .min(res.inner_rect.top() + res.content_size.y);
        let mut set_playhead = SetPlayhead::new(timeline_rect, tracks_bottom);
        set_playhead.bottom_bar_rect = bottom_bar_rect;
        set_playhead.top_panel_rect = self.top_panel_rect;
        set_playhead
    }
}

impl SetPlayhead {
    /// Instantiate the playhead over the top of the whole timeline.
    pub fn playhead(
        &self,
        ui: &mut egui::Ui,
        info: &mut dyn PlayheadApi,
        playhead: crate::playhead::Playhead,
    ) -> &Self {
        crate::playhead::set(ui, info, self.timeline_rect(), self.tracks_bottom(), playhead);
        self
    }

    /// Display time in the top panel.
    /// 
    /// `playhead_api` should provide access to the current playhead position.
    pub fn top_panel_time(&self, ui: &mut egui::Ui, playhead_api: Option<&dyn crate::playhead::PlayheadApi>) -> &Self {
        if let Some(top_panel_rect) = self.top_panel_rect {
            // Create UI for top panel to display time
            let mut top_panel_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(top_panel_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            
            // Display time in top right corner
            top_panel_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(api) = playhead_api {
                    // Get current playhead position in ticks (absolute from timeline start at 0)
                    // playhead_ticks() returns relative ticks from timeline start (scroll position)
                    // We need the absolute position from the beginning of the timeline (tick 0)
                    let playhead_ticks_relative = api.playhead_ticks();
                    // Get timeline_start from MusicalInfo trait (this is the scroll offset)
                    let timeline_start = api.timeline_start().unwrap_or(0.0);
                    // Absolute playhead position from the beginning of the timeline
                    let absolute_playhead_ticks = timeline_start + playhead_ticks_relative;
                    
                    // Convert ticks to time based on bars
                    // Each bar should be 1 second, so calculate which bar we're in and the fraction within that bar
                    let ticks_per_beat = api.ticks_per_beat() as f32;
                    // 4/4 time signature = 4 beats per bar
                    const BEATS_PER_BAR: f32 = 4.0;
                    let ticks_per_bar = ticks_per_beat * BEATS_PER_BAR;
                    
                    // Calculate which bar we're in and the fraction within that bar
                    let bar_number = absolute_playhead_ticks / ticks_per_bar;
                    let total_seconds = bar_number; // Each bar = 1 second
                    
                    let minutes = (total_seconds / 60.0).floor() as u32;
                    let seconds = (total_seconds % 60.0).floor() as u32;
                    let centiseconds = ((total_seconds % 1.0) * 100.0).floor() as u32;
                    
                    let time_string = format!("{:02}:{:02}:{:02}", minutes, seconds, centiseconds);
                    ui.label(time_string);
                } else {
                    // Fallback if no playhead API
                    ui.label("00:00:00");
                }
            });
        }
        self
    }

    /// Show the bottom bar with global buttons.
    /// 
    /// `global_panel_visible` should be a mutable reference to a bool that tracks
    /// whether the global panel is visible. It will be toggled when the "Global" button is clicked.
    pub fn bottom_bar(&self, ui: &mut egui::Ui, global_panel_visible: &mut bool) {
        if let Some(bottom_bar_rect) = self.bottom_bar_rect {
            // Get style before creating child UI
            let vis = ui.style().noninteractive();
            let bg_fill = vis.bg_fill;
            let bg_stroke = vis.bg_stroke;
            
            // Draw bottom bar background
            ui.painter().rect(bottom_bar_rect, 0.0, bg_fill, bg_stroke);
            
            // Create UI for bottom bar
            let mut bottom_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(bottom_bar_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            
            // Add "Global" button
            if bottom_ui.button("Global").clicked() {
                *global_panel_visible = !*global_panel_visible;
            }
            
            // Draw global panel if visible (100px height, above everything)
            if *global_panel_visible {
                const PANEL_HEIGHT: f32 = 100.0;
                let panel_rect = egui::Rect::from_min_max(
                    egui::Pos2::new(bottom_bar_rect.min.x, bottom_bar_rect.min.y - PANEL_HEIGHT),
                    egui::Pos2::new(bottom_bar_rect.max.x, bottom_bar_rect.min.y),
                );
                
                // Draw panel background
                ui.painter().rect(panel_rect, 0.0, bg_fill, bg_stroke);
                
                // Create UI for panel (using a new child to ensure it's above everything)
                let mut panel_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(panel_rect)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                
                // Divide panel into 6 columns
                panel_ui.horizontal(|ui| {
                    // Column 1: "Global Panel" label
                    ui.vertical(|ui| {
                        ui.set_width(panel_rect.width() / 6.0);
                        ui.label("Global Panel");
                    });
                    
                    // Column 2: Available for widgets
                    ui.vertical(|ui| {
                        ui.set_width(panel_rect.width() / 6.0);
                        // Add widgets here
                    });
                    
                    // Column 3: Available for widgets
                    ui.vertical(|ui| {
                        ui.set_width(panel_rect.width() / 6.0);
                        // Add widgets here
                    });
                    
                    // Column 4: Available for widgets
                    ui.vertical(|ui| {
                        ui.set_width(panel_rect.width() / 6.0);
                        // Add widgets here
                    });
                    
                    // Column 5: Available for widgets
                    ui.vertical(|ui| {
                        ui.set_width(panel_rect.width() / 6.0);
                        // Add widgets here
                    });
                    
                    // Column 6: Available for widgets
                    ui.vertical(|ui| {
                        ui.set_width(panel_rect.width() / 6.0);
                        // Add widgets here
                    });
                });
                
                // Draw 1px grey vertical borders between columns (100% height)
                let grey_border = egui::Stroke {
                    width: 1.0,
                    color: egui::Color32::from_rgb(128, 128, 128), // Grey
                };
                let column_width = panel_rect.width() / 6.0;
                for i in 1..6 {
                    let x = panel_rect.min.x + (column_width * i as f32);
                    let top = egui::Pos2::new(x, panel_rect.min.y);
                    let bottom = egui::Pos2::new(x, panel_rect.max.y);
                    ui.painter().line_segment([top, bottom], grey_border);
                }
            }
        }
    }
}
