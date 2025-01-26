use std::cell::RefCell;
use std::rc::Rc;

use log::trace;

pub type Callback<T> = Rc<dyn Fn(&T)>;

pub struct EventEmitter<T: std::fmt::Debug> {
    channel: Channel<T>,
}

impl<T: std::fmt::Debug> Clone for EventEmitter<T> {
    fn clone(&self) -> Self {
        Self {
            channel: self.channel.clone(),
        }
    }
}

pub struct EventObserver<T: std::fmt::Debug> {
    channel: Channel<T>,
}
impl<T: std::fmt::Debug> Clone for EventObserver<T> {
    fn clone(&self) -> Self {
        Self {
            channel: self.channel.clone(),
        }
    }
}

pub struct Channel<T: std::fmt::Debug> {
    listeners: Rc<RefCell<Vec<Callback<T>>>>,
}

impl<T: std::fmt::Debug> Clone for Channel<T> {
    fn clone(&self) -> Self {
        Self {
            listeners: Rc::clone(&self.listeners),
        }
    }
}

impl<T: std::fmt::Debug> Channel<T> {
    pub fn new() -> (EventEmitter<T>, EventObserver<T>) {
        let listeners = Rc::new(RefCell::new(Vec::new()));
        let channel = Channel {
            listeners: Rc::clone(&listeners),
        };
        (
            EventEmitter {
                channel: channel.clone(),
            },
            EventObserver {
                channel: channel.clone(),
            },
        )
    }

    pub fn subscribe<F>(&self, callback: F)
    where
        F: Fn(&T) + 'static,
    {
        self.listeners.borrow_mut().push(Rc::new(callback));
    }

    pub fn emit(&self, data: &T) {
        trace!(target: "events", "Emitting event: {:?}", data);
        let listeners = self.listeners.borrow().clone();
        for listener in listeners.iter() {
            trace!(target: "events", "Receiving event: {:?}", data);
            listener(data);
        }
    }

    pub fn clear(&self) {
        self.listeners.borrow_mut().clear();
    }
}

impl<T: std::fmt::Debug> EventEmitter<T> {
    pub fn emit(&self, data: &T) {
        self.channel.emit(data);
    }
}

impl<T: std::fmt::Debug> EventObserver<T> {
    pub fn subscribe<F>(&self, callback: F)
    where
        F: Fn(&T) + 'static,
    {
        self.channel.subscribe(callback);
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

        emitter.emit(&42);
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

        emitter.emit(&5);
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
        emitter2.emit(&42);
        assert_eq!(counter.get(), 1);

        // Subscribe using second observer
        let counter_clone = counter.clone();
        observer2.subscribe(move |_data: &i32| {
            counter_clone.set(counter_clone.get() + 1);
        });

        // Emit using first emitter
        emitter1.emit(&42);
        assert_eq!(counter.get(), 3); // Two listeners, each adding 1
    }
}
