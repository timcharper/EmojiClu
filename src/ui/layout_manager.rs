use std::{cell::RefCell, rc::Rc};

use gtk::{
    gdk::Monitor,
    glib::{ObjectExt, SignalHandlerId},
    prelude::{MonitorExt, NativeExt, SurfaceExt, WidgetExt},
    ApplicationWindow,
};
use itertools::Itertools;
use log::trace;

use crate::{
    destroyable::Destroyable,
    events::{EventEmitter, EventObserver, Unsubscriber},
    model::{
        ClueSet, CluesSizing, Difficulty, Dimensions, GameActionEvent, GameStateEvent, GlobalEvent,
        GridCellSizing, GridSizing, LayoutConfiguration,
    },
};

use super::ResourceSet;

// Base unit sizes
const SPACING_SMALL: i32 = 2;
const SPACING_LARGE: i32 = 10;

// Derived sizes
const FRAME_MARGIN: i32 = SPACING_SMALL;

// Icon sizes
const SOLUTION_IMG_SIZE: i32 = 128;
const CANDIDATE_IMG_SIZE: i32 = SOLUTION_IMG_SIZE / 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClueStats {
    pub n_vertical_clues: usize,
    pub n_horizontal_clues: usize,
    pub n_vertical_clue_groups: usize,
}

pub struct LayoutManager {
    global_event_emitter: EventEmitter<GlobalEvent>,
    window: Rc<ApplicationWindow>,
    handle_surface_enter_monitor: Option<SignalHandlerId>,
    handle_surface_layout: Option<SignalHandlerId>,
    game_action_subscription: Option<Unsubscriber<GameActionEvent>>,
    game_state_subscription: Option<Unsubscriber<GameStateEvent>>,
    resources: Rc<ResourceSet>,
    current_difficulty: Difficulty,
    pub scrolled_window: gtk::ScrolledWindow,
    container_dimensions: Option<Dimensions>,
    clue_stats: ClueStats,
    last_layout: Option<LayoutConfiguration>,
}

impl Destroyable for LayoutManager {
    fn destroy(&mut self) {
        if let Some(subscription_id) = self.game_action_subscription.take() {
            subscription_id.unsubscribe();
        }
    }
}

impl Drop for LayoutManager {
    fn drop(&mut self) {
        if let Some(handle) = self.handle_surface_enter_monitor.take() {
            self.window.surface().disconnect(handle);
        }
        if let Some(handle) = self.handle_surface_layout.take() {
            self.window.surface().disconnect(handle);
        }
    }
}

impl LayoutManager {
    pub fn new(
        window: Rc<ApplicationWindow>,
        global_event_emitter: EventEmitter<GlobalEvent>,
        game_action_observer: EventObserver<GameActionEvent>,
        game_state_observer: EventObserver<GameStateEvent>,
        resources: Rc<ResourceSet>,
        current_difficulty: Difficulty,
    ) -> Rc<RefCell<Self>> {
        // Create main container
        let scrolled_container = gtk::ScrolledWindow::builder()
            .hexpand_set(true)
            .vexpand_set(true)
            .build();
        let dw = Rc::new(RefCell::new(Self {
            global_event_emitter,
            window: window.clone(),
            handle_surface_enter_monitor: None,
            handle_surface_layout: None,
            resources,
            scrolled_window: scrolled_container,
            current_difficulty,
            game_action_subscription: None,
            game_state_subscription: None,
            container_dimensions: None,
            clue_stats: ClueStats::default(),
            last_layout: None,
        }));

        {
            let dw = dw.clone();
            let d2_handle = dw.clone();
            let handle = game_action_observer.subscribe(move |event| {
                d2_handle.borrow_mut().handle_game_action_event(event);
            });
            dw.borrow_mut().game_action_subscription = Some(handle);
        }

        {
            let dw = dw.clone();
            let d2_handle = dw.clone();
            let handle = game_state_observer.subscribe(move |event| {
                d2_handle.borrow_mut().handle_game_state_event(event);
            });
            dw.borrow_mut().game_state_subscription = Some(handle);
        }

        {
            let window = window.clone();

            {
                let dw = dw.clone();
                window.connect_realize(move |window| {
                    let surface = window.surface();
                    trace!(target: "layout_manager", "realized; surface: {:?}", surface);
                    let handle = surface.connect_enter_monitor(move |_, monitor| {
                        trace!(target: "layout_manager", "Entering monitor {:?}; geometry: {:?}, scale_factor: {}", monitor.display(), monitor.geometry(), monitor.scale_factor());
                    });

                    let d2_handle2 = dw.clone();
                    let handle2 = surface.connect_layout(move |_, _, _| {
                        let mut dw = RefCell::borrow_mut(&d2_handle2);
                        let dimensions = Dimensions {
                            width: dw.scrolled_window.allocated_width(),
                            height: dw.scrolled_window.allocated_height(),
                        };
                        dw.update_dimensions(Some(dimensions));
                    });
                    dw.borrow_mut().handle_surface_enter_monitor = Some(handle);
                    dw.borrow_mut().handle_surface_layout = Some(handle2);
                });
            }
        }

        dw
    }

    fn handle_game_action_event(&mut self, event: &GameActionEvent) {
        match event {
            GameActionEvent::NewGame(difficulty) => self.update_difficulty(*difficulty),
            _ => (),
        }
    }

    fn handle_game_state_event(&mut self, event: &GameStateEvent) {
        match event {
            GameStateEvent::ClueSetUpdate(clue_set) => self.update_clue_stats(clue_set.as_ref()),
            _ => (),
        }
    }

    fn update_dimensions(&mut self, dimensions: Option<Dimensions>) {
        if self.container_dimensions != dimensions {
            trace!(target: "layout_manager", "update_dimensions; dimensions: {:?}", dimensions);
            self.container_dimensions = dimensions;
            let new_layout = self.calculate_layout();
            self.maybe_publish_layout(new_layout);
        }
    }

    fn update_difficulty(&mut self, difficulty: Difficulty) {
        if self.current_difficulty != difficulty {
            self.current_difficulty = difficulty;
            let new_layout = self.calculate_layout();
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
            let new_layout = self.calculate_layout();
            self.maybe_publish_layout(new_layout);
        }
    }

    fn maybe_publish_layout(&mut self, new_layout: LayoutConfiguration) {
        let layout_changed = !self.last_layout.iter().contains(&new_layout);
        if layout_changed {
            trace!(target: "layout_manager", "layout changed");
            self.global_event_emitter
                .emit(&&GlobalEvent::LayoutChanged(new_layout.clone()));
            self.last_layout = Some(new_layout);
        } else {
            trace!(target: "layout_manager", "layout unchanged");
        }
    }

    pub fn unscaled_layout(
        difficulty: Difficulty,
        clue_stats: Option<ClueStats>,
    ) -> LayoutConfiguration {
        let n_variants = difficulty.grid_size();
        let n_rows = difficulty.grid_size();
        let n_horizontal_clues = clue_stats.unwrap_or_default().n_horizontal_clues;

        let base_cell_aspect_ratio_width = (n_variants as i32 + 1) / 2;
        let base_cell_aspect_ratio_height = 2;

        let base_cell_sizing = GridCellSizing {
            dimensions: Dimensions {
                width: SOLUTION_IMG_SIZE * base_cell_aspect_ratio_width
                    / base_cell_aspect_ratio_height,
                height: SOLUTION_IMG_SIZE,
            },
            solution_image: Dimensions {
                width: SOLUTION_IMG_SIZE,
                height: SOLUTION_IMG_SIZE,
            },
            candidate_image: Dimensions {
                width: CANDIDATE_IMG_SIZE,
                height: CANDIDATE_IMG_SIZE,
            },
            padding: FRAME_MARGIN,
        };

        // Calculate total grid dimensions without scaling
        let total_grid_width = base_cell_sizing.dimensions.width * n_variants as i32
            + SPACING_LARGE * (n_variants as i32 - 1)
            + base_cell_sizing.padding * 2;

        let grid_height = base_cell_sizing.dimensions.height * n_rows as i32
            + SPACING_LARGE * (n_rows as i32 - 1)
            + base_cell_sizing.padding * 2;

        // Calculate vertical clues height (3 tiles high for each clue)
        let base_vert_clue_height = CANDIDATE_IMG_SIZE * 3  // 3 tiles
            + SPACING_LARGE * 2  // spacing between tiles
            + FRAME_MARGIN * 2; // padding on both sides

        // Calculate horizontal clues space needed (16 clues per column)
        let horiz_clue_columns = (n_horizontal_clues as i32) / 16 + 1;

        // Base width for a single clue column
        let base_clue_panel_width = CANDIDATE_IMG_SIZE * 3  // 3 tiles
            + SPACING_LARGE * 2  // spacing between tiles
            + FRAME_MARGIN * 2; // padding on both sides

        // Total width for all horizontal clue columns including spacing between columns
        let horiz_clues_width = if horiz_clue_columns > 1 {
            trace!(target: "layout_manager", "horiz_clues_width; horiz_clue_columns: {}; base_clue_panel_width: {}; SPACING_LARGE: {}; n_horizontal_clues: {}", horiz_clue_columns, base_clue_panel_width, SPACING_LARGE, n_horizontal_clues);
            base_clue_panel_width * horiz_clue_columns + (horiz_clue_columns - 1) * SPACING_LARGE
        } else {
            base_clue_panel_width
        };

        LayoutConfiguration {
            grid: GridSizing {
                column_spacing: SPACING_LARGE,
                row_spacing: SPACING_LARGE,
                outer_padding: 3, // CSS padding
                cell: base_cell_sizing,
                n_variants: n_variants,
                n_rows: n_rows,
                total_dimensions: Dimensions {
                    width: total_grid_width,
                    height: grid_height,
                },
            },
            clues: CluesSizing {
                clue_tile_size: Dimensions {
                    width: CANDIDATE_IMG_SIZE,
                    height: CANDIDATE_IMG_SIZE,
                },
                horizontal_clue_panel_width: horiz_clues_width,
                vertical_clue_panel_height: base_vert_clue_height,
                vertical_clue_group_spacer: SPACING_LARGE,
                clue_annotation_size: Dimensions {
                    width: CANDIDATE_IMG_SIZE / 3,
                    height: CANDIDATE_IMG_SIZE / 3,
                },
                horizontal_margin: 10,
                vertical_margin: 10,
                horizontal_clue_column_spacing: 10,
            },
        }
    }

    // TODO - get rid of inputs array
    fn calculate_layout(&self) -> LayoutConfiguration {
        // fn get_layout_inputs(&self) -> LayoutInputs {
        //     LayoutInputs {
        //         difficulty: self.current_difficulty,
        //         window: self.container_dimensions.clone(),
        //         n_vertical_clues: self.clue_stats.n_vertical_clues,
        //         n_horizontal_clues: self.clue_stats.n_horizontal_clues,
        //     }
        // }

        let base_layout =
            LayoutManager::unscaled_layout(self.current_difficulty, Some(self.clue_stats));

        if self.container_dimensions.is_none() {
            return base_layout;
        }

        let window = self.container_dimensions.as_ref().unwrap();
        let n_variants = self.current_difficulty.grid_size();
        let n_rows = self.current_difficulty.grid_size();

        // Calculate total required dimensions
        let total_grid_width = base_layout.grid.cell.dimensions.width * n_variants as i32
            + base_layout.grid.column_spacing * (n_variants as i32 - 1)
            + base_layout.grid.cell.padding * 2;

        let grid_height = base_layout.grid.cell.dimensions.height * n_rows as i32
            + base_layout.grid.row_spacing * (n_rows as i32 - 1)
            + base_layout.grid.cell.padding * 2;

        let total_required_height =
            grid_height + base_layout.clues.vertical_clue_panel_height + SPACING_LARGE;
        let total_required_width =
            total_grid_width + base_layout.clues.horizontal_clue_panel_width + SPACING_LARGE;

        // Calculate scaling factors based on window dimensions
        let window_margin = 40; // pixels for window decorations
        let available_width = window.width - window_margin;
        let available_height = window.height - window_margin;

        // Calculate scale factors for both dimensions
        let width_scale = available_width as f32 / total_required_width as f32;
        let height_scale = available_height as f32 / total_required_height as f32;

        // Use the smaller scale factor to maintain aspect ratio
        let scale = width_scale.min(height_scale).min(1.0);

        // Apply scaling to cell dimensions
        let scaled_cell_sizing = GridCellSizing {
            dimensions: Dimensions {
                width: (base_layout.grid.cell.dimensions.width as f32 * scale) as i32,
                height: (base_layout.grid.cell.dimensions.height as f32 * scale) as i32,
            },
            solution_image: Dimensions {
                width: (base_layout.grid.cell.solution_image.width as f32 * scale) as i32,
                height: (base_layout.grid.cell.solution_image.height as f32 * scale) as i32,
            },
            candidate_image: Dimensions {
                width: (base_layout.grid.cell.candidate_image.width as f32 * scale) as i32,
                height: (base_layout.grid.cell.candidate_image.height as f32 * scale) as i32,
            },
            padding: (base_layout.grid.cell.padding as f32 * scale) as i32,
        };

        // Scale spacing proportionally
        let scaled_spacing = (base_layout.grid.column_spacing as f32 * scale) as i32;

        // Scale clue dimensions
        let scaled_clues = CluesSizing {
            clue_tile_size: Dimensions {
                width: (base_layout.clues.clue_tile_size.width as f32 * scale) as i32,
                height: (base_layout.clues.clue_tile_size.height as f32 * scale) as i32,
            },
            horizontal_clue_panel_width: (base_layout.clues.horizontal_clue_panel_width as f32
                * scale) as i32,
            vertical_clue_panel_height: (base_layout.clues.vertical_clue_panel_height as f32
                * scale) as i32,
            clue_annotation_size: Dimensions {
                width: (base_layout.clues.clue_annotation_size.width as f32 * scale) as i32,
                height: (base_layout.clues.clue_annotation_size.height as f32 * scale) as i32,
            },
            horizontal_margin: (base_layout.clues.horizontal_margin as f32 * scale) as i32,
            vertical_margin: (base_layout.clues.vertical_margin as f32 * scale) as i32,
            horizontal_clue_column_spacing: (base_layout.clues.horizontal_clue_column_spacing
                as f32
                * scale) as i32,
            vertical_clue_group_spacer: (base_layout.clues.vertical_clue_group_spacer as f32
                * scale) as i32,
        };

        // Recompute grid dimensions based on scaled components to avoid rounding errors
        let scaled_grid_width = scaled_cell_sizing.dimensions.width * n_variants as i32
            + scaled_spacing * (n_variants as i32 - 1)
            + scaled_cell_sizing.padding * 2;

        let scaled_grid_height = scaled_cell_sizing.dimensions.height * n_rows as i32
            + scaled_spacing * (n_rows as i32 - 1)
            + scaled_cell_sizing.padding * 2;

        LayoutConfiguration {
            grid: GridSizing {
                column_spacing: scaled_spacing,
                row_spacing: scaled_spacing,
                outer_padding: (base_layout.grid.outer_padding as f32 * scale) as i32,
                cell: scaled_cell_sizing,
                n_variants: n_variants,
                n_rows: n_rows,
                total_dimensions: Dimensions {
                    width: scaled_grid_width,
                    height: scaled_grid_height,
                },
            },
            clues: scaled_clues,
        }
    }

    // fn set_appropriate_resource_size(&self, monitor: &Monitor) {
    //     let scale_factor = monitor.scale_factor();
    //     let max_dimensions = monitor.geometry();
    //     // enough to fit 8 tiles
    //     // let tile_width = self.resources.tile_width * scale_factor;
    //     // let tile_height = self.resources.tile_height * scale_factor;
    // }
}
