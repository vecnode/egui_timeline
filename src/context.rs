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
        }
    }
}

impl<'a> TrackCtx<'a> {
    /// UI for the track's header.
    pub fn header(mut self, header: impl FnOnce(&mut egui::Ui)) -> Self {
        let header_h = self
            .tracks
            .header_full_rect
            .map(|mut rect| {
                rect.min.y = self.available_rect.min.y;
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
    pub fn show(self, track: impl FnOnce(&TimelineCtx, &mut egui::Ui)) {
        // The UI and area for the track timeline.
        let track_h = {
            let mut rect = self.tracks.timeline.full_rect;
            rect.min.y = self.available_rect.min.y;
            let ui = &mut self.ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(*self.ui.layout()),
            );
            track(&self.tracks.timeline, ui);
            ui.min_rect().height()
        };
        // Manually add space occuppied by the child UIs, otherwise `ScrollArea` won't consider the
        // space occuppied. TODO: Is there a better way to handle this?
        let w = self.tracks.full_rect.width();
        let h = self.header_height.max(track_h);
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
        }
    }

    pub(crate) fn timeline_rect(&self) -> Rect {
        self.timeline_rect
    }

    pub(crate) fn tracks_bottom(&self) -> f32 {
        self.tracks_bottom
    }
}
