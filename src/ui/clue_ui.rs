use glib::SignalHandlerId;
use gtk4::gdk::Rectangle;
use gtk4::{
    prelude::*, Align, ApplicationWindow, Box, Frame, Grid, IconTheme, Label, Orientation,
    TextBuffer, TextTagTable, TextView, Widget, WrapMode,
};
use std::{cell::RefCell, rc::Rc};

use crate::destroyable::Destroyable;
use crate::events::EventEmitter;
use crate::model::ClueOrientation;
use crate::model::LayoutConfiguration;
use crate::model::{Clickable, ClueWithAddress, InputEvent};
use crate::model::{Clue, ClueSelection, CluesSizing};
use crate::ui::clue_tile_ui::ClueTileUI;
use crate::ui::template::TemplateParser;
use crate::ui::ImageSet;
use crate::ui::{deferred_size_reallocation, register_left_click_handler};

const NEW_GROUP_CSS_CLASS: &str = "new-group";

#[derive(Debug)]
struct ClueTooltipData {
    clue: Clue,
}

pub struct ClueUI {
    pub frame: Frame,
    window: Rc<ApplicationWindow>,
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

fn tooltip_rect(layout: &CluesSizing) -> Rectangle {
    let width = layout.horizontal_clue_panel.clue_dimensions.width * 2;
    let height = width / 4;
    Rectangle::new(0, 0, width, height)
}

impl ClueUI {
    pub fn new(
        resources: Rc<ImageSet>,
        window: Rc<ApplicationWindow>,
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
            window,
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

        let tooltip_signal = clue_ui.borrow().frame.connect_query_tooltip({
            move |_frame, _x, _y, _keyboard_mode, tooltip| {
                // tooltip.set_tip_area(&tooltip_rect(&clue_ui.borrow().layout));
                if let Some(clue_ui) = weak_clue_ui.upgrade() {
                    let clue_ui = clue_ui.borrow();
                    if let Some(tooltip_widget) = &clue_ui.tooltip_widget {
                        tooltip.set_custom(Some(tooltip_widget));
                        deferred_size_reallocation(tooltip_widget);
                    }
                }
                true
            }
        });
        clue_ui.borrow_mut().tooltip_signal = Some(tooltip_signal);
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

    fn create_tooltip_widget(&self) -> Box {
        let rect = tooltip_rect(&self.layout);
        let tooltip_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(5)
            .margin_start(5)
            .margin_end(5)
            .margin_top(5)
            .margin_bottom(5)
            .hexpand(false)
            .vexpand(false)
            .width_request(rect.width())
            .height_request(rect.height())
            .build();

        if self.tooltip_data.is_none() {
            return tooltip_box;
        }

        let clue_data = self.tooltip_data.as_ref().unwrap();

        // Add title
        let title_box = Box::new(Orientation::Horizontal, 5);
        let title = Label::new(None);
        title.set_markup(&format!("<b>{}</b>", clue_data.clue.clue_type.get_title()));
        title_box.append(&title);
        tooltip_box.append(&title_box);

        let display = WidgetExt::display(self.window.as_ref());
        let theme = IconTheme::for_display(&display);
        let parser = TemplateParser::new(self.resources.clone(), Some(Rc::new(theme)));

        let text_tag_table = TextTagTable::new();
        let buffer = TextBuffer::builder().tag_table(&text_tag_table).build();
        let mut end = buffer.end_iter();
        let tutorial_text = TextView::builder()
            .visible(true)
            .buffer(&buffer)
            .editable(false)
            .cursor_visible(false)
            .halign(Align::Fill)
            .valign(Align::Start)
            .css_classes(["tutorial-text"])
            .vexpand(true)
            .hexpand(true)
            .wrap_mode(WrapMode::Word)
            .build();
        let template = self.clue.clue.description();
        parser.append_to_text_buffer(&tutorial_text, &mut end, &template);

        tooltip_box.append(&tutorial_text);
        deferred_size_reallocation(&tutorial_text);
        deferred_size_reallocation(&tooltip_box);

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
