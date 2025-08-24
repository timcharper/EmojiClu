use crate::destroyable::Destroyable;
use crate::events::{EventObserver, Unsubscriber};
use crate::game::settings::Settings;
use crate::model::GameEngineEvent;
use std::cell::RefCell;
use std::rc::Rc;

pub struct SettingsProjection {
    settings: Settings,
    subscription: Option<Unsubscriber<GameEngineEvent>>,
}

impl Destroyable for SettingsProjection {
    fn destroy(&mut self) {
        if let Some(subscription) = self.subscription.take() {
            subscription.unsubscribe();
        }
    }
}

impl SettingsProjection {
    pub fn new(initial: &Settings, observer: &EventObserver<GameEngineEvent>) -> Rc<RefCell<Self>> {
        let instance = Rc::new(RefCell::new(Self {
            settings: initial.clone(),
            subscription: None,
        }));

        SettingsProjection::attach(&instance, observer);
        instance
    }

    pub fn current_settings(&self) -> Settings {
        self.settings.clone()
    }

    fn attach(instance: &Rc<RefCell<Self>>, observer: &EventObserver<GameEngineEvent>) {
        let subscription = observer.subscribe({
            let settings_rc = instance.clone();
            move |event| {
                if let GameEngineEvent::SettingsChanged(new_settings) = event {
                    settings_rc.borrow_mut().settings = new_settings.clone();
                }
            }
        });
        instance.borrow_mut().subscription = Some(subscription);
    }
}
