use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use gio::{Menu, SimpleAction};
use glib::prelude::ToVariant;
use gtk4::{prelude::*, ApplicationWindow};

use crate::{
    destroyable::Destroyable,
    events::{EventEmitter, Unsubscriber},
    game::settings::Settings,
    model::{GameStateEvent, GlobalEvent},
};

pub struct SettingsMenuUI {
    window: Rc<ApplicationWindow>,
    settings_menu: Menu,
    action_toggle_tooltips: SimpleAction,
    action_toggle_xray: SimpleAction,
    action_toggle_touch_controls: SimpleAction,
    game_state_subscription: Option<Unsubscriber<GameStateEvent>>,
    settings_ref: Rc<RefCell<Settings>>,
    global_event_emitter: EventEmitter<GlobalEvent>,
}

impl Destroyable for SettingsMenuUI {
    fn destroy(&mut self) {
        if let Some(subscription) = self.game_state_subscription.take() {
            subscription.unsubscribe();
        }
        // Remove actions from window
        self.window
            .remove_action(&self.action_toggle_tooltips.name());
        self.window.remove_action(&self.action_toggle_xray.name());
        self.window
            .remove_action(&self.action_toggle_touch_controls.name());
    }
}

impl SettingsMenuUI {
    pub fn new(
        window: Rc<ApplicationWindow>,
        global_event_emitter: EventEmitter<GlobalEvent>,
        settings_ref: Rc<RefCell<Settings>>,
    ) -> Rc<RefCell<Self>> {
        let settings_menu = Menu::new();
        settings_menu.append(Some("Show Clue Tooltips"), Some("win.toggle-tooltips"));
        settings_menu.append(
            Some("Touch Screen Controls"),
            Some("win.toggle-touch-controls"),
        );

        if Settings::is_debug_mode() {
            settings_menu.append(Some("Show Clue X-Ray"), Some("win.toggle-xray"));
        }

        let action_toggle_tooltips: SimpleAction;
        let action_toggle_xray: SimpleAction;
        let action_toggle_touch_controls: SimpleAction;

        {
            let settings = settings_ref.borrow();
            // Create toggle tooltips action
            action_toggle_tooltips = SimpleAction::new_stateful(
                "toggle-tooltips",
                None,
                &settings.clue_tooltips_enabled.to_variant(),
            );

            // Create x-ray mode action
            action_toggle_xray = SimpleAction::new_stateful(
                "toggle-xray",
                None,
                &settings.clue_xray_enabled.to_variant(),
            );

            // Create touch screen controls action
            action_toggle_touch_controls = SimpleAction::new_stateful(
                "toggle-touch-controls",
                None,
                &settings.touch_screen_controls.to_variant(),
            );
        }

        let settings_menu_ui = Rc::new(RefCell::new(Self {
            window: window.clone(),
            settings_menu,
            action_toggle_tooltips,
            action_toggle_xray,
            action_toggle_touch_controls,
            game_state_subscription: None,
            settings_ref: settings_ref,
            global_event_emitter: global_event_emitter.clone(),
        }));

        // Connect actions
        Self::connect_actions(settings_menu_ui.clone(), window.clone());

        settings_menu_ui
    }

    fn connect_actions(settings_menu_ui: Rc<RefCell<Self>>, window: Rc<ApplicationWindow>) {
        let weak_settings_menu_ui = Rc::downgrade(&settings_menu_ui);
        let settings_menu_ui_ref = settings_menu_ui.borrow();

        // Connect toggle tooltips action
        {
            let weak_settings_menu_ui = Weak::clone(&weak_settings_menu_ui);
            settings_menu_ui_ref
                .action_toggle_tooltips
                .connect_activate(move |action, _| {
                    let current_state = action.state().unwrap().get::<bool>().unwrap();
                    let new_state = !current_state;
                    action.set_state(&new_state.to_variant());
                    if let Some(settings_menu_ui) = weak_settings_menu_ui.upgrade() {
                        settings_menu_ui
                            .borrow_mut()
                            .set_tooltips_enabled(new_state);
                    }
                });
            window.add_action(&settings_menu_ui_ref.action_toggle_tooltips);
        }

        // Connect x-ray mode action
        {
            let weak_settings_menu_ui = Weak::clone(&weak_settings_menu_ui);
            settings_menu_ui_ref
                .action_toggle_xray
                .connect_activate(move |action, _| {
                    let current_state = action.state().unwrap().get::<bool>().unwrap();
                    let new_state = !current_state;
                    action.set_state(&new_state.to_variant());
                    if let Some(settings_menu_ui) = weak_settings_menu_ui.upgrade() {
                        settings_menu_ui
                            .borrow_mut()
                            .set_clue_xray_enabled(new_state);
                    }
                });
            window.add_action(&settings_menu_ui_ref.action_toggle_xray);
        }

        // Connect touch screen controls action
        {
            let weak_settings_menu_ui = Weak::clone(&weak_settings_menu_ui);
            settings_menu_ui_ref
                .action_toggle_touch_controls
                .connect_activate(move |action, _| {
                    let current_state = action.state().unwrap().get::<bool>().unwrap();
                    let new_state = !current_state;
                    action.set_state(&new_state.to_variant());
                    if let Some(settings_menu_ui) = weak_settings_menu_ui.upgrade() {
                        settings_menu_ui
                            .borrow_mut()
                            .set_touch_screen_controls(new_state);
                    }
                });
            window.add_action(&settings_menu_ui_ref.action_toggle_touch_controls);
        }
    }

    fn set_tooltips_enabled(&mut self, enabled: bool) {
        let mut settings = self.settings_ref.borrow_mut();
        settings.clue_tooltips_enabled = enabled;
        if !settings.save().is_ok() {
            log::error!("Failed to save settings");
        }

        let settings = settings.clone();
        self.global_event_emitter
            .emit(GlobalEvent::SettingsChanged(settings));
    }

    fn set_clue_xray_enabled(&mut self, enabled: bool) {
        let mut settings = self.settings_ref.borrow_mut();
        settings.clue_xray_enabled = enabled;
        if !settings.save().is_ok() {
            log::error!("Failed to save settings");
        }

        let settings = settings.clone();
        self.global_event_emitter
            .emit(GlobalEvent::SettingsChanged(settings));
    }

    fn set_touch_screen_controls(&mut self, enabled: bool) {
        let mut settings = self.settings_ref.borrow_mut();
        settings.touch_screen_controls = enabled;
        if !settings.save().is_ok() {
            log::error!("Failed to save settings");
        }

        let settings = settings.clone();
        self.global_event_emitter
            .emit(GlobalEvent::SettingsChanged(settings));
    }

    pub fn get_menu(&self) -> &Menu {
        &self.settings_menu
    }
}
