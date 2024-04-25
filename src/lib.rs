//! For simplicity, all locks in this crate ignore the unwinding problem. We focus on the algorithms
//! of different locks.

mod lock_guard;
mod mcs_lock;
mod spin_lock;
mod thread_local_msc_lock;
mod ticket_lock;

pub use mcs_lock::MSCLock;
pub use spin_lock::SpinLock;
pub use thread_local_msc_lock::ThreadLocalMscLock;
pub use ticket_lock::TicketLock;
