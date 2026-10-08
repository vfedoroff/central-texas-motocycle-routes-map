pub mod topology;
pub mod types;

pub use topology::{Spur, detect_spurs};
pub use types::*;

#[cfg(feature = "native")]
pub mod load;
#[cfg(feature = "native")]
pub mod validation;

#[cfg(feature = "native")]
pub use load::load_catalog;
#[cfg(feature = "native")]
pub use validation::{validate_catalog, validate_catalog_with_options};
