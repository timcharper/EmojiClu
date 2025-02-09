use glib::SignalHandlerId;
use gtk4::{prelude::*, Box, Frame, Grid, Label, Orientation, Widget};
use std::{cell::RefCell, rc::Rc};

use crate::destroyable::Destroyable;
use crate::events::EventEmitter;
use crate::model::{Clickable, ClueWithAddress, InputEvent};
use crate::model::{Clue, ClueSelection, CluesSizing};
use crate::model::{ClueOrientation, TileAssertion};
use crate::model::{ClueType, HorizontalClueType, LayoutConfiguration, VerticalClueType};
use crate::ui::clue_tile_ui::ClueTileUI;
use crate::ui::register_left_click_handler;
use crate::ui::ImageSet;

#[derive(Debug)]
struct ClueTooltipData {
    clue: Clue,
}

#[derive(Debug)]
enum TemplateElement {
    Label(String),
    Tile(usize),
}

const NEW_GROUP_CSS_CLASS: &str = "new-group";

pub struct ClueUI {
    pub frame: Frame,
    grid: Grid,
    clue_tiles: Vec<ClueTileUI>,
    tooltip_data: Option<ClueTooltipData>,
    tooltip_widget: Option<Box>,
    resources: Rc<ImageSet>,
    layout: CluesSizing,
    tooltip_signal: Option<SignalHandlerId>,
    input_event_emitter: Rc<EventEmitter<InputEvent>>,
    clue: ClueWithAddress,
    gesture_right: Option<gtk4::GestureClick>,
    gesture_left: Option<gtk4::GestureClick>,
    clue_spotlight_enabled: bool,
}

impl ClueUI {
    pub fn new(
        resources: Rc<ImageSet>,
        clue: ClueWithAddress,
        layout: CluesSizing,
        input_event_emitter: EventEmitter<InputEvent>,
        clue_spotlight_enabled: bool,
        tooltips_enabled: bool,
    ) -> Rc<RefCell<Self>> {
        let orientation = clue.address().orientation;
        let frame = Frame::builder()
            .name(&format!("clue-frame-{}", orientation))
            .css_classes(["clue-frame"])
            // Set up tooltip handling
            .has_tooltip(tooltips_enabled)
            .build();

        let grid = Grid::builder()
            .name("clue-cell-grid")
            .css_classes(["clue-cell-grid"])
            .row_spacing(0)
            .column_spacing(0)
            .build();

        // Create the three cells for this clue
        let mut cells = Vec::new();
        for i in 0..3 {
            let clue_cell = ClueTileUI::new(Rc::clone(&resources), Some(clue.clue.clone()), i);
            match orientation {
                ClueOrientation::Horizontal => {
                    grid.attach(&clue_cell.frame, i as i32, 0, 1, 1);
                }
                ClueOrientation::Vertical => {
                    grid.attach(&clue_cell.frame, 0, i as i32, 1, 1);
                }
            }
            cells.push(clue_cell);
        }

        // Add content to root overlay instead of frame directly
        frame.set_child(Some(&grid));

        let clue_ui = Self {
            frame,
            grid,
            clue_tiles: cells,
            tooltip_data: None,
            tooltip_widget: None,
            resources,
            layout,
            tooltip_signal: None,
            input_event_emitter: Rc::new(input_event_emitter),
            clue,
            gesture_right: None,
            gesture_left: None,
            clue_spotlight_enabled,
        };
        let clue_ui_ref = Rc::new(RefCell::new(clue_ui));
        ClueUI::wire_handlers(clue_ui_ref.clone());
        ClueUI::wire_tooltip_handlers(clue_ui_ref.clone());
        clue_ui_ref
            .borrow_mut()
            .update_spotlight_enabled(clue_spotlight_enabled);
        clue_ui_ref
    }

    fn wire_tooltip_handlers(clue_ui: Rc<RefCell<Self>>) {
        let weak_clue_ui = Rc::downgrade(&clue_ui);
        let mut clue_ui = clue_ui.borrow_mut();

        let tooltip_signal =
            clue_ui
                .frame
                .connect_query_tooltip(move |_frame, _x, _y, _keyboard_mode, tooltip| {
                    if let Some(clue_ui) = weak_clue_ui.upgrade() {
                        let clue_ui = clue_ui.borrow();
                        if let Some(tooltip_widget) = &clue_ui.tooltip_widget {
                            tooltip.set_custom(Some(tooltip_widget));
                        }
                    }
                    true
                });
        clue_ui.tooltip_signal = Some(tooltip_signal);
    }

    pub fn set_tooltips_enabled(&mut self, enabled: bool) {
        self.frame.set_has_tooltip(enabled);
    }

    fn wire_handlers(clue_ui: Rc<RefCell<Self>>) {
        let weak_clue_ui = Rc::downgrade(&clue_ui);
        let mut clue_ui = clue_ui.borrow_mut();
        let clue_address = clue_ui.clue.address();

        // Right click handler

        let gesture_right = gtk4::GestureClick::new();
        gesture_right.set_button(3);
        gesture_right.connect_pressed({
            let weak_clue_ui = weak_clue_ui.clone();

            move |gesture, _, _, _| {
                if let Some(clue_ui) = weak_clue_ui.upgrade() {
                    let clue_ui = clue_ui.borrow();
                    clue_ui
                        .input_event_emitter
                        .emit(InputEvent::RightClick(Clickable::Clue(clue_address)));
                    gesture.set_state(gtk4::EventSequenceState::Claimed);
                }
            }
        });
        clue_ui.frame.add_controller(gesture_right.clone());
        clue_ui.gesture_right = Some(gesture_right);

        let gesture_left = gtk4::GestureClick::new();
        gesture_left.set_button(1);
        register_left_click_handler(clue_ui.input_event_emitter.clone(), &gesture_left, {
            move |_, _, _, _| Some(Clickable::Clue(clue_address))
        });

        clue_ui.frame.add_controller(gesture_left.clone());
        clue_ui.gesture_left = Some(gesture_left);
    }

    fn apply_layout(&self) {
        match self.clue.address().orientation {
            ClueOrientation::Horizontal => {
                self.frame.set_size_request(
                    self.layout.horizontal_clue_panel.clue_dimensions.width,
                    self.layout.horizontal_clue_panel.clue_dimensions.height,
                );
            }
            ClueOrientation::Vertical => {
                self.frame.set_size_request(
                    self.layout.vertical_clue_panel.clue_dimensions.width,
                    self.layout.vertical_clue_panel.clue_dimensions.height,
                );

                if self.frame.has_css_class(NEW_GROUP_CSS_CLASS) {
                    self.frame
                        .set_margin_start(self.layout.vertical_clue_panel.group_spacing);
                } else {
                    self.frame.set_margin_start(0);
                }
            }
        }
        self.grid.set_margin_bottom(self.layout.clue_padding);
        self.grid.set_margin_top(self.layout.clue_padding);
        self.grid.set_margin_start(self.layout.clue_padding);
        self.grid.set_margin_end(self.layout.clue_padding);

        // Update individual tile sizes
        for cell in &self.clue_tiles {
            cell.update_layout(&self.layout);
        }
    }

    pub(crate) fn update_layout(&mut self, layout: &LayoutConfiguration) {
        self.layout = layout.clues.clone();
        self.apply_layout();
    }

    pub fn set_clue(&mut self, clue: Option<&Clue>, is_new_group: bool) {
        if let Some(clue) = clue {
            let tooltip_data = ClueTooltipData { clue: clue.clone() };
            self.tooltip_data = Some(tooltip_data);

            // Create new tooltip widget when clue changes
            let new_tooltip = self.create_tooltip_widget();
            self.tooltip_widget = Some(new_tooltip);

            for clue_tile in &mut self.clue_tiles {
                clue_tile.set_clue(Some(clue));
            }
            self.frame.set_visible(true);
            if clue.is_vertical() && is_new_group {
                self.frame.add_css_class(NEW_GROUP_CSS_CLASS);
            } else {
                self.frame.remove_css_class(NEW_GROUP_CSS_CLASS);
            }
            self.apply_layout();
        } else {
            self.tooltip_data = None;
            self.tooltip_widget = None;
            // clear
            for clue_tile in &mut self.clue_tiles {
                clue_tile.set_clue(None);
            }

            self.frame.set_visible(false);
            self.frame.remove_css_class(NEW_GROUP_CSS_CLASS);
        }
    }

    pub fn highlight_for(&self, from_secs: std::time::Duration) {
        for cell in &self.clue_tiles {
            cell.highlight_for(from_secs);
        }
    }

    pub fn set_completed(&self, completed: bool) {
        if completed {
            self.frame.add_css_class("completed");
        } else {
            self.frame.remove_css_class("completed");
        }
    }

    fn parse_template_elements(&self, template: &str) -> Vec<TemplateElement> {
        let mut elements = Vec::new();
        let mut current_text = String::new();
        let mut chars = template.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                // If we have accumulated text, add it as a label
                if !current_text.is_empty() {
                    elements.push(TemplateElement::Label(current_text.clone()));
                    current_text.clear();
                }

                // Parse the token
                let mut token = String::new();
                while let Some(&next_c) = chars.peek() {
                    chars.next();
                    if next_c == '}' {
                        break;
                    }
                    token.push(next_c);
                }

                // Handle tile tokens
                if let Ok(tile_idx) = token.trim_start_matches('t').parse::<usize>() {
                    elements.push(TemplateElement::Tile(tile_idx));
                }
            } else {
                current_text.push(c);
            }
        }

        // Add any remaining text
        if !current_text.is_empty() {
            elements.push(TemplateElement::Label(current_text));
        }

        elements
    }

    fn parse_template(&self, template: &str, clue_data: &ClueTooltipData) -> Box {
        let box_container = Box::new(Orientation::Horizontal, 5);

        // Transform TemplateElements into GTK widgets
        self.parse_template_elements(template)
            .into_iter()
            .flat_map(|element| match element {
                TemplateElement::Label(text) => {
                    let label = Label::new(None);
                    label.set_markup(&text);
                    label.set_wrap(true);
                    label.set_max_width_chars(40);
                    Some(label.upcast::<Widget>())
                }
                TemplateElement::Tile(tile_idx) => {
                    // Get the tile assertion and create an image if it exists
                    self.clue_tiles
                        .get(tile_idx)
                        .and_then(|_| clue_data.clue.assertions.get(tile_idx))
                        .and_then(|ta| self.resources.get_candidate_icon(&ta.tile))
                        .map(|pixbuf| {
                            let image = gtk4::Image::from_pixbuf(Some(&pixbuf));
                            image.upcast::<Widget>()
                        })
                }
            })
            .for_each(|widget| box_container.append(&widget));

        box_container
    }

    fn create_tooltip_widget(&self) -> Box {
        let tooltip_box = Box::new(Orientation::Vertical, 5);
        if self.tooltip_data.is_none() {
            return tooltip_box;
        }

        let clue_data = self.tooltip_data.as_ref().unwrap();

        tooltip_box.set_margin_start(5);
        tooltip_box.set_margin_end(5);
        tooltip_box.set_margin_top(5);
        tooltip_box.set_margin_bottom(5);

        // Add title
        let title_box = Box::new(Orientation::Horizontal, 5);
        let title = Label::new(None);
        title.set_markup(&format!("<b>{}</b>", clue_data.clue.clue_type.get_title()));
        title_box.append(&title);
        tooltip_box.append(&title_box);

        // Add description with example
        let desc_box = Box::new(Orientation::Horizontal, 5);

        // Create a temporary UI just for parsing templates
        match &clue_data.clue.clue_type {
            ClueType::Horizontal(horiz) => match horiz {
                HorizontalClueType::TwoAdjacent | HorizontalClueType::ThreeAdjacent => {
                    // Create template string with tiles and description
                    let mut template = String::new();
                    for (i, _) in clue_data.clue.assertions.iter().enumerate() {
                        if i > 0 {
                            template.push(' ');
                        }
                        template.push_str(&format!("{{t{}}}", i));
                    }
                    template.push_str(" are adjacent (forward, backward).");
                    desc_box.append(&self.parse_template(&template, clue_data));
                }
                HorizontalClueType::TwoApartNotMiddle => {
                    let template = "{t0} is two away from {t2}, without {t1} in the middle (forward, backward).";
                    desc_box.append(&self.parse_template(&template, clue_data));
                }
                HorizontalClueType::LeftOf => {
                    let template = "{t0} is left of {t1} (any number of tiles in between).";
                    desc_box.append(&self.parse_template(&template, clue_data));
                }
                HorizontalClueType::NotAdjacent => {
                    let template = "{t0} is not next to {t1} (forward, backward).";
                    desc_box.append(&self.parse_template(&template, clue_data));
                }
            },
            ClueType::Vertical(vert) => match vert {
                VerticalClueType::ThreeInColumn | VerticalClueType::TwoInColumn => {
                    let mut template = String::new();
                    for (i, _) in clue_data.clue.assertions.iter().enumerate() {
                        if i > 0 {
                            template.push(' ');
                        }
                        template.push_str(&format!("{{t{}}}", i));
                    }
                    template.push_str(" are in the same column.");
                    desc_box.append(&self.parse_template(&template, clue_data));
                }
                VerticalClueType::TwoInColumnWithout => {
                    let clue_assertions: Vec<(usize, &TileAssertion)> =
                        clue_data.clue.assertions.iter().enumerate().collect();

                    let positive_assertion_positions = clue_assertions
                        .iter()
                        .filter(|(_, ta)| ta.assertion)
                        .map(|(i, _)| format!("t{}", i))
                        .collect::<Vec<_>>();

                    let negative_assertion_positions = clue_assertions
                        .iter()
                        .filter(|(_, ta)| !ta.assertion)
                        .map(|(i, _)| format!("t{}", i))
                        .collect::<Vec<_>>();

                    assert!(positive_assertion_positions.len() == 2);
                    assert!(negative_assertion_positions.len() == 1);

                    let template = format!(
                        "{{{}}} and {{{}}} are in the same column, but {{{}}} isn't.",
                        positive_assertion_positions[0],
                        positive_assertion_positions[1],
                        negative_assertion_positions[0],
                    );
                    desc_box.append(&self.parse_template(&template, clue_data));
                }

                VerticalClueType::NotInSameColumn => {
                    let template = "{t0} is not in the same column as {t1}";
                    desc_box.append(&self.parse_template(&template, clue_data));
                }
                VerticalClueType::OneMatchesEither => {
                    let template =
                        "{t0} is either in the same column as {t1} or {t2}, but not both.";
                    desc_box.append(&self.parse_template(&template, clue_data));
                }
            },
        }

        tooltip_box.append(&desc_box);

        tooltip_box
    }

    pub fn set_selected(&self, clue_selection: &Option<ClueSelection>) {
        let my_clue_selected = if let Some(clue_selection) = clue_selection {
            clue_selection.clue == self.clue
        } else {
            false
        };
        let any_clue_focused = if let Some(clue_selection) = clue_selection {
            clue_selection.is_focused
        } else {
            false
        };

        if my_clue_selected {
            self.frame.add_css_class("selected");
        } else {
            self.frame.remove_css_class("selected");
        }

        if any_clue_focused && !my_clue_selected {
            self.frame.add_css_class("not-focused");
        } else {
            self.frame.remove_css_class("not-focused");
        }
    }

    pub(crate) fn update_spotlight_enabled(&mut self, enabled: bool) {
        self.clue_spotlight_enabled = enabled;
    }

    pub(crate) fn set_image_set(&mut self, image_set: Rc<ImageSet>) {
        self.resources = image_set;
        self.sync_images();
    }

    fn sync_images(&mut self) {
        for clue_tile in &mut self.clue_tiles {
            clue_tile.set_image_set(self.resources.clone());
        }
    }
}

impl Destroyable for ClueUI {
    fn destroy(&mut self) {
        log::trace!(target: "clue_ui", "Destroying clue UI");

        // Remove the click gesture controller
        if let Some(gesture_right) = self.gesture_right.take() {
            self.frame.remove_controller(&gesture_right);
        }
        if let Some(gesture_left) = self.gesture_left.take() {
            self.frame.remove_controller(&gesture_left);
        }
        log::trace!(target: "clue_ui", "Removed gesture controllers");

        if let Some(tooltip_signal) = self.tooltip_signal.take() {
            self.frame.disconnect(tooltip_signal);
        }
        log::trace!(target: "clue_ui", "Disconnected tooltip_signal");
        for cell in &mut self.clue_tiles {
            cell.destroy();
        }
        log::trace!(target: "clue_ui", "Destroyed clue UI");
    }
}

impl Drop for ClueUI {
    fn drop(&mut self) {
        // Clear tooltip data and widget first to drop any resource references
        self.tooltip_data = None;
        self.tooltip_widget = None;

        // Remove all cells from the grid
        if let Some(grid) = self.frame.child().and_then(|w| w.downcast::<Grid>().ok()) {
            for cell in &self.clue_tiles {
                if cell.frame.parent().is_some() {
                    grid.remove(&cell.frame);
                }
            }
            // Unparent the grid from frame
            self.frame.set_child(None::<&Widget>);
        }
    }
}
