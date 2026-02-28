use egui::Rect;

/// A context for instantiating tracks, either pinned or unpinned.
pub struct TracksCtx {
    /// The rectangle encompassing the entire widget area including both header and timeline and
    /// both pinned and unpinned track areas.
    pub full_rect: Rect,
    /// The rect encompassing the left-hand-side track headers including pinned and unpinned.
    pub header_full_rect: Option<Rect>,
    /// Context specific to the timeline (non-header) area.
    pub timeline: TimelineCtx,
}

/// Some context for the timeline, providing short-hand for setting some useful widgets.
pub struct TimelineCtx {
    /// The total visible rect of the timeline area including pinned and unpinned tracks.
    pub full_rect: Rect,
    /// The total number of ticks visible on the timeline area.
    pub visible_ticks: f32,
}

/// A type used to assist with setting a track with an optional `header`.
pub struct TrackCtx<'a> {
    tracks: &'a TracksCtx,
    ui: &'a mut egui::Ui,
    available_rect: Rect,
    header_height: f32,
    track_id: Option<String>,
}

/// Context for instantiating the playhead after all tracks have been set.
pub struct SetPlayhead {
    timeline_rect: Rect,
    /// The y position at the bottom of the last track, or the bottom of the
    /// tracks' scrollable area in the case that the size of the tracks
    /// exceed the visible height.
    tracks_bottom: f32,
    /// The bottom bar rectangle (20px height at the bottom).
    pub(crate) bottom_bar_rect: Option<Rect>,
    /// The top panel rectangle (40px height at the top).
    pub(crate) top_panel_rect: Option<Rect>,
}

/// Relevant information for displaying a background for the timeline.
pub struct BackgroundCtx<'a> {
    pub header_full_rect: Option<Rect>,
    pub timeline: &'a TimelineCtx,
}

impl TracksCtx {
    /// Begin showing the next `Track`.
    pub fn next<'a>(&'a self, ui: &'a mut egui::Ui) -> TrackCtx<'a> {
        let available_rect = ui.available_rect_before_wrap();
        TrackCtx {
            tracks: self,
            ui,
            available_rect,
            header_height: 0.0,
            track_id: None,
        }
    }
}

impl<'a> TrackCtx<'a> {
    /// Set the track identifier for selection tracking.
    pub fn with_id(mut self, track_id: impl Into<String>) -> Self {
        self.track_id = Some(track_id.into());
        self
    }

    /// UI for the track's header.
    ///
    /// The header content (text, buttons, etc.) is automatically padded 4px from the left edge
    /// to provide consistent spacing for track labels and controls like mute/solo buttons.
    pub fn header(mut self, header: impl FnOnce(&mut egui::Ui)) -> Self {
        const LEFT_PADDING: f32 = 4.0;
        let header_h = self
            .tracks
            .header_full_rect
            .map(|mut rect| {
                rect.min.y = self.available_rect.min.y;
                // Constrain header height to available rect to prevent overlap with next track
                rect.max.y = rect.min.y.min(self.available_rect.max.y);
                // Add 4px left padding by adjusting the rect
                rect.min.x += LEFT_PADDING;
                let ui = &mut self.ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect)
                        .layout(*self.ui.layout()),
                );
                header(ui);
                ui.min_rect().height()
            })
            .unwrap_or(0.0);
        self.header_height = header_h;
        self
    }

    /// Set the track, with a function for instantiating contents for the timeline.
    pub fn show(
        self,
        track: impl FnOnce(&TimelineCtx, &mut egui::Ui),
        playhead_api: Option<&dyn crate::playhead::PlayheadApi>,
        selection_api: Option<&dyn crate::interaction::TrackSelectionApi>,
    ) {
        // The UI and area for the track timeline.
        let track_timeline_rect = {
            let mut rect = self.tracks.timeline.full_rect;
            rect.min.y = self.available_rect.min.y;
            rect
        };
        
        let track_h = {
            let ui = &mut self.ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(track_timeline_rect)
                    .layout(*self.ui.layout()),
            );
            track(&self.tracks.timeline, ui);
            ui.min_rect().height()
        };
        
        // Calculate the actual track area (only the height of this track, not the full timeline)
        let actual_track_rect = {
            let mut rect = track_timeline_rect;
            rect.max.y = track_timeline_rect.min.y + track_h;
            rect
        };
        
        // Handle interaction for this track
        if let Some(track_id) = &self.track_id {
            // Get selection data before calling handle_track_interaction (which takes ownership)
            // Check if this track has the selection (only one selection exists across all tracks)
            let selection_data = selection_api.as_ref().and_then(|api| {
                if api.get_selected_track_id().as_ref() == Some(track_id) {
                    api.get_selection(track_id)
                } else {
                    None
                }
            });
            let ticks_per_point_for_selection = selection_api.as_ref().map(|api| api.ticks_per_point());
            
            crate::interaction::handle_track_interaction(
                self.ui,
                actual_track_rect,
                track_timeline_rect, // Pass full timeline rect for tick calculation
                track_id,
                playhead_api,
                selection_api,
            );
            
            // Draw selection if it exists on this track
            if let (Some((absolute_start_tick, absolute_end_tick)), Some(ticks_per_point)) = (selection_data, ticks_per_point_for_selection) {
                let timeline_w = track_timeline_rect.width();
                let visible_ticks = ticks_per_point * timeline_w;
                let timeline_start = selection_api.as_ref().map(|api| api.timeline_start()).unwrap_or(0.0);
                
                // Convert absolute ticks to relative ticks for drawing
                let relative_start_tick = absolute_start_tick - timeline_start;
                let relative_end_tick = absolute_end_tick - timeline_start;
                
                // Only draw if selection is visible in current viewport
                if relative_end_tick >= 0.0 && relative_start_tick <= visible_ticks {
                    let start_x = track_timeline_rect.min.x + (relative_start_tick.max(0.0) / visible_ticks) * timeline_w;
                    let end_x = track_timeline_rect.min.x + (relative_end_tick.min(visible_ticks) / visible_ticks) * timeline_w;
                    
                    // Selection height should match track height only (not extend to bottom of screen)
                    // Use track_h to determine the actual bottom of this track
                    let track_top = track_timeline_rect.min.y;
                    let track_bottom = track_timeline_rect.min.y + track_h;
                    let selection_rect = egui::Rect::from_min_max(
                        egui::Pos2::new(start_x.min(end_x), track_top),
                        egui::Pos2::new(start_x.max(end_x), track_bottom),
                    );
                    
                    let selection_fill = egui::Color32::from_rgba_unmultiplied(100, 150, 255, 100);
                    self.ui.painter().rect_filled(selection_rect, 0.0, selection_fill);
                }
            }
        }
        
        // Calculate the full track rect (header + timeline, 100% width)
        let full_track_height = self.header_height.max(track_h);
        let full_track_rect = egui::Rect::from_min_max(
            egui::Pos2::new(
                self.tracks.full_rect.min.x, // Left edge (includes header)
                self.available_rect.min.y,    // Top of this track
            ),
            egui::Pos2::new(
                self.tracks.full_rect.max.x,              // Right edge (full width)
                self.available_rect.min.y + full_track_height, // Bottom of this track
            ),
        );
        
        // Draw a pink border around the entire track (header + timeline) to visualize its boundaries
        // This applies to all tracks including the ruler
        let pink_border = egui::Stroke {
            width: 1.0,
            color: egui::Color32::from_rgb(255, 192, 203), // Pink
        };
        self.ui.painter().rect_stroke(full_track_rect, 0.0, pink_border);
        
        // Manually add space occuppied by the child UIs, otherwise `ScrollArea` won't consider the
        // space occuppied. TODO: Is there a better way to handle this?
        let w = self.tracks.full_rect.width();
        let h = full_track_height;
        self.ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.spacing_mut().interact_size.y = 0.0;
            ui.horizontal(|ui| ui.add_space(w));
            ui.add_space(h);
        });
    }
}

impl TimelineCtx {
    /// The number of visible ticks across the width of the timeline.
    pub fn visible_ticks(&self) -> f32 {
        self.visible_ticks
    }

    /// Get the left edge X position where tick 0 should be displayed.
    pub fn left_edge_x(&self) -> f32 {
        self.full_rect.min.x
    }
}

// Internal access for timeline module
impl TracksCtx {
    pub(crate) fn new(full_rect: Rect, header_full_rect: Option<Rect>, timeline: TimelineCtx) -> Self {
        Self {
            full_rect,
            header_full_rect,
            timeline,
        }
    }
}

impl TimelineCtx {
    pub(crate) fn new(full_rect: Rect, visible_ticks: f32) -> Self {
        Self {
            full_rect,
            visible_ticks,
        }
    }
}

impl SetPlayhead {
    pub(crate) fn new(timeline_rect: Rect, tracks_bottom: f32) -> Self {
        Self {
            timeline_rect,
            tracks_bottom,
            bottom_bar_rect: None,
            top_panel_rect: None,
        }
    }

    pub(crate) fn timeline_rect(&self) -> Rect {
        self.timeline_rect
    }

    pub(crate) fn tracks_bottom(&self) -> f32 {
        self.tracks_bottom
    }
}
