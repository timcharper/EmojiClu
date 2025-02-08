use gtk4::{
    prelude::{GridExt, WidgetExt},
    Grid,
};
use std::{cell::RefCell, collections::HashSet, rc::Rc, time::Duration};

use crate::{
    destroyable::Destroyable, events::Unsubscriber, game::settings::Settings, model::ClueSelection,
};
use crate::{
    events::{EventEmitter, EventObserver},
    model::{ClueOrientation, ClueSet, GameStateEvent, GlobalEvent, InputEvent},
};
use crate::{
    game::clue_generator_state::{MAX_HORIZ_CLUES, MAX_VERT_CLUES},
    model::ClueWithGrouping,
};
use crate::{model::Difficulty, ui::ImageSet};
use crate::{model::LayoutConfiguration, ui::clue_ui::ClueUI};

pub struct CluePanelsUI {
    pub horizontal_grid: Grid,
    pub vertical_grid: Grid,
    horizontal_clue_uis: Vec<Rc<RefCell<ClueUI>>>,
    vertical_clue_uis: Vec<Rc<RefCell<ClueUI>>>,
    input_event_emitter: EventEmitter<InputEvent>,
    resources: Rc<ImageSet>,
    game_state_subscription_id: Option<Unsubscriber<GameStateEvent>>,
    settings_subscription_id: Option<Unsubscriber<GlobalEvent>>,
    current_layout: LayoutConfiguration,
    tooltips_enabled: bool,
    current_xray_enabled: bool,
}

impl Destroyable for CluePanelsUI {
    fn destroy(&mut self) {
        // Unparent all widgets
        self.horizontal_grid.unparent();
        self.vertical_grid.unparent();
        if let Some(subscription_id) = self.game_state_subscription_id.take() {
            subscription_id.unsubscribe();
        }
        if let Some(subscription_id) = self.settings_subscription_id.take() {
            subscription_id.unsubscribe();
        }
        for clue_ui in &mut self.horizontal_clue_uis {
            clue_ui.borrow_mut().destroy();
        }
        for clue_ui in &mut self.vertical_clue_uis {
            clue_ui.borrow_mut().destroy();
        }
        self.horizontal_clue_uis.clear();
        self.vertical_clue_uis.clear();
    }
}

// Parent widget for both horizontal clues and vertical clues
impl CluePanelsUI {
    pub fn new(
        input_event_emitter: EventEmitter<InputEvent>,
        game_state_observer: EventObserver<GameStateEvent>,
        global_event_observer: EventObserver<GlobalEvent>,
        resources: &Rc<ImageSet>,
        layout: LayoutConfiguration,
        settings: &Settings,
    ) -> Rc<RefCell<Self>> {
        let horizontal_clues_grid = Grid::builder()
            .row_spacing(layout.clues.horizontal_clue_panel.row_spacing)
            .column_spacing(layout.clues.horizontal_clue_panel.column_spacing)
            .margin_start(layout.clues.horizontal_clue_panel.left_margin)
            .hexpand(true)
            .vexpand(true)
            .name("horizontal-clues-panel")
            .css_classes(["horizontal-clues"])
            .build();

        // Create vertical clues area (bottom)
        let vertical_clues_grid = Grid::builder()
            .column_spacing(layout.clues.vertical_clue_panel.column_spacing)
            .margin_top(layout.clues.vertical_clue_panel.margin_top)
            .hexpand(true)
            .vexpand(true)
            .name("vertical-clues-panel")
            .css_classes(["vertical-clues"])
            .build();

        let clue_set_ui = Rc::new(RefCell::new(Self {
            horizontal_grid: horizontal_clues_grid,
            vertical_grid: vertical_clues_grid,
            horizontal_clue_uis: Vec::with_capacity(MAX_HORIZ_CLUES),
            vertical_clue_uis: Vec::with_capacity(MAX_VERT_CLUES),
            input_event_emitter: input_event_emitter,
            resources: Rc::clone(resources),
            game_state_subscription_id: None,
            settings_subscription_id: None,
            current_layout: layout,
            tooltips_enabled: settings.clue_tooltips_enabled,
            current_xray_enabled: settings.clue_spotlight_enabled,
        }));

        Self::connect_observers(
            clue_set_ui.clone(),
            game_state_observer,
            global_event_observer,
        );

        clue_set_ui
    }

    fn connect_observers(
        clue_set_ui: Rc<RefCell<Self>>,
        game_state_observer: EventObserver<GameStateEvent>,
        global_event_observer: EventObserver<GlobalEvent>,
    ) {
        let clue_set_ui_moved = clue_set_ui.clone();
        let game_state_subscription = game_state_observer.subscribe(move |event| {
            clue_set_ui_moved
                .borrow_mut()
                .handle_game_state_event(event);
        });

        let clue_set_ui_moved = clue_set_ui.clone();
        let settings_subscription = global_event_observer.subscribe(move |event| {
            clue_set_ui_moved.borrow_mut().handle_global_event(event);
        });

        clue_set_ui.borrow_mut().game_state_subscription_id = Some(game_state_subscription);
        clue_set_ui.borrow_mut().settings_subscription_id = Some(settings_subscription);
    }

    fn handle_global_event(&mut self, event: &GlobalEvent) {
        match event {
            GlobalEvent::SettingsChanged(settings) => {
                self.update_tooltip_visibility(settings.clue_tooltips_enabled);
                self.update_xray_enabled(settings.clue_spotlight_enabled);
            }
            GlobalEvent::LayoutChanged(new_layout) => {
                self.update_layout(new_layout);
            }
            GlobalEvent::ImagesOptimized(new_image_set) => {
                self.resources = new_image_set.clone();
                // propagate image set to all clue_uis
                for clue_ui in &mut self.horizontal_clue_uis {
                    clue_ui.borrow_mut().set_image_set(self.resources.clone());
                }
                for clue_ui in &mut self.vertical_clue_uis {
                    clue_ui.borrow_mut().set_image_set(self.resources.clone());
                }
            }
            _ => {}
        }
    }

    fn update_xray_enabled(&mut self, enabled: bool) {
        self.current_xray_enabled = enabled;
        self.sync_xray_enabled();
    }

    fn sync_xray_enabled(&mut self) {
        // dispatch to clue_uis
        for clue_ui in &mut self.horizontal_clue_uis {
            clue_ui
                .borrow_mut()
                .update_xray_enabled(self.current_xray_enabled);
        }
        for clue_ui in &mut self.vertical_clue_uis {
            clue_ui
                .borrow_mut()
                .update_xray_enabled(self.current_xray_enabled);
        }
    }

    fn handle_game_state_event(&mut self, event: &GameStateEvent) {
        match event {
            GameStateEvent::ClueSetUpdate(clue_set, difficulty) => {
                self.set_clues(clue_set, *difficulty);
            }
            GameStateEvent::ClueHintHighlight(Some(clue_with_grouping)) => {
                self.highlight_clue(
                    clue_with_grouping.orientation,
                    clue_with_grouping.index,
                    Duration::from_secs(4),
                );
            }
            GameStateEvent::GridUpdate(grid) => {
                self.set_horiz_completion(&grid.completed_horizontal_clues);
                self.set_vert_completion(&grid.completed_vertical_clues);
            }
            GameStateEvent::ClueSelected(clue_selection) => {
                self.set_clue_selected(&clue_selection);
            }
            _ => {}
        }
    }

    fn allocate_clue_uis(&mut self, difficulty: Difficulty, clue_set: &ClueSet) {
        let n_rows = difficulty.grid_size();
        let clues_per_column = n_rows * 2;

        // horizontal clues
        for (idx, clue_with_grouping) in clue_set.horizontal_clues().iter().enumerate() {
            let grid_col = idx / clues_per_column;
            let grid_row = idx % clues_per_column;

            let clue_set = ClueUI::new(
                Rc::clone(&self.resources),
                clue_with_grouping.clue.clone(),
                self.current_layout.clues.clone(),
                self.input_event_emitter.clone(),
                idx,
                self.current_xray_enabled,
                self.tooltips_enabled,
            );
            self.horizontal_grid.attach(
                &clue_set.borrow().frame,
                grid_col as i32,
                grid_row as i32,
                1,
                1,
            );
            self.horizontal_clue_uis.push(clue_set);
        }

        // Create vertical clue cells (3 tiles high for each clue)
        for (col, clue_with_grouping) in clue_set.vertical_clues().iter().enumerate() {
            let clue_set = ClueUI::new(
                Rc::clone(&self.resources),
                clue_with_grouping.clue.clone(),
                self.current_layout.clues.clone(),
                self.input_event_emitter.clone(),
                col,
                self.current_xray_enabled,
                self.tooltips_enabled,
            );
            self.vertical_grid
                .attach(&clue_set.borrow().frame, col as i32, 0, 1, 1);
            self.vertical_clue_uis.push(clue_set);
        }
    }

    fn highlight_clue(&self, orientation: ClueOrientation, clue_idx: usize, duration: Duration) {
        match orientation {
            ClueOrientation::Horizontal => {
                self.horizontal_clue_uis[clue_idx]
                    .borrow_mut()
                    .highlight_for(duration);
            }
            ClueOrientation::Vertical => {
                self.vertical_clue_uis[clue_idx]
                    .borrow_mut()
                    .highlight_for(duration);
            }
        }
    }

    fn clear_clue_uis(&mut self) {
        // First destroy the ClueUI instances which will cleanup their internal grids
        for clue_ui in &mut self.horizontal_clue_uis {
            clue_ui.borrow_mut().destroy();
        }
        for clue_ui in &mut self.vertical_clue_uis {
            clue_ui.borrow_mut().destroy();
        }
        self.horizontal_clue_uis.clear();
        self.vertical_clue_uis.clear();

        // Then clean up the container grids that hold the clue frames
        while let Some(child) = self.horizontal_grid.first_child() {
            self.horizontal_grid.remove(&child);
        }
        while let Some(child) = self.vertical_grid.first_child() {
            self.vertical_grid.remove(&child);
        }
    }

    fn set_clues(&mut self, clue_set: &ClueSet, difficulty: Difficulty) {
        self.clear_clue_uis();
        self.allocate_clue_uis(difficulty, clue_set);
        self.populate_clue_uis(clue_set);
    }

    fn populate_clue_uis(&mut self, clue_set: &ClueSet) {
        let mut previous_clue: Option<&ClueWithGrouping> = None;
        for (idx, clue_ui) in self.horizontal_clue_uis.iter().enumerate() {
            let clue = clue_set.horizontal_clues().get(idx);
            let is_new_group = match (clue, previous_clue) {
                (Some(clue), Some(previous_clue)) => clue.group != previous_clue.group,
                _ => false,
            };
            clue_ui
                .borrow_mut()
                .set_clue(clue.map(|c| &c.clue), is_new_group);

            previous_clue = clue;
        }
        for (idx, clue_ui) in self.vertical_clue_uis.iter().enumerate() {
            let clue = clue_set.vertical_clues().get(idx);
            let is_new_group = match (clue, previous_clue) {
                (Some(clue), Some(previous_clue)) => clue.group != previous_clue.group,
                _ => false,
            };
            clue_ui
                .borrow_mut()
                .set_clue(clue.map(|c| &c.clue), is_new_group);
            previous_clue = clue;
        }
        let horiz_dim = &self
            .current_layout
            .clues
            .horizontal_clue_panel
            .total_clues_dimensions;
        self.horizontal_grid
            .set_size_request(horiz_dim.width, horiz_dim.height);
    }

    fn set_horiz_completion(&self, completed_clues: &HashSet<usize>) {
        for (idx, clue_ui) in self.horizontal_clue_uis.iter().enumerate() {
            clue_ui
                .borrow_mut()
                .set_completed(completed_clues.contains(&idx));
        }
    }

    fn set_vert_completion(&self, completed_clues: &HashSet<usize>) {
        for (idx, clue_ui) in self.vertical_clue_uis.iter().enumerate() {
            clue_ui
                .borrow_mut()
                .set_completed(completed_clues.contains(&idx));
        }
    }

    fn update_tooltip_visibility(&mut self, enabled: bool) {
        self.tooltips_enabled = enabled;
        for clue_ui in &self.horizontal_clue_uis {
            clue_ui.borrow_mut().set_tooltips_enabled(enabled);
        }
        for clue_ui in &self.vertical_clue_uis {
            clue_ui.borrow_mut().set_tooltips_enabled(enabled);
        }
    }

    fn update_layout(&mut self, layout: &LayoutConfiguration) {
        self.current_layout = layout.clone();

        // Update horizontal clues grid
        self.horizontal_grid
            .set_row_spacing(layout.clues.horizontal_clue_panel.row_spacing as u32);
        self.horizontal_grid
            .set_column_spacing(layout.clues.horizontal_clue_panel.column_spacing as u32);
        self.horizontal_grid
            .set_margin_start(layout.clues.horizontal_clue_panel.left_margin);
        let horiz_dim = &self
            .current_layout
            .clues
            .horizontal_clue_panel
            .total_clues_dimensions;
        self.horizontal_grid
            .set_size_request(horiz_dim.width, horiz_dim.height);

        // Update vertical clues grid
        self.vertical_grid.set_row_spacing(0);
        self.vertical_grid
            .set_column_spacing(layout.clues.vertical_clue_panel.column_spacing as u32);
        self.vertical_grid
            .set_margin_top(layout.clues.vertical_clue_panel.margin_top);
        self.vertical_grid.set_size_request(
            -1,
            self.current_layout
                .clues
                .vertical_clue_panel
                .total_clues_height,
        );

        // Update individual clue UIs
        for clue_ui in self.horizontal_clue_uis.iter_mut() {
            clue_ui.borrow_mut().update_layout(layout);
        }
        for clue_ui in self.vertical_clue_uis.iter_mut() {
            clue_ui.borrow_mut().update_layout(layout);
        }
    }

    pub fn calc_clues_per_column(difficulty: Difficulty) -> usize {
        let n_rows = difficulty.grid_size();
        n_rows * 2
    }

    fn set_clue_selected(&self, clue_selection: &Option<ClueSelection>) {
        // dispatch to all clues
        for clue_ui in &self.horizontal_clue_uis {
            clue_ui.borrow_mut().set_selected(clue_selection);
        }
        for clue_ui in &self.vertical_clue_uis {
            clue_ui.borrow_mut().set_selected(clue_selection);
        }
    }
}
