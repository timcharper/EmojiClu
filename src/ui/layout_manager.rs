use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use fixed::types::I8F8;
use glib::{object::ObjectExt, source::SourceId, timeout_add_local, ControlFlow};
use gtk4::{
    glib::SignalHandlerId,
    prelude::{MonitorExt, NativeExt, SurfaceExt, WidgetExt},
    ApplicationWindow,
};
use itertools::Itertools;
use log::trace;

use crate::{
    destroyable::Destroyable,
    events::{EventEmitter, EventObserver, Unsubscriber},
    game::clue_generator_state::MAX_HORIZ_CLUES,
    model::{
        ClueSet, CluesSizing, Difficulty, Dimensions, GameActionEvent, GameStateEvent, GlobalEvent,
        GridCellSizing, GridSizing, HorizontalCluePanelSizing, LayoutConfiguration,
        VerticalCluePanelSizing, MAX_GRID_SIZE,
    },
};

use super::clue_panels_ui::CluePanelsUI;

// Base unit sizes
const SPACING_SMALL: i32 = 2;
const SPACING_MEDIUM: i32 = 5;
const SPACING_LARGE: i32 = 10;

// Icon sizes
const SOLUTION_IMG_SIZE: i32 = 128;
const CANDIDATE_IMG_SIZE: i32 = SOLUTION_IMG_SIZE / 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClueStats {
    pub n_vertical_clues: usize,
    pub n_horizontal_clues: usize,
    pub n_vertical_clue_groups: usize,
}

struct HorizCluePanelSizingInputs {
    n_rows: i32,
    n_columns: i32,
    row_spacing: i32,
    column_spacing: i32,
    margin_left: i32,
    clue_img_size: i32,
    clue_padding: i32,
}

struct VertCluePanelSizingInputs {
    candidate_img_size: i32,
    margin_top: i32,
    column_spacing: i32,
    group_spacing: i32,
    clue_padding: i32,
}

pub struct LayoutManager {
    global_event_emitter: EventEmitter<GlobalEvent>,
    window: Rc<ApplicationWindow>,
    handle_surface_enter_monitor: Option<SignalHandlerId>,
    handle_surface_layout: Option<SignalHandlerId>,
    game_action_subscription: Option<Unsubscriber<GameActionEvent>>,
    game_state_subscription: Option<Unsubscriber<GameStateEvent>>,
    current_difficulty: Difficulty,
    scrolled_window: gtk4::ScrolledWindow,
    container_dimensions: Option<Dimensions>,
    clue_stats: ClueStats,
    last_layout: Option<LayoutConfiguration>,
    last_layout_change: Option<Instant>,
    layout_monitor_source: Option<SourceId>,
    scale_factor: I8F8,
}

impl Destroyable for LayoutManager {
    fn destroy(&mut self) {
        if let Some(subscription_id) = self.game_action_subscription.take() {
            subscription_id.unsubscribe();
        }
        if let Some(source_id) = self.layout_monitor_source.take() {
            source_id.remove();
        }
    }
}

impl Drop for LayoutManager {
    fn drop(&mut self) {
        if let Some(handle) = self.handle_surface_enter_monitor.take() {
            if let Some(surface) = self.window.surface() {
                surface.disconnect(handle);
            }
        }
        if let Some(handle) = self.handle_surface_layout.take() {
            if let Some(surface) = self.window.surface() {
                surface.disconnect(handle);
            }
        }
    }
}

impl LayoutManager {
    pub fn new(
        window: Rc<ApplicationWindow>,
        global_event_emitter: EventEmitter<GlobalEvent>,
        game_action_observer: EventObserver<GameActionEvent>,
        game_state_observer: EventObserver<GameStateEvent>,
        scrolled_window: gtk4::ScrolledWindow,
        current_difficulty: Difficulty,
    ) -> Rc<RefCell<Self>> {
        let dw = Rc::new(RefCell::new(Self {
            global_event_emitter,
            window: window.clone(),
            handle_surface_enter_monitor: None,
            handle_surface_layout: None,
            scrolled_window,
            current_difficulty,
            game_action_subscription: None,
            game_state_subscription: None,
            container_dimensions: None,
            clue_stats: ClueStats::default(),
            last_layout: None,
            last_layout_change: Some(Instant::now()),
            layout_monitor_source: None,
            scale_factor: I8F8::from_num(1),
        }));

        let game_action_handle = game_action_observer.subscribe({
            let dw = dw.clone();
            move |event| {
                dw.borrow_mut().handle_game_action_event(event);
            }
        });

        let game_state_handle = game_state_observer.subscribe({
            let dw = dw.clone();
            move |event| {
                dw.borrow_mut().handle_game_state_event(event);
            }
        });

        window.connect_realize({
            let dw = dw.clone();
                move |window| {
                if let Some(surface) = window.surface() {
                    trace!(target: "layout_manager", "realized; surface: {:?}", surface);
                    let handle = surface.connect_enter_monitor({
                        let dw = dw.clone();
                        move |_, monitor| {
                            trace!(target: "layout_manager", "Entering monitor {:?}; geometry: {:?}, scale_factor: {}", monitor.display(), monitor.geometry(), monitor.scale_factor());
                            dw.borrow_mut().update_scale_factor(monitor.scale());
                        }});

                    let handle2 = surface.connect_layout({
                        let dw = dw.clone();
                        move |_, _, _| {
                            let mut dw = RefCell::borrow_mut(&dw);
                            let dimensions = Dimensions {
                                width: dw.scrolled_window.allocated_width(),
                                height: dw.scrolled_window.allocated_height(),
                        };
                        dw.update_dimensions(Some(dimensions));
                    }});
                    dw.borrow_mut().handle_surface_enter_monitor = Some(handle);
                    dw.borrow_mut().handle_surface_layout = Some(handle2);
                }
            }});

        // Set up layout monitoring

        let source_id = timeout_add_local(Duration::from_millis(100), {
            let weak_dw = Rc::downgrade(&dw);
            move || {
                if let Some(dw) = weak_dw.upgrade() {
                    let mut manager = dw.borrow_mut();
                    manager.check_layout_stability();
                    ControlFlow::Continue
                } else {
                    ControlFlow::Break
                }
            }
        });
        dw.borrow_mut().layout_monitor_source = Some(source_id);
        dw.borrow_mut().game_action_subscription = Some(game_action_handle);
        dw.borrow_mut().game_state_subscription = Some(game_state_handle);

        dw
    }

    fn handle_game_action_event(&mut self, event: &GameActionEvent) {
        match event {
            GameActionEvent::NewGame(difficulty, _) => self.update_difficulty(*difficulty),
            _ => (),
        }
    }

    fn handle_game_state_event(&mut self, event: &GameStateEvent) {
        match event {
            GameStateEvent::ClueSetUpdate(clue_set, _) => self.update_clue_stats(clue_set.as_ref()),
            _ => (),
        }
    }

    fn update_scale_factor(&mut self, scale_factor: f64) {
        if self.scale_factor != scale_factor {
            self.scale_factor = I8F8::from_num(scale_factor);
            let new_layout = self.calculate_scaled_layout();
            self.maybe_publish_layout(new_layout);
        }
    }

    fn update_dimensions(&mut self, dimensions: Option<Dimensions>) {
        if self.container_dimensions != dimensions {
            trace!(target: "layout_manager", "update_dimensions; dimensions: {:?}", dimensions);
            self.container_dimensions = dimensions;
            let new_layout = self.calculate_scaled_layout();
            self.maybe_publish_layout(new_layout);
        }
    }

    fn update_difficulty(&mut self, difficulty: Difficulty) {
        if self.current_difficulty != difficulty {
            self.current_difficulty = difficulty;
            let new_layout = self.calculate_scaled_layout();
            self.maybe_publish_layout(new_layout);
        }
    }

    fn update_clue_stats(&mut self, clue_set: &ClueSet) {
        let v_clue_groups = clue_set
            .vertical_clues()
            .iter()
            .map(|clue| clue.group)
            .unique()
            .count();
        let clue_stats = ClueStats {
            n_vertical_clues: clue_set.vertical_clues().len(),
            n_horizontal_clues: clue_set.horizontal_clues().len(),
            n_vertical_clue_groups: v_clue_groups,
        };
        if self.clue_stats != clue_stats {
            trace!(target: "layout_manager", "update_clue_stats; clue_stats: {:?}", clue_stats);
            self.clue_stats = clue_stats;
            let new_layout = self.calculate_scaled_layout();
            self.maybe_publish_layout(new_layout);
        }
    }

    fn check_layout_stability(&mut self) {
        if let Some(last_change) = self.last_layout_change {
            if last_change.elapsed() >= Duration::from_secs(1) {
                // Layout has been stable for 3 seconds
                if let Some(layout) = &self.last_layout {
                    self.global_event_emitter.emit(GlobalEvent::OptimizeImages {
                        candidate_tile_size: layout.grid.cell.candidate_image.width,
                        solution_tile_size: layout.grid.cell.solution_image.width,
                        scale_factor: self.scale_factor,
                    });
                }
                self.last_layout_change = None;
            }
        }
    }

    fn maybe_publish_layout(&mut self, new_layout: LayoutConfiguration) {
        let layout_changed = !self.last_layout.iter().contains(&new_layout);
        if layout_changed {
            trace!(target: "layout_manager", "layout changed");
            self.global_event_emitter
                .emit(GlobalEvent::LayoutChanged(new_layout.clone()));
            self.last_layout = Some(new_layout);
            self.last_layout_change = Some(Instant::now());
        } else {
            trace!(target: "layout_manager", "layout unchanged");
        }
    }

    pub fn calculate_layout(
        difficulty: Difficulty,
        clue_stats: Option<ClueStats>,
    ) -> LayoutConfiguration {
        let n_variants = difficulty.grid_size();
        let n_rows = difficulty.grid_size();
        let n_horizontal_clues = clue_stats.unwrap_or_default().n_horizontal_clues;

        let solution_image = Dimensions {
            width: SOLUTION_IMG_SIZE,
            height: SOLUTION_IMG_SIZE,
        };

        let candidate_image = Dimensions {
            width: CANDIDATE_IMG_SIZE,
            height: CANDIDATE_IMG_SIZE,
        };

        let clues_per_column = CluePanelsUI::calc_clues_per_column(difficulty) as i32;

        let (horiz_clue_columns, horiz_clue_rows) =
            LayoutManager::calc_horiz_clue_columns(n_horizontal_clues as i32, clues_per_column);

        let clue_padding = SPACING_MEDIUM;

        LayoutConfiguration {
            scale_factor: I8F8::from_num(1),
            grid: LayoutManager::calc_grid_sizing(GridSizingInputs {
                solution_image: solution_image,
                candidate_image: candidate_image,
                n_variants: n_variants as i32,
                n_rows: n_rows as i32,
                candidate_spacing: SPACING_SMALL,
                grid_column_spacing: SPACING_LARGE,
                grid_row_spacing: SPACING_LARGE,
                grid_outer_padding: SPACING_MEDIUM,
            }),
            clues: CluesSizing {
                clue_tile_size: Dimensions {
                    width: CANDIDATE_IMG_SIZE,
                    height: CANDIDATE_IMG_SIZE,
                },
                horizontal_clue_panel: LayoutManager::calc_horiz_clue_panel(
                    HorizCluePanelSizingInputs {
                        n_rows: horiz_clue_rows,
                        n_columns: horiz_clue_columns,
                        row_spacing: SPACING_SMALL,
                        column_spacing: SPACING_MEDIUM * 2,
                        margin_left: SPACING_LARGE * 2,
                        clue_img_size: CANDIDATE_IMG_SIZE,
                        clue_padding,
                    },
                    difficulty,
                ),
                vertical_clue_panel: LayoutManager::calc_vert_clue_panel(
                    VertCluePanelSizingInputs {
                        candidate_img_size: CANDIDATE_IMG_SIZE,
                        margin_top: SPACING_LARGE,
                        column_spacing: SPACING_SMALL,
                        group_spacing: SPACING_MEDIUM * 3,
                        clue_padding,
                    },
                ),
                clue_annotation_size: Dimensions {
                    width: CANDIDATE_IMG_SIZE / 2,
                    height: CANDIDATE_IMG_SIZE / 2,
                },
                clue_padding,
            },
        }
    }

    // TODO - get rid of inputs array
    fn calculate_scaled_layout(&self) -> LayoutConfiguration {
        let base_layout =
            LayoutManager::calculate_layout(self.current_difficulty, Some(self.clue_stats));

        if self.container_dimensions.is_none() {
            return base_layout;
        }

        let surface = self.container_dimensions.as_ref().unwrap();
        let n_variants = self.current_difficulty.grid_size();
        let n_rows = self.current_difficulty.grid_size();

        // Calculate total required dimensions
        let total_grid_width = base_layout.grid.cell.dimensions.width * n_variants as i32
            + base_layout.grid.column_spacing * (n_variants as i32 - 1)
            + SPACING_MEDIUM * 2;

        let grid_height = base_layout.grid.cell.dimensions.height * n_rows as i32
            + base_layout.grid.row_spacing * (n_rows as i32 - 1)
            + SPACING_MEDIUM * 2;

        let grid_plus_vert_clues_height =
            grid_height + base_layout.clues.vertical_clue_panel.total_clues_height + SPACING_LARGE;

        let total_required_height = grid_plus_vert_clues_height.max(
            base_layout
                .clues
                .horizontal_clue_panel
                .total_clues_dimensions
                .height,
        );

        let total_required_width = total_grid_width
            + base_layout
                .clues
                .horizontal_clue_panel
                .total_clues_dimensions
                .width
            + SPACING_LARGE;

        // Calculate scaling factors based on window dimensions
        let available_width = surface.width;
        let available_height = surface.height;

        // Calculate scale factors for both dimensions
        let width_scale = available_width as f32 / total_required_width as f32;
        let height_scale = available_height as f32 / total_required_height as f32;

        // Use the smaller scale factor to maintain aspect ratio
        let scale = width_scale.min(height_scale);
        self.scale_layout(base_layout, scale)
    }

    fn scale_layout(&self, layout: LayoutConfiguration, scale: f32) -> LayoutConfiguration {
        let candidate_image = layout.grid.cell.candidate_image.scale_by(scale);
        let solution_image = layout.grid.cell.solution_image.scale_by(scale);
        let clue_padding = (layout.clues.clue_padding as f32 * scale) as i32;

        let scaled_clues = CluesSizing {
            clue_tile_size: layout.clues.clue_tile_size.scale_by(scale),
            horizontal_clue_panel: LayoutManager::calc_horiz_clue_panel(
                HorizCluePanelSizingInputs {
                    n_rows: layout.clues.horizontal_clue_panel.n_rows,
                    n_columns: layout.clues.horizontal_clue_panel.n_columns,
                    row_spacing: (layout.clues.horizontal_clue_panel.row_spacing as f32 * scale)
                        as i32,
                    column_spacing: (layout.clues.horizontal_clue_panel.column_spacing as f32
                        * scale) as i32,
                    margin_left: (layout.clues.horizontal_clue_panel.left_margin as f32 * scale)
                        as i32,
                    clue_img_size: candidate_image.width,
                    clue_padding: clue_padding,
                },
                self.current_difficulty,
            ),
            vertical_clue_panel: LayoutManager::calc_vert_clue_panel(VertCluePanelSizingInputs {
                candidate_img_size: candidate_image.width,
                margin_top: (layout.clues.vertical_clue_panel.margin_top as f32 * scale) as i32,
                column_spacing: (layout.clues.vertical_clue_panel.column_spacing as f32 * scale)
                    as i32,
                group_spacing: (layout.clues.vertical_clue_panel.group_spacing as f32 * scale)
                    as i32,
                clue_padding,
            }),
            clue_annotation_size: layout.clues.clue_annotation_size.scale_by(scale),
            clue_padding: clue_padding,
        };

        LayoutConfiguration {
            scale_factor: self.scale_factor,
            grid: LayoutManager::calc_grid_sizing(GridSizingInputs {
                solution_image: solution_image,
                candidate_image: candidate_image,
                n_variants: layout.grid.n_variants,
                n_rows: layout.grid.n_rows,
                candidate_spacing: (layout.grid.cell.candidate_spacing as f32 * scale) as i32,
                grid_column_spacing: (layout.grid.column_spacing as f32 * scale) as i32,
                grid_row_spacing: (layout.grid.row_spacing as f32 * scale) as i32,
                grid_outer_padding: (layout.grid.outer_margin as f32 * scale) as i32,
            }),
            clues: scaled_clues,
        }
    }

    fn calc_horiz_clue_columns(n_horizontal_clues: i32, clues_per_column: i32) -> (i32, i32) {
        let n_cols =
            (n_horizontal_clues - 1/* 16 clues is still 1 column */) / clues_per_column + 1;
        let n_rows = n_horizontal_clues.clamp(0, clues_per_column);
        (n_cols, n_rows)
    }

    fn calc_horiz_clue_panel(
        inputs: HorizCluePanelSizingInputs,
        difficulty: Difficulty,
    ) -> HorizontalCluePanelSizing {
        let clue_width = inputs.clue_img_size * 3  // 3 tiles
            + inputs.clue_padding * 2; // padding on both sides
        let clue_height = inputs.clue_img_size + inputs.clue_padding * 2;

        let clues_per_column = CluePanelsUI::calc_clues_per_column(difficulty) as i32;

        let max_columns = MAX_HORIZ_CLUES as i32 / clues_per_column;
        let n_horiz_spacers = inputs.n_rows.clamp(1, clues_per_column) - 1;
        let n_vert_spacers = inputs.n_columns.clamp(1, max_columns) - 1;

        // Total width for all horizontal clue columns including spacing between columns
        let all_horiz_clues_width = (clue_width * inputs.n_columns)
            + (n_vert_spacers * inputs.column_spacing)
            + inputs.margin_left;

        let all_horiz_clues_height =
            (clue_height * inputs.n_rows) + (inputs.row_spacing * n_horiz_spacers);

        HorizontalCluePanelSizing {
            total_clues_dimensions: Dimensions {
                width: all_horiz_clues_width,
                height: all_horiz_clues_height,
            },
            row_spacing: inputs.row_spacing,
            column_spacing: inputs.column_spacing,
            left_margin: inputs.margin_left,
            n_rows: inputs.n_rows,
            n_columns: inputs.n_columns,
            clue_dimensions: Dimensions {
                width: inputs.clue_img_size * 3,
                height: inputs.clue_img_size,
            },
        }
    }

    fn calc_vert_clue_panel(inputs: VertCluePanelSizingInputs) -> VerticalCluePanelSizing {
        let clue_height = inputs.candidate_img_size * 3 + inputs.clue_padding * 2;
        let clue_width = inputs.candidate_img_size + inputs.clue_padding * 2;

        VerticalCluePanelSizing {
            total_clues_height: clue_height + inputs.margin_top,
            margin_top: inputs.margin_top,
            column_spacing: inputs.column_spacing,
            group_spacing: inputs.group_spacing,
            clue_dimensions: Dimensions {
                width: clue_width,
                height: clue_height,
            },
        }
    }

    fn calc_grid_sizing(inputs: GridSizingInputs) -> GridSizing {
        let n_variants = inputs.n_variants;
        let n_rows = inputs.n_rows;

        let candidate_n_rows = (n_variants as i32 + 1) / 2;
        let candidate_n_columns = n_variants as i32;

        let base_cell_aspect_ratio_width = (n_variants as i32 + 1) / 2;
        let base_cell_aspect_ratio_height = 2;

        let cell_width = (inputs.solution_image.width * base_cell_aspect_ratio_width
            / base_cell_aspect_ratio_height)
            + inputs.candidate_spacing * (candidate_n_columns - 1);

        let cell_height =
            inputs.solution_image.height + inputs.candidate_spacing * (candidate_n_rows - 1);

        let base_cell_sizing = GridCellSizing {
            dimensions: Dimensions {
                width: cell_width,
                height: cell_height,
            },
            solution_image: inputs.solution_image,
            candidate_image: inputs.candidate_image,
            candidate_spacing: inputs.candidate_spacing,
            candidate_rows: candidate_n_rows,
            candidate_columns: candidate_n_columns,
        };

        // Calculate total grid dimensions without scaling
        let total_grid_width = base_cell_sizing.dimensions.width * n_variants as i32
            + inputs.grid_column_spacing * (n_variants as i32 - 1)
            + inputs.grid_outer_padding * 2;

        let grid_height = base_cell_sizing.dimensions.height * n_rows as i32
            + inputs.grid_row_spacing * (n_rows.clamp(1, MAX_GRID_SIZE as i32) - 1)
            + inputs.grid_outer_padding * 2;

        GridSizing {
            column_spacing: inputs.grid_column_spacing,
            row_spacing: inputs.grid_row_spacing,
            outer_margin: inputs.grid_outer_padding,
            cell: base_cell_sizing,
            n_variants: inputs.n_variants,
            n_rows: inputs.n_rows,
            total_dimensions: Dimensions {
                width: total_grid_width,
                height: grid_height,
            },
        }
    }
}

struct GridSizingInputs {
    solution_image: Dimensions,
    candidate_image: Dimensions,
    n_variants: i32,
    n_rows: i32,
    candidate_spacing: i32, // space between candidate tiles
    grid_column_spacing: i32,
    grid_row_spacing: i32,
    grid_outer_padding: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_horiz_clue_columns() {
        // Test case 1: Empty case
        assert_eq!(LayoutManager::calc_horiz_clue_columns(0, 16), (1, 0));

        // Test case 2: Single column not full (10 clues, 16 per column)
        assert_eq!(LayoutManager::calc_horiz_clue_columns(10, 16), (1, 10));

        // Test case 3: Single column exactly full (16 clues, 16 per column)
        assert_eq!(LayoutManager::calc_horiz_clue_columns(16, 16), (1, 16));

        // Test case 4: Two columns needed (17 clues, 16 per column)
        assert_eq!(LayoutManager::calc_horiz_clue_columns(17, 16), (2, 16));

        // Test case 5: Two columns with partial second column (20 clues, 16 per column)
        assert_eq!(LayoutManager::calc_horiz_clue_columns(20, 16), (2, 16));

        // Test case 6: Three full columns (48 clues, 16 per column)
        assert_eq!(LayoutManager::calc_horiz_clue_columns(48, 16), (3, 16));

        // Test case 7: Different clues_per_column value
        assert_eq!(LayoutManager::calc_horiz_clue_columns(10, 5), (2, 5));

        // Test case 8: Negative number of clues (should handle gracefully)
        assert_eq!(LayoutManager::calc_horiz_clue_columns(-1, 16), (1, 0));
    }
}
