//! Pure Rust domain models, business logic, and shared utilities for Swift Bill.
//!
//! This crate has zero dependency on Tauri or any GUI/FFI layer. It is the
//! single source of truth for:
//!
//! * Domain data structures ([`models`])
//! * Report processing algorithms ([`reports`])
//! * Receiving-number allocation with lock support ([`numbering`])
//! * Thai date and register-number helpers ([`date`], [`register`])

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub mod continuity;
pub mod date;
pub mod models;
pub mod numbering;
pub mod register;
pub mod reports;

pub use models::*;
