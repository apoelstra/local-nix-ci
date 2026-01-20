// SPDX-License-Identifier: GPL-3.0-or-later

mod merge_description;
mod serde_types;

pub use merge_description::{compute_merge_description, MergeDescriptionError};
pub use serde_types::PrInfo;
