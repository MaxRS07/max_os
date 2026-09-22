use core::cell::OnceCell;

use alloc::boxed::Box;
use collections::hashmap::HashMap;
use log::info;
use sync::once::OnceLock;

use crate::arch::riscv::interrupt::route::Trap;

pub static INTERRUPT_NOTIFIER: OnceLock<InterruptNotifier> = OnceLock::new();

pub fn init_notifier() {
    let global_notifier = InterruptNotifier::default();
    INTERRUPT_NOTIFIER.init(|| global_notifier);
}

pub trait InterruptListener: Send + Sync {
    fn call(&self);

    fn clone_box(&self) -> Box<dyn InterruptListener>;
}
impl<F> InterruptListener for F
where
    F: Fn() + Clone + Send + Sync + 'static,
{
    fn call(&self) {
        self(); // Invoke the closure
    }

    fn clone_box(&self) -> Box<dyn InterruptListener> {
        Box::new(self.clone())
    }
}
// `HashMap` stores values by clone, so the boxed trait object needs to be
// cloneable. Delegate to `clone_box` since `dyn` types can't derive `Clone`.
impl Clone for Box<dyn InterruptListener> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
impl PartialEq for dyn InterruptListener {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self, other)
    }
}

/// Notifies subscribed listeners when a relevant interrupt code is detected
#[derive(Default)]
pub struct InterruptNotifier {
    listeners: HashMap<u32, Box<dyn InterruptListener>>,
}

impl InterruptNotifier {
    /// Registers listener to a notifier object
    ///
    /// # Arguments
    /// - `layer`: the trap event to listen for
    /// - `callback`: callable invoked when `layer` fires
    ///
    /// # Return
    /// - `Some(())` if a listener was already registered for `layer` (it is replaced)
    /// - `None` if this is a fresh registration
    pub fn subscribe<F>(&mut self, ipr_id: u32, callback: F) -> Option<()>
    where
        F: Fn() + Clone + Send + Sync + 'static,
    {
        self.listeners
            .insert(ipr_id, Box::new(callback))
            .map(|_| ())
    }

    pub fn unsubscribe(&mut self, _subscriber_id: u32) {
        todo!()
    }
    /// Notifies to all listeners for `trap` event
    pub fn notify(&self, device_id: u32) {
        for listener in self.listeners.get_iter(device_id) {
            listener.call()
        }
    }
}

unsafe impl Send for InterruptNotifier {}
unsafe impl Sync for InterruptNotifier {}
