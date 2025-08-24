use std::{cell::Cell, rc::Rc};

use glib::Propagation;
use gtk4::{
    gdk::Key,
    prelude::{BoxExt, ButtonExt, GtkWindowExt, WidgetExt},
    ApplicationWindow, EventControllerKey, Label,
};

use crate::{events::EventEmitter, model::GameEngineCommand};
use fluent_i18n::t;

pub struct NotQuiteRightDialog {
    window: Rc<ApplicationWindow>,
    game_engine_command_emitter: EventEmitter<GameEngineCommand>,
}

impl NotQuiteRightDialog {
    pub fn new(
        window: &Rc<ApplicationWindow>,
        game_engine_command_emitter: EventEmitter<GameEngineCommand>,
    ) -> Self {
        Self {
            window: window.clone(),
            game_engine_command_emitter,
        }
    }

    pub fn show(&self) {
        let content_area = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(10)
            .margin_bottom(10)
            .margin_top(10)
            .margin_start(20)
            .margin_end(20)
            .build();
        let dialog = gtk4::Window::builder()
            .transient_for(self.window.as_ref())
            .child(&content_area)
            .modal(true)
            .build();

        content_area.append(&Label::new(Some(
            "Sorry, that's not quite right. Click OK to rewind to the last correct state.",
        )));

        let buttons = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .halign(gtk4::Align::End)
            .spacing(10)
            .build();
        content_area.append(&buttons);

        let cancel_button = gtk4::Button::builder().label(&t!("cancel")).build();
        buttons.append(&cancel_button);
        let ok_button = gtk4::Button::builder().label(&t!("ok")).build();
        buttons.append(&ok_button);

        let ok_clicked = Rc::new(Cell::new(false));

        cancel_button.connect_clicked({
            let dialog = dialog.clone();
            move |_| {
                dialog.close();
            }
        });

        ok_button.connect_clicked({
            let dialog = dialog.clone();
            let ok_clicked = ok_clicked.clone();
            move |_| {
                ok_clicked.set(true);
                dialog.close();
            }
        });

        let key_controller = EventControllerKey::new();
        key_controller.connect_key_pressed({
            let dialog = dialog.clone();
            move |_, keyval, _, _| {
                if keyval == Key::Escape {
                    dialog.close();
                    return Propagation::Stop;
                }
                Propagation::Proceed
            }
        });
        dialog.add_controller(key_controller);

        dialog.connect_close_request({
            let game_engine_command_emitter = self.game_engine_command_emitter.clone();
            move |_| {
                if ok_clicked.get() {
                    game_engine_command_emitter.emit(GameEngineCommand::RewindLastGood);
                } else {
                    game_engine_command_emitter.emit(GameEngineCommand::Undo);
                }
                Propagation::Proceed
            }
        });

        dialog.present();
    }
}
