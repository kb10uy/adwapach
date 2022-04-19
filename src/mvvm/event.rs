use std::sync::{Arc, Weak};

/// Subscription held by Observable and Subscriber.
pub struct Subscription<M>(Arc<dyn Fn(M) + Send + Sync + 'static>);

/// Weak reference of subscription function. Internally used.
pub struct WeakSubscription<M>(Weak<dyn Fn(M) + Send + Sync + 'static>);

impl<M: Clone + Send + Sync + 'static> WeakSubscription<M> {
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
    type Message: Clone + Send + Sync + 'static;

    /// Subscribes this object.
    /// Subscriber closure **must NOT** lock the observable object itself in the current thread, or it will end up to recursive lock.
    fn subscribe<S>(&mut self, subscription: S) -> Subscription<Self::Message>
    where
        S: Fn(Self::Message) + Send + Sync + 'static;

    /// Notifies a message to subscribers.
    fn notify(&mut self, message: Self::Message);
}

/// Stores and manages subscriptions.
pub struct EventManager<M>(Vec<WeakSubscription<M>>);

impl<M: Clone + Send + Sync + 'static> EventManager<M> {
    /// Allocates new manager.
    pub fn new() -> EventManager<M> {
        EventManager(vec![])
    }

    /// Subscribes this manager.
    pub fn subscribe<S>(&mut self, subscription: S) -> Subscription<M>
    where
        S: Fn(M) + Send + Sync + 'static,
    {
        let subscription: Arc<dyn Fn(M) + Send + Sync + 'static> = Arc::new(subscription);
        let weak_subscription = WeakSubscription(Arc::downgrade(&subscription));
        self.0.push(weak_subscription);

        Subscription(subscription)
    }

    /// Notifies a message to all subscribers.
    pub fn notify(&mut self, message: M) {
        let mut valid_all = true;
        for subscription in &self.0 {
            valid_all &= subscription.notify(message.clone());
        }

        if !valid_all {
            self.0.retain(|s| s.0.upgrade().is_some());
        }
    }
}
