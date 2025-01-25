use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::model::TileAssertion;
use crate::ui::layout::{CELL_SIZE, ICON_SIZE_SMALL};
use gtk::glib::{timeout_add_local_once, SourceId};
use gtk::prelude::*;
use gtk::{Frame, Image, Overlay};

use super::ResourceSet;

pub struct ClueTileUI {
    pub frame: Frame,
    pub overlay: Overlay,
    pub image: Image,       // Main tile image
    pub x_image: Image,     // Red X for negative assertions
    pub maybe_image: Image, // Question mark for maybe assertions
    pub triple_dot: Image,  // Triple dot for LeftOf clues
    pub highlight_frame: Arc<Frame>,
    pub decoration_frame: Arc<Frame>, // For red border on negative assertions or yellow for maybe
    pub resources: Rc<ResourceSet>,
    highlight_timeout: Rc<RefCell<Option<SourceId>>>, // Track active highlight timeout
}
impl ClueTileUI {
    pub fn new(resources: Rc<ResourceSet>) -> Self {
        let frame = Frame::new(None);
        frame.set_css_classes(&["clue-cell-frame"]);
        frame.set_hexpand(false);
        frame.set_vexpand(false);

        let image = Image::new();
        image.set_pixel_size(CELL_SIZE); // Half size of main solution tiles

        let x_image = Image::new();
        x_image.set_pixel_size(ICON_SIZE_SMALL); // Small enough to fit in corner
        x_image.set_visible(false);
        x_image.set_css_classes(&["negative-assertion-x"]);
        x_image.set_halign(gtk::Align::Start);
        x_image.set_valign(gtk::Align::Start);

        let maybe_image = Image::new();
        maybe_image.set_pixel_size(ICON_SIZE_SMALL); // Small enough to fit in corner
        maybe_image.set_visible(false);
        maybe_image.set_css_classes(&["maybe-assertion-mark"]);
        maybe_image.set_halign(gtk::Align::Start);
        maybe_image.set_valign(gtk::Align::Start);

        let triple_dot = Image::new();
        triple_dot.set_pixel_size(CELL_SIZE); // Same size as clue tiles
        triple_dot.set_visible(false);
        triple_dot.set_halign(gtk::Align::Center); // Center in the cell
        triple_dot.set_valign(gtk::Align::Center);

        let highlight_frame = Frame::new(None);
        highlight_frame.set_visible(false);

        let decoration_frame = Frame::new(None);
        decoration_frame.set_visible(false);

        let overlay = Overlay::new();
        overlay.set_css_classes(&["clue-overlay"]);
        overlay.set_child(Some(&image));
        overlay.add_overlay(&x_image);
        overlay.add_overlay(&maybe_image);
        overlay.add_overlay(&triple_dot);
        overlay.add_overlay(&highlight_frame);
        overlay.add_overlay(&decoration_frame);

        frame.set_child(Some(&overlay));

        Self {
            frame,
            overlay,
            image,
            x_image,
            maybe_image,
            triple_dot,
            highlight_frame: Arc::new(highlight_frame),
            resources,
            decoration_frame: Arc::new(decoration_frame),
            highlight_timeout: Rc::new(RefCell::new(None)),
        }
    }

    pub fn set_tile(&self, assertion: Option<&TileAssertion>) {
        // reset decorations
        self.highlight_frame.set_visible(false);
        self.maybe_image.set_visible(false);
        self.x_image.set_visible(false);
        self.triple_dot.set_visible(false);
        self.decoration_frame.set_visible(false);

        if let Some(assertion) = assertion {
            if let Some(pixbuf) = self.resources.get_tile_icon(&assertion.tile) {
                self.image.set_from_pixbuf(Some(&pixbuf));
                self.image.set_visible(true);
            }
            if !assertion.assertion {
                self.set_negative();
            }
        } else {
            self.image.set_from_pixbuf(None);
        }
    }

    pub fn show_triple_dot(&self) {
        let dot_pixbuf = self.resources.get_triple_dot();
        self.triple_dot.set_from_pixbuf(Some(&dot_pixbuf));
        self.triple_dot.set_visible(true);
    }

    fn set_negative(&self) {
        let x_pixbuf = self.resources.get_negative_assertion();
        self.x_image.set_from_pixbuf(Some(&x_pixbuf));
        self.x_image.set_visible(true);
        self.maybe_image.set_visible(false);
        self.decoration_frame.set_visible(true);
        self.decoration_frame
            .set_css_classes(&["negative-assertion-frame"]);
        self.decoration_frame.set_visible(true);
    }

    pub(crate) fn set_maybe(&self) {
        let maybe_pixbuf = self.resources.get_maybe_assertion();
        self.maybe_image.set_from_pixbuf(Some(&maybe_pixbuf));
        self.maybe_image.set_visible(true);
        self.x_image.set_visible(false);
        self.decoration_frame.set_visible(true);
        self.decoration_frame
            .set_css_classes(&["maybe-assertion-frame"]);
        self.decoration_frame.set_visible(true);
    }

    pub(crate) fn highlight_for(&self, from_secs: std::time::Duration) {
        // Cancel any existing timeout
        if let Some(source_id) = self.highlight_timeout.take() {
            let _ = std::panic::catch_unwind(move || {
                source_id.remove();
            });
        }

        self.highlight_frame.remove_css_class("clue-nohighlight");
        self.highlight_frame.add_css_class("clue-highlight");
        self.highlight_frame.set_visible(true);
        let highlight_frame = self.highlight_frame.clone();
        let highlight_timeout_cell = Rc::clone(&self.highlight_timeout);
        let source_id = timeout_add_local_once(from_secs, move || {
            if let Ok(mut highlight_timeout) = highlight_timeout_cell.try_borrow_mut() {
                *highlight_timeout = None;
            }
            highlight_frame.remove_css_class("clue-highlight");
            highlight_frame.add_css_class("clue-nohighlight");
        });
        if let Ok(mut highlight_timeout) = self.highlight_timeout.try_borrow_mut() {
            *highlight_timeout = Some(source_id);
        }
    }
}

impl Drop for ClueTileUI {
    fn drop(&mut self) {
        // Cancel any pending highlight timeout
        if let Some(source_id) = self.highlight_timeout.take() {
            log::trace!(
                "Dropping ClueCellUI, removing highlight timeout {:?}",
                source_id
            );
            let _ = std::panic::catch_unwind(move || {
                source_id.remove();
            });
        }

        // Unparent all overlays
        self.overlay.remove_overlay(&self.x_image);
        self.overlay.remove_overlay(&self.maybe_image);
        self.overlay.remove_overlay(&self.triple_dot);
        self.overlay.remove_overlay(self.highlight_frame.as_ref());
        self.overlay.remove_overlay(self.decoration_frame.as_ref());

        // Unparent the main image from overlay
        self.overlay.set_child(None::<&gtk::Widget>);

        // Finally unparent the overlay from frame
        self.frame.set_child(None::<&gtk::Widget>);
    }
}
