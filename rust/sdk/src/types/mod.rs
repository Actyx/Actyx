//! Types that you may want to use in describing the event payload data

mod arcval;
mod binary;
pub mod varint;

pub use arcval::ArcVal;
pub use binary::Binary;

pub mod intern_arc {
    pub use intern_arc::*;
}
