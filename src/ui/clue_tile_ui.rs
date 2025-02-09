use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::destroyable::Destroyable;
use crate::model::{Clue, ClueType, CluesSizing, HorizontalClueType, Tile, VerticalClueType};
use gtk4::glib::{timeout_add_local_once, SourceId};
use gtk4::prelude::*;
use gtk4::{Frame, Image, Overlay, Widget};

use super::ImageSet;

enum Decoration {
    Negative,
    Maybe,
}

enum ClueTileContents {
    TileAssertion(Tile, Option<Decoration>),
    LeftOf, /* show the LeftOf icon */
    None,
}

pub struct ClueTileUI {
    pub frame: Frame,
    pub overlay: Overlay,
    image: Image,       // Main tile image
    x_image: Image,     // Red X for negative assertions
    maybe_image: Image, // Question mark for maybe assertions
    left_of: Image,     // LeftOf clues
    highlight_frame: Arc<Frame>,
    decoration_frame: Arc<Frame>, // For red border on negative assertions or yellow for maybe
    resources: Rc<ImageSet>,
    highlight_timeout: Rc<RefCell<Option<SourceId>>>, // Track active highlight timeout
    clue: Option<Clue>,
    idx: usize,
}

impl ClueTileUI {
    pub fn new(resources: Rc<ImageSet>, clue: Option<Clue>, idx: usize) -> Self {
        let frame = Frame::builder()
            .visible(true)
            .name("clue-cell")
            .css_classes(["clue-cell"])
            .hexpand(false)
            .vexpand(false)
            .build();

        let image = Image::new();

        let x_image = Image::builder()
            .visible(false)
            .css_classes(["negative-assertion-x"])
            .halign(gtk4::Align::Start)
            .valign(gtk4::Align::Start)
            .hexpand(false)
            .vexpand(false)
            .build();

        let maybe_image = Image::new();
        maybe_image.set_visible(false);
        maybe_image.set_css_classes(&["maybe-assertion-mark"]);
        maybe_image.set_halign(gtk4::Align::Start);
        maybe_image.set_valign(gtk4::Align::Start);

        let left_of = Image::new();
        left_of.set_visible(false);
        left_of.set_halign(gtk4::Align::Center);
        left_of.set_valign(gtk4::Align::Center);

        let highlight_frame = Frame::new(None);
        highlight_frame.set_visible(false);

        let decoration_frame = Frame::new(None);
        decoration_frame.set_visible(false);

        let overlay = Overlay::new();
        overlay.set_css_classes(&["clue-overlay"]);
        overlay.set_child(Some(&image));
        overlay.add_overlay(&x_image);
        overlay.add_overlay(&maybe_image);
        overlay.add_overlay(&left_of);
        overlay.add_overlay(highlight_frame.upcast_ref::<Widget>());
        overlay.add_overlay(decoration_frame.upcast_ref::<Widget>());

        frame.set_child(Some(&overlay));

        Self {
            frame,
            overlay,
            image,
            x_image,
            maybe_image,
            left_of,
            highlight_frame: Arc::new(highlight_frame),
            decoration_frame: Arc::new(decoration_frame),
            resources,
            highlight_timeout: Rc::new(RefCell::new(None)),
            clue,
            idx,
        }
    }

    pub fn update_layout(&self, layout: &CluesSizing) {
        // Update main image size
        self.image.set_pixel_size(layout.clue_tile_size.width);
        self.left_of.set_pixel_size(layout.clue_tile_size.width);

        // Update decoration sizes and force a queue_resize
        self.x_image
            .set_pixel_size(layout.clue_annotation_size.width);

        self.maybe_image
            .set_pixel_size(layout.clue_annotation_size.width);
    }

    pub fn set_clue(&mut self, clue: Option<&Clue>) {
        self.clue = clue.cloned();
        // reset decorations
        self.highlight_frame.set_visible(false);
        self.maybe_image.set_visible(false);
        self.x_image.set_visible(false);
        self.left_of.set_visible(false);
        self.decoration_frame.set_visible(false);

        self.sync_images();
    }

    fn set_negative(&self) {
        let x_pixbuf = self.resources.get_negative_assertion();
        self.x_image.set_paintable(Some(x_pixbuf.as_ref()));
        self.x_image.set_visible(true);
        self.maybe_image.set_visible(false);
        self.decoration_frame.set_visible(true);
        self.decoration_frame
            .set_css_classes(&["negative-assertion-frame"]);
        self.decoration_frame.set_visible(true);
    }

    fn set_maybe(&self) {
        let paintable = self.resources.get_maybe_assertion();
        self.maybe_image.set_paintable(Some(paintable.as_ref()));
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
        let new_source_id: SourceId;
        {
            let highlight_timeout = Rc::downgrade(&self.highlight_timeout);
            new_source_id = timeout_add_local_once(from_secs, move || {
                if let Some(highlight_timeout_cell) = highlight_timeout.upgrade() {
                    if let Ok(mut highlight_timeout) = highlight_timeout_cell.try_borrow_mut() {
                        // clear the timeout as we've fired, since removing twice causes a panic.
                        *highlight_timeout = None;
                    }
                }
                highlight_frame.remove_css_class("clue-highlight");
                highlight_frame.add_css_class("clue-nohighlight");
            });
        }
        let mut highlight_timeout = self.highlight_timeout.borrow_mut();
        *highlight_timeout = Some(new_source_id);
    }

    pub(crate) fn set_image_set(&mut self, image_set: Rc<ImageSet>) {
        self.resources = image_set;
        self.sync_images();
    }

    fn sync_images(&self) {
        if let Some(clue) = &self.clue {
            let my_tile_info = ClueTileUI::get_clue_tile_contents(clue, self.idx);

            match my_tile_info {
                ClueTileContents::TileAssertion(tile, decoration) => {
                    if let Some(paintable) = self.resources.get_solution_icon(&tile) {
                        self.image.set_paintable(Some(paintable.as_ref()));
                        self.image.set_visible(true);
                    }
                    if let Some(decoration) = decoration {
                        match decoration {
                            Decoration::Negative => self.set_negative(),
                            Decoration::Maybe => self.set_maybe(),
                        }
                    }
                }
                ClueTileContents::LeftOf => {
                    let left_of = self.resources.get_left_of();
                    self.left_of.set_paintable(Some(left_of.as_ref()));
                    self.left_of.set_visible(true);
                    self.image.clear();
                }
                ClueTileContents::None => {
                    self.image.clear();
                }
            }
        }
    }

    fn get_clue_tile_contents(clue: &Clue, idx: usize) -> ClueTileContents {
        match &clue.clue_type {
            ClueType::Horizontal(HorizontalClueType::LeftOf) => match idx {
                0 => ClueTileContents::TileAssertion(clue.assertions[0].tile, None),
                1 => ClueTileContents::LeftOf,
                2 => ClueTileContents::TileAssertion(clue.assertions[1].tile, None),
                _ => ClueTileContents::None,
            },
            ClueType::Vertical(VerticalClueType::OneMatchesEither) => match idx {
                0 => ClueTileContents::TileAssertion(clue.assertions[0].tile, None),
                1 => ClueTileContents::TileAssertion(
                    clue.assertions[1].tile,
                    Some(Decoration::Maybe),
                ),
                2 => ClueTileContents::TileAssertion(
                    clue.assertions[2].tile,
                    Some(Decoration::Maybe),
                ),
                _ => ClueTileContents::None,
            },
            _ => match clue.assertions.get(idx) {
                Some(assertion) if assertion.is_positive() => {
                    ClueTileContents::TileAssertion(assertion.tile, None)
                }
                Some(assertion) if assertion.is_negative() => {
                    ClueTileContents::TileAssertion(assertion.tile, Some(Decoration::Negative))
                }
                _ => ClueTileContents::None,
            },
        }
    }
}

impl Destroyable for ClueTileUI {
    fn destroy(&mut self) {
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
    }
}

impl Drop for ClueTileUI {
    fn drop(&mut self) {
        // Unparent all overlays
        self.overlay.remove_overlay(&self.x_image);
        self.overlay.remove_overlay(&self.maybe_image);
        self.overlay.remove_overlay(&self.left_of);
        self.overlay.remove_overlay(self.highlight_frame.as_ref());
        self.overlay.remove_overlay(self.decoration_frame.as_ref());

        // Unparent the main image from overlay
        self.overlay.set_child(None::<&Widget>);

        // Finally unparent the overlay from frame
        self.frame.set_child(None::<&Widget>);
    }
}
