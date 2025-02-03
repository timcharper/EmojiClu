use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use glib;
use log::trace;

pub type Callback<T> = Rc<dyn Fn(&T)>;
type SubscriptionId = u64;

pub struct EventEmitter<T: std::fmt::Debug + 'static> {
    channel: Channel<T>,
    pending: Rc<RefCell<VecDeque<T>>>,
}

impl<T: std::fmt::Debug + 'static> Clone for EventEmitter<T> {
    fn clone(&self) -> Self {
        Self {
            channel: self.channel.clone(),
            pending: Rc::clone(&self.pending),
        }
    }
}

pub struct EventObserver<T: std::fmt::Debug + 'static> {
    channel: Channel<T>,
}

impl<T: std::fmt::Debug + 'static> Clone for EventObserver<T> {
    fn clone(&self) -> Self {
        Self {
            channel: self.channel.clone(),
        }
    }
}

pub struct Channel<T: std::fmt::Debug> {
    listeners: Rc<RefCell<HashMap<SubscriptionId, Callback<T>>>>,
    next_id: Rc<RefCell<SubscriptionId>>,
}

impl<T: std::fmt::Debug> Clone for Channel<T> {
    fn clone(&self) -> Self {
        Self {
            listeners: Rc::clone(&self.listeners),
            next_id: Rc::clone(&self.next_id),
        }
    }
}

pub struct Unsubscriber<T: std::fmt::Debug + 'static> {
    channel: Channel<T>,
    id: SubscriptionId,
}

impl<T: std::fmt::Debug + 'static> Unsubscriber<T> {
    pub fn unsubscribe(&self) -> bool {
        self.channel.unsubscribe(self.id)
    }
}

impl<T: std::fmt::Debug + 'static> Channel<T> {
    pub fn new() -> (EventEmitter<T>, EventObserver<T>) {
        let listeners = Rc::new(RefCell::new(HashMap::new()));
        let next_id = Rc::new(RefCell::new(0));
        let pending = Rc::new(RefCell::new(VecDeque::new()));

        let channel = Channel {
            listeners: Rc::clone(&listeners),
            next_id: Rc::clone(&next_id),
        };

        (
            EventEmitter {
                channel: channel.clone(),
                pending: Rc::clone(&pending),
            },
            EventObserver {
                channel: channel.clone(),
            },
        )
    }

    pub fn subscribe<F>(&self, callback: F) -> Unsubscriber<T>
    where
        F: Fn(&T) + 'static,
    {
        let id = {
            let mut next_id = self.next_id.borrow_mut();
            let id = *next_id;
            *next_id += 1;
            id
        };
        self.listeners.borrow_mut().insert(id, Rc::new(callback));
        Unsubscriber {
            channel: self.clone(),
            id,
        }
    }

    pub fn unsubscribe(&self, id: SubscriptionId) -> bool {
        self.listeners.borrow_mut().remove(&id).is_some()
    }

    pub fn emit(&self, data: &T) {
        let listeners = self.listeners.borrow();
        trace!(target: "events", "Emitting event to {} listeners: {:?}", listeners.len(), data);
        for listener in listeners.values() {
            listener(data);
        }
    }

    pub fn clear(&self) {
        self.listeners.borrow_mut().clear();
    }
}

impl<T: std::fmt::Debug + 'static> EventEmitter<T> {
    fn drain_pending_events(&self) {
        let mut pending = self.pending.take();
        while let Some(event) = pending.pop_front() {
            self.channel.emit(&event);
        }
    }

    pub fn emit(&self, data: T) {
        let mut pending = self.pending.borrow_mut();
        if pending.is_empty() {
            // Only schedule a new timeout if this is the first pending event
            let emitter = self.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(0), move || {
                emitter.drain_pending_events();
                // Return false to not repeat the timeout
                glib::ControlFlow::Break
            });
        }
        pending.push_back(data);
    }
}

impl<T: std::fmt::Debug + 'static> EventObserver<T> {
    pub fn subscribe<F>(&self, callback: F) -> Unsubscriber<T>
    where
        F: Fn(&T) + 'static,
    {
        self.channel.subscribe(callback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn test_event_subscription_and_emission() {
        let (emitter, observer) = Channel::<i32>::new();
        let counter = Rc::new(Cell::new(0));
        let counter_clone = counter.clone();

        observer.subscribe(move |_data: &i32| {
            counter_clone.set(counter_clone.get() + 1);
        });

        emitter.emit(42);
        emitter.drain_pending_events();
        assert_eq!(counter.get(), 1);
    }

    #[test]
    fn test_multiple_listeners() {
        let (emitter, observer) = Channel::<i32>::new();
        let sum = Rc::new(Cell::new(0));
        let sum_clone1 = sum.clone();
        let sum_clone2 = sum.clone();

        observer.subscribe(move |data: &i32| {
            sum_clone1.set(sum_clone1.get() + data);
        });

        observer.subscribe(move |data: &i32| {
            sum_clone2.set(sum_clone2.get() + data);
        });

        emitter.emit(5);
        emitter.drain_pending_events();
        assert_eq!(sum.get(), 10); // Each listener adds 5
    }

    #[test]
    fn test_clone_and_share() {
        let (emitter1, observer1) = Channel::<i32>::new();
        let emitter2 = emitter1.clone();
        let observer2 = observer1.clone();

        let counter = Rc::new(Cell::new(0));
        let counter_clone = counter.clone();

        // Subscribe using first observer
        observer1.subscribe(move |_data: &i32| {
            counter_clone.set(counter_clone.get() + 1);
        });

        // Emit using second emitter
        emitter2.emit(42);
        emitter2.drain_pending_events();
        assert_eq!(counter.get(), 1);

        // Subscribe using second observer
        let counter_clone = counter.clone();
        observer2.subscribe(move |_data: &i32| {
            counter_clone.set(counter_clone.get() + 1);
        });

        // Emit using first emitter
        emitter1.emit(42);
        emitter1.drain_pending_events();
        assert_eq!(counter.get(), 3); // Two listeners, each adding 1
    }

    #[test]
    fn test_unsubscribe() {
        let (emitter, observer) = Channel::<i32>::new();
        let counter = Rc::new(Cell::new(0));
        let counter_clone = counter.clone();

        let sub_id = observer.subscribe(move |_data: &i32| {
            counter_clone.set(counter_clone.get() + 1);
        });

        emitter.emit(42);
        emitter.drain_pending_events();
        assert_eq!(counter.get(), 1);

        // Unsubscribe and verify no more updates
        assert!(sub_id.unsubscribe());
        emitter.emit(42);
        emitter.drain_pending_events();
        assert_eq!(counter.get(), 1);

        // Trying to unsubscribe again should return false
        assert!(!sub_id.unsubscribe());
    }
}
