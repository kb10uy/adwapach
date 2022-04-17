pub mod application;

use std::sync::{Arc, Mutex, Weak};

/// Subscription held by Observable and Subscriber.
pub struct Subscription<M>(Arc<dyn Fn(M) + Send + 'static>);

/// Weak reference of subscription function. Internally used.
pub struct WeakSubscription<M>(Weak<dyn Fn(M) + Send + 'static>);

impl<M: Clone + Send + 'static> WeakSubscription<M> {
    /// Notifies a message.
    pub fn notify(&self, message: M) -> bool {
        if let Some(subscription) = self.0.upgrade() {
            subscription(message);
            true
        } else {
            false
        }
    }
}

pub trait Observable {
    /// Message type which will be sent to subscrbers.
    type Message: Clone + Send + 'static;

    /// Subscribes this object.
    fn subscribe<S>(&self, subscription: S) -> Subscription<Self::Message>
    where
        S: Fn(Self::Message) + Send + 'static;

    /// Notifies a message to subscribers.
    fn notify(&self, message: Self::Message);
}

/// Stores and manages subscriptions.
pub struct EventManager<M>(Arc<Mutex<Vec<WeakSubscription<M>>>>);

impl<M: Clone + Send + 'static> EventManager<M> {
    /// Allocates new manager.
    pub fn new() -> EventManager<M> {
        EventManager(Arc::new(Mutex::new(vec![])))
    }

    /// Subscribes this manager.
    pub fn subscribe<S>(&self, subscription: S) -> Subscription<M>
    where
        S: Fn(M) + Send + 'static,
    {
        let subscription: Arc<dyn Fn(M) + Send + 'static> = Arc::new(subscription);
        let weak_subscription = WeakSubscription(Arc::downgrade(&subscription));

        let mut locked = self.0.lock().expect("EventManager was poisoned");
        locked.push(weak_subscription);

        Subscription(subscription)
    }

    /// Notifies a message to all subscribers.
    pub fn notify(&self, message: M) {
        let mut locked = self.0.lock().expect("EventManager was poisoned");

        let mut some_invalid = false;
        for subscription in locked.iter() {
            match subscription.0.upgrade() {
                Some(s) => {
                    s(message.clone());
                }
                None => {
                    some_invalid = true;
                }
            }
        }

        if some_invalid {
            locked.retain(|s| s.0.upgrade().is_some());
        }
    }
}
