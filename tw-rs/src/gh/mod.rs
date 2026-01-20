// SPDX-License-Identifier: GPL-3.0-or-later

mod merge_description;
mod serde_types;

pub use merge_description::{get_acks_from_github, compute_merge_description, GetAcksError, MergeDescriptionError};
pub use serde_types::PrInfo;
