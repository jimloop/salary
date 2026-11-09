//! Pure, deterministic payroll rules. No filesystem, UI, or network access.

pub mod bonus;
pub mod contributions;
pub mod formula;
pub mod inference;
pub mod monthly;
pub mod rules;

pub use rust_decimal::Decimal;
