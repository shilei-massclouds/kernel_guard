//! RAII wrappers to create a critical section with local IRQs or preemption
//! disabled, used to implement spin locks in kernel.
//!
//! The critical section is created after the guard struct is created, and is
//! ended when the guard falls out of scope.
//!
//! The crate user must implement the [`KernelGuardIf`] trait using
//! [`crate_interface::impl_interface`] to provide the low-level implementantion
//! of how to enable/disable kernel preemption, if the feature `preempt` is
//! enabled.
//!
//! Available guards:
//!
//! - [`NoOp`]: Does nothing around the critical section.
//! - [`IrqSave`]: Disables/enables local IRQs around the critical section.
//!   section.
//!
//! # Crate features
//!
//! - `preempt`: Use in the preemptive system. If this feature is enabled, you
//!    need to implement the [`KernelGuardIf`] trait in other crates. Otherwise
//!    the preemption enable/disable operations will be no-ops. This feature is
//!    disabled by default.
//!
//! # Examples
//!
//! ```
//! use kernel_guard::{KernelGuardIf, NoPreempt};
//!
//! struct KernelGuardIfImpl;
//!
//! #[crate_interface::impl_interface]
//! impl KernelGuardIf for KernelGuardIfImpl {
//!     fn enable_preempt() {
//!         // Your implementation here
//!     }
//!     fn disable_preempt() {
//!         // Your implementation here
//!     }
//! }
//!
//! let guard = NoPreempt::new();
//! /* The critical section starts here
//!
//! Do something that requires preemption to be disabled
//!
//! The critical section ends here */
//! drop(guard);
//! ```

#![no_std]
#![feature(asm_const)]

mod arch;

/// A base trait that all guards implement.
pub trait BaseGuard {
    /// The saved state when entering the critical section.
    type State: Clone + Copy;

    /// Something that must be done before entering the critical section.
    fn acquire() -> Self::State;

    /// Something that must be done after leaving the critical section.
    fn release(state: Self::State);
}

/// A no-op guard that does nothing around the critical section.
pub struct NoOp;

cfg_if::cfg_if! {
    // For user-mode std apps, we use the alias of [`NoOp`] for all guards,
    // since we can not disable IRQs or preemption in user-mode.
    if #[cfg(any(target_os = "none", doc))] {
        /// A guard that disables/enables local IRQs around the critical section.
        pub struct IrqSave(usize);
    } else {
        /// Alias of [`NoOp`].
        pub type IrqSave = NoOp;
    }
}

impl BaseGuard for NoOp {
    type State = ();
    fn acquire() -> Self::State {}
    fn release(_state: Self::State) {}
}

impl NoOp {
    /// Creates a new [`NoOp`] guard.
    pub const fn new() -> Self {
        Self
    }
}

impl Drop for NoOp {
    fn drop(&mut self) {}
}

#[cfg(any(target_os = "none", doc))]
mod imp {
    use super::*;

    impl BaseGuard for IrqSave {
        type State = usize;

        #[inline]
        fn acquire() -> Self::State {
            super::arch::local_irq_save_and_disable()
        }

        #[inline]
        fn release(state: Self::State) {
            // restore IRQ states
            super::arch::local_irq_restore(state);
        }
    }

    impl IrqSave {
        /// Creates a new [`IrqSave`] guard.
        pub fn new() -> Self {
            Self(Self::acquire())
        }
    }

    impl Drop for IrqSave {
        fn drop(&mut self) {
            Self::release(self.0)
        }
    }

    impl Default for IrqSave {
        fn default() -> Self {
            Self::new()
        }
    }
}
