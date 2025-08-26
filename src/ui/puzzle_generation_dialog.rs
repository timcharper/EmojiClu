use std::rc::Rc;

use gtk4::{
    prelude::{BoxExt, GtkWindowExt},
    ApplicationWindow, Label, Spinner,
};

use crate::{
    destroyable::Destroyable,
    events::EventHandler,
    model::{GameBoardChangeReason, GameEngineEvent},
};
use fluent_i18n::t;

pub struct PuzzleGenerationDialog {
    window: Rc<ApplicationWindow>,
    dialog: Option<gtk4::Window>,
}

impl PuzzleGenerationDialog {
    pub fn new(window: &Rc<ApplicationWindow>) -> Rc<std::cell::RefCell<Self>> {
        let dialog = Rc::new(std::cell::RefCell::new(Self {
            window: window.clone(),
            dialog: None,
        }));
        dialog
    }

    fn show_dialog(&mut self) {
        if self.dialog.is_some() {
            return; // Dialog already shown
        }

        let content_area = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(15)
            .margin_bottom(20)
            .margin_top(20)
            .margin_start(30)
            .margin_end(30)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build();

        let dialog = gtk4::Window::builder()
            .transient_for(self.window.as_ref())
            .child(&content_area)
            .modal(true)
            .resizable(false)
            .deletable(false)
            .build();

        // Add spinner
        let spinner = Spinner::builder()
            .width_request(32)
            .height_request(32)
            .build();
        spinner.start();
        content_area.append(&spinner);

        // Add label
        let label = Label::new(Some(&t!("generating-puzzle")));
        content_area.append(&label);

        dialog.present();
        self.dialog = Some(dialog);
    }

    fn hide_dialog(&mut self) {
        if let Some(dialog) = self.dialog.take() {
            dialog.close();
        }
    }
}

impl Destroyable for PuzzleGenerationDialog {
    fn destroy(&mut self) {
        self.hide_dialog();
    }
}

impl EventHandler<GameEngineEvent> for PuzzleGenerationDialog {
    fn handle_event(&mut self, event: &GameEngineEvent) {
        // delegate to the existing handler method
        match event {
            GameEngineEvent::PuzzleGenerationStarted => self.show_dialog(),
            GameEngineEvent::GameBoardUpdated { change_reason, .. } => {
                if *change_reason == GameBoardChangeReason::NewGame {
                    self.hide_dialog();
                }
            }
            _ => {}
        }
    }
}
