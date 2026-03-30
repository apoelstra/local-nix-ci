// SPDX-License-Identifier: GPL-3.0-or-later

use core::fmt;

use lcilib::db::{AckStatus, CiStatus, MergeStatus, ReviewStatus};
use lcilib::git::CommitId;
use super::ColorFormat;

/// Object which can be colored on the terminal in a consistent way.
pub trait Colorable: fmt::Display {
    fn with_color(&self) -> ColorFormat<&Self>;
}

impl Colorable for bool {
    fn with_color(&self) -> ColorFormat<&Self> {
        if *self {
            ColorFormat::dull_green(self)
        } else {
            ColorFormat::dull_red(self)
        }
    }
}

impl Colorable for CommitId {
    fn with_color(&self) -> ColorFormat<&Self> {
        ColorFormat::white(self)
    }
}


impl Colorable for AckStatus {
    fn with_color(&self) -> ColorFormat<&Self> {
        match *self {
            Self::Pending => ColorFormat::light_purple(self),
            Self::Failed => ColorFormat::dull_red(self),
            Self::Posted => ColorFormat::dull_green(self),
            Self::External => ColorFormat::dull_green(self),
        }
    }
}

impl Colorable for CiStatus {
    fn with_color(&self) -> ColorFormat<&Self> {
        match *self {
            Self::Unstarted => ColorFormat::light_purple(self),
            Self::Skipped => ColorFormat::pale_yellow(self),
            Self::Failed => ColorFormat::dull_red(self),
            Self::Passed => ColorFormat::dull_green(self),
        }
    }
}

impl Colorable for MergeStatus {
    fn with_color(&self) -> ColorFormat<&Self> {
        match *self {
            Self::Pending => ColorFormat::light_purple(self),
            Self::Cancelled => ColorFormat::pale_yellow(self),
            Self::Conflicted => ColorFormat::dull_red(self),
            Self::Pushed => ColorFormat::dull_green(self),
        }
    }
}

impl Colorable for ReviewStatus {
    fn with_color(&self) -> ColorFormat<&Self> {
        match *self {
            Self::Unreviewed => ColorFormat::light_purple(self),
            Self::Rejected => ColorFormat::dull_red(self),
            Self::Approved => ColorFormat::dull_green(self),
        }
    }
}