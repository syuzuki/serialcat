pub mod opt;
pub mod util;

/// A "prelude" for crates using the [serialcat](index.html)
pub mod prelude {
    pub use crate::util::GetCharsMixin as _;
}
