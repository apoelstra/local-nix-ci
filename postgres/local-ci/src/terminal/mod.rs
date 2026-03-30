// SPDX-License-Identifier: GPL-3.0-or-later

//! Terminal Support
//!
//! Utilities to output RGB to a terminal
//!

// (Taken from trade-tracker, then made minor tweaks and clippy cleanups.)

mod colorable;

use core::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

pub use colorable::Colorable;

static ENABLE_COLOR: AtomicBool = AtomicBool::new(true);

/// Disable terminal colors
pub fn disable_terminal_color() {
    ENABLE_COLOR.store(true, Ordering::SeqCst);
}

// lol at how many casting lints we fail on this
#[expect(clippy::cast_possible_truncation)] // clearly ok
#[expect(clippy::cast_precision_loss)] // clearly ok
#[expect(clippy::cast_sign_loss)] // clearly ok
fn hsv_to_rgb(hue: usize, sat: f64, light: f64) -> (usize, usize, usize) {
    assert!(hue <= 360, "Hue must lie between 0 and 360 inclusive.");
    assert!(sat >= 0.0, "Saturation must be >= 0.0");
    assert!(sat <= 1.0, "Saturation must be <= 1.0");
    assert!(light >= 0.0, "Lightness must be >= 0.0");
    assert!(light <= 1.0, "Lightness must be <= 1.0");

    let chroma = (1.0 - (2.0 * light - 1.0).abs()) * sat;
    let x = chroma * (1.0 - ((hue % 120) as f64 / 60.0 - 1.0).abs());
    let m = light - chroma / 2.0;
    let ret_f64 = if hue <= 60 {
        (chroma + m, x + m, 0.0 + m)
    } else if hue <= 120 {
        (x + m, chroma + m, 0.0 + m)
    } else if hue <= 180 {
        (0.0 + m, chroma + m, x + m)
    } else if hue <= 240 {
        (0.0 + m, x + m, chroma + m)
    } else if hue <= 300 {
        (x + m, 0.0 + m, chroma + m)
    } else {
        (chroma + m, 0.0 + m, x + m)
    };
    (
        (255.0 * ret_f64.0) as usize,
        (255.0 * ret_f64.1) as usize,
        (255.0 * ret_f64.2) as usize,
    )
}

/// Structure wrapping a "color formatter"
///
/// When run through Display with the alternate flag set, this will output the
/// enclosed data with terminal codes to display the RGB.
pub struct ColorFormat<D: fmt::Display> {
    data: D,
    red: usize,
    green: usize,
    blue: usize,
}

impl<D: fmt::Display> fmt::Display for ColorFormat<D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let color_on = ENABLE_COLOR.load(Ordering::Relaxed);
        if color_on {
            write!(f, "\x1b[38;2;{};{};{}m", self.red, self.green, self.blue)?;
        }
        fmt::Display::fmt(&self.data, f)?;
        if color_on {
            write!(f, "\x1b[0m")?;
        }
        Ok(())
    }
}

impl<D: fmt::Display> ColorFormat<D> {
    /// Construct a new formatter with a given color
    pub fn new(data: D, red: usize, green: usize, blue: usize) -> Self {
        Self {
            data,
            red,
            green,
            blue,
        }
    }

    /// Construct a formatter which takes a value, a "red endpoint" and a "green endpoint"
    /// and interpolates a color between them
    #[expect(clippy::cast_possible_truncation)] // clearly ok
    #[expect(clippy::cast_sign_loss)] // clearly ok
    pub fn redgreen(data: D, val: f64, red: f64, green: f64) -> Self {
        let mut percent_red = if green >= red {
            (val - red) / (green - red)
        } else {
            1.0 - (val - green) / (red - green)
        };
        percent_red = percent_red.clamp(0.0, 1.0);
        let rgb = hsv_to_rgb((percent_red * 120.0) as usize, 1.0, 0.6);
        Self::new(data, rgb.0, rgb.1, rgb.2)
    }

    /// Construct a new white formatter
    pub fn white(data: D) -> Self {
        Self::new(data, 250, 250, 250)
    }

    /// Construct a new light-blue formatter
    #[expect(dead_code)]
    pub fn pale_aqua(data: D) -> Self {
        Self::new(data, 110, 250, 250)
    }

    /// Construct a new light-blue formatter
    #[expect(dead_code)]
    pub fn pale_blue(data: D) -> Self {
        Self::new(data, 180, 180, 250)
    }

    /// Construct a new light-blue formatter
    #[expect(dead_code)]
    pub fn light_blue(data: D) -> Self {
        Self::new(data, 64, 192, 255)
    }

    /// Construct a new light-green formatter
    #[expect(dead_code)]
    pub fn light_green(data: D) -> Self {
        Self::new(data, 80, 250, 80)
    }

    /// Construct a new light-purple formatter
    pub fn light_purple(data: D) -> Self {
        Self::new(data, 250, 110, 250)
    }

    /// Construct a new pale yellow formatter
    pub fn pale_yellow(data: D) -> Self {
        Self::new(data, 250, 250, 180)
    }

    /// Construct a new dull-green formatter
    pub fn dull_green(data: D) -> Self {
        Self::new(data, 130, 220, 130)
    }

    /// Construct a new dull-redformatter
    pub fn dull_red(data: D) -> Self {
        Self::new(data, 220, 130, 130)
    }

    #[expect(dead_code)]
    pub fn grey(data: D) -> Self {
        Self::new(data, 160, 160, 160)
    }
}
