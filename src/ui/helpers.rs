use std::{cell::Cell, rc::Rc, time::Instant};

use gtk4::{prelude::*, GestureClick};

use crate::{
    events::EventEmitter,
    model::{Clickable, InputEvent, LONG_PRESS_DURATION},
};

pub fn register_left_click_handler<
    F: Fn(&GestureClick, i32, f64, f64) -> Option<Clickable> + 'static,
>(
    event_emitter: Rc<EventEmitter<InputEvent>>,
    gesture_click: &GestureClick,
    handler: F,
) {
    let handler = Rc::new(handler);
    let timer_source_id: Rc<Cell<Option<glib::SourceId>>> = Rc::new(Cell::new(None));
    let press_start_time: Rc<Cell<Option<Instant>>> = Rc::new(Cell::new(None));
    gesture_click.connect_pressed({
        let press_start_for_press = Rc::clone(&press_start_time);
        let event_emitter = Rc::downgrade(&event_emitter);
        let timer_source_id = Rc::clone(&timer_source_id);
        let handler = Rc::clone(&handler);
        move |gesture, i, x, y| {
            if let Some(event_emitter) = event_emitter.upgrade() {
                let clickable = handler(gesture, i, x, y);
                press_start_for_press.set(Some(Instant::now()));

                // Emit LeftClick immediately on press
                if let Some(clickable) = clickable {
                    event_emitter.emit(InputEvent::LeftClick(clickable));

                    // Set up timer for long press
                    let press_start_for_timer = Rc::clone(&press_start_for_press);
                    let event_emitter_for_timer = event_emitter.clone();
                    let timer_duration = LONG_PRESS_DURATION + std::time::Duration::from_millis(50);
                    let source_id = glib::timeout_add_local_once(timer_duration, {
                        let timer_source_id = Rc::clone(&timer_source_id);
                        move || {
                            timer_source_id.set(None);
                            if let Some(start) = press_start_for_timer.take() {
                                event_emitter_for_timer
                                    .emit(InputEvent::TouchEvent(clickable, start.elapsed()));
                                press_start_for_timer.set(None);
                            }
                        }
                    });
                    gesture.set_state(gtk4::EventSequenceState::Claimed);
                    timer_source_id.set(Some(source_id));
                }
            }
        }
    });

    // Handle release with duration check and emit LeftClickUp
    gesture_click.connect_released({
        let press_start_for_release = Rc::clone(&press_start_time);
        let event_emitter = Rc::downgrade(&event_emitter);
        let timer_source_id = Rc::clone(&timer_source_id);
        let handler = Rc::clone(&handler);
        move |gesture, i, x, y| {
            if let Some(event_emitter) = event_emitter.upgrade() {
                if let Some(source_id) = timer_source_id.take() {
                    source_id.remove();
                }
                if let Some(start) = press_start_for_release.take() {
                    let duration = start.elapsed();
                    let clickable = handler(gesture, i, x, y);
                    if let Some(clickable) = clickable {
                        event_emitter.emit(InputEvent::TouchEvent(clickable, duration));
                        gesture.set_state(gtk4::EventSequenceState::Claimed);
                    }
                }
            }
        }
    });
}
