//! QRCode encoder
//!
//! This crate provides a QR code, Micro QR and rMQR code encoder for binary data.
//!
//! ```
//! use qrqrpar::{QrCode, QrStyle};
//!
//! // Encode some data into bits.
//! let code = QrCode::rmqr("Hello, rmqr!").unwrap();
//!
//! // Define style
//! let style = QrStyle::default();
//!
//! // Render the bits into an image and save it.
//! code.save_png("rmqr.png", &style).unwrap();
//! ```

pub mod bits;
pub mod canvas;
pub mod coding;
pub mod ec;
mod render;
pub mod types;

pub use crate::bits::RmqrStrategy;
pub use crate::types::{Color, EcLevel, QrResult, Version};

#[derive(Debug, Copy, Clone)]
pub enum QrShape {
    Square,
    Round,
}

#[derive(Debug)]
pub struct QrStyle {
    pub color: String,
    pub background_color: String,
    pub shape: QrShape,
    /// output image width. The height is automatically calculated.
    pub width: u32,
    /// Size of the quiet zone around the QR code, measured in terms of a single dot size.
    pub quiet_zone: f64,
}

impl QrStyle {
    pub fn new(
        color: impl Into<String>,
        background_color: impl Into<String>,
        shape: QrShape,
        width: u32,
        quiet_zone: f64,
    ) -> Self {
        Self {
            color: color.into(),
            background_color: background_color.into(),
            shape,
            width,
            quiet_zone,
        }
    }
}

impl Default for QrStyle {
    fn default() -> Self {
        Self {
            color: String::from("#000000"),
            background_color: String::from("#ffffff"),
            shape: QrShape::Square,
            width: 720,
            quiet_zone: 2.0,
        }
    }
}

#[derive(Clone)]
pub struct QrCode {
    content: Vec<Color>,
    version: Version,
    ec_level: EcLevel,
    width: usize,
    height: usize,
}

impl QrCode {
    /// Constructs a new QR code which automatically encodes the given data.
    ///
    /// This method uses the "medium" error correction level and automatically
    /// chooses the smallest QR code.
    ///
    ///     use qrqrpar::QrCode;
    ///
    ///     let code = QrCode::new(b"Some data").unwrap();
    ///
    /// # Errors
    ///
    /// Returns error if the QR code cannot be constructed, e.g. when the data
    /// is too long.
    pub fn new<D: AsRef<[u8]>>(data: D) -> QrResult<Self> {
        Self::with_error_correction_level(data, EcLevel::M)
    }

    /// Constructs a new QR code which automatically encodes the given data at a
    /// specific error correction level.
    ///
    /// This method automatically chooses the smallest QR code.
    ///
    ///     use qrqrpar::{QrCode, EcLevel};
    ///
    ///     let code = QrCode::with_error_correction_level(b"Some data", EcLevel::H).unwrap();
    ///
    /// # Errors
    ///
    /// Returns error if the QR code cannot be constructed, e.g. when the data
    /// is too long.
    pub fn with_error_correction_level<D: AsRef<[u8]>>(
        data: D,
        ec_level: EcLevel,
    ) -> QrResult<Self> {
        let bits = bits::encode_auto(data.as_ref(), ec_level)?;
        Self::with_bits(bits, ec_level)
    }
    /// Constructs a new QR code for the given version and error correction
    /// level.
    ///
    ///     use qrqrpar::{QrCode, Version, EcLevel};
    ///
    ///     let code = QrCode::with_version(b"Some data", Version::Normal(5), EcLevel::M).unwrap();
    ///
    /// This method can also be used to generate Micro QR code.
    ///
    ///     use qrqrpar::{QrCode, Version, EcLevel};
    ///
    ///     let micro_code = QrCode::with_version(b"123", Version::Micro(1), EcLevel::L).unwrap();
    ///
    /// # Errors
    ///
    /// Returns error if the QR code cannot be constructed, e.g. when the data
    /// is too long, or when the version and error correction level are
    /// incompatible.
    pub fn with_version<D: AsRef<[u8]>>(
        data: D,
        version: Version,
        ec_level: EcLevel,
    ) -> QrResult<Self> {
        let mut bits = bits::Bits::new(version);
        bits.push_optimal_data(data.as_ref())?;
        bits.push_terminator(ec_level)?;
        Self::with_bits(bits, ec_level)
    }

    /// Constructs a new QR code with encoded bits.
    ///
    /// Use this method only if there are very special need to manipulate the
    /// raw bits before encoding.
    ///
    /// * Encode data using specific character set with ECI
    /// * Use the FNC1 modes
    /// * Avoid the optimal segmentation algorithm
    ///
    /// # Errors
    ///
    /// Returns error if the QR code cannot be constructed, e.g. when the bits
    /// are too long, or when the version and error correction level are
    /// incompatible.
    pub fn with_bits(bits: bits::Bits, ec_level: EcLevel) -> QrResult<Self> {
        let version = bits.version();
        let data = bits.into_bytes();
        let (encoded_data, ec_data) = ec::construct_codewords(&data, version, ec_level)?;
        let mut canvas = canvas::Canvas::new(version, ec_level);
        canvas.draw_all_functional_patterns();
        canvas.draw_data(&encoded_data, &ec_data);
        let canvas = canvas.apply_best_mask();
        Ok(Self {
            content: canvas.into_colors(),
            version,
            ec_level,
            width: version.width() as usize,
            height: version.height() as usize,
        })
    }

    /// Gets the version of this QR code.
    pub fn version(&self) -> Version {
        self.version
    }

    /// Gets the error correction level of this QR code.
    pub fn error_correction_level(&self) -> EcLevel {
        self.ec_level
    }

    /// Gets the number of modules per side, i.e. the width of this QR code.
    ///
    /// The width here does not contain the quiet zone paddings.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Gets the number of modules per side, i.e. the height of this QR code.
    ///
    /// The height here does not contain the quiet zone paddings.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Converts the QR code to a vector of colors.
    pub fn to_colors(&self) -> Vec<Color> {
        self.content.clone()
    }

    /// Converts the QR code to a vector of colors.
    pub fn into_colors(self) -> Vec<Color> {
        self.content
    }

    /// Converts the QR code into a human-readable string.
    pub fn to_str(&self, dark: char, light: char) -> String {
        let mut s = String::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let color = self.content[y * self.width + x];
                s.push(if color == Color::Dark { dark } else { light });
            }
            s.push('\n');
        }
        s
    }

    /// Constructs a new rMQR code which automatically encodes the given data.
    /// This method uses the "medium" error correction level and automatically
    ///
    ///     use qrqrpar::QrCode;
    ///    
    ///     let code = QrCode::rmqr(b"Some data").unwrap();
    ///
    /// # Errors
    ///
    /// Returns error if the QR code cannot be constructed, e.g. when the data
    /// is too long.
    pub fn rmqr<D: AsRef<[u8]>>(data: D) -> QrResult<Self> {
        Self::rmqr_with_options(data, EcLevel::M, bits::RmqrStrategy::Area)
    }

    /// Constructs a new rMQR code which automatically encodes the given data at a
    /// specific error correction level and rmqr strategy.
    ///
    ///     use qrqrpar::{QrCode, EcLevel,RmqrStrategy};
    ///
    ///     let code = QrCode::rmqr_with_options(b"Some data", EcLevel::H, RmqrStrategy::Area).unwrap();
    ///
    /// # Errors
    ///
    /// Returns error if the QR code cannot be constructed, e.g. when the data
    /// is too long.
    pub fn rmqr_with_options<D: AsRef<[u8]>>(
        data: D,
        ec_level: EcLevel,
        strategy: bits::RmqrStrategy,
    ) -> QrResult<Self> {
        let bits = bits::encode_auto_rmqr(data.as_ref(), ec_level, strategy)?;
        Self::with_bits(bits, ec_level)
    }
}

impl QrCode {
    /// Return `viewbox_width`, `viewbox_height`, `image_width`, `image_height`
    pub fn image_sizes(&self, style: &QrStyle) -> (f64, f64, u32, u32) {
        let quiet = style.quiet_zone;
        let vb_width = self.width as f64 + quiet * 2.0;
        let vb_height = self.height as f64 + quiet * 2.0;
        let width = style.width;
        let height = (width as f64 * vb_height / vb_width).round() as u32;
        (vb_width, vb_height, width, height)
    }

    /// Converts the QR to a simple SVG string.
    pub fn to_simple_svg(&self) -> String {
        let style = QrStyle {
            quiet_zone: 0.0,
            width: self.width as u32,
            ..Default::default()
        };
        self.to_svg(&style)
    }

    /// Converts the QR to a SVG string.
    pub fn to_svg(&self, style: &QrStyle) -> String {
        let mut directed_segments = render::DirectedSegments::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if self.content[y * self.width + x] == Color::Dark {
                    directed_segments.add_or_remove(x as i16, y as i16);
                }
            }
        }
        let path_string = match style.shape {
            QrShape::Square => directed_segments.to_path_square_mut(),
            QrShape::Round => directed_segments.to_path_round_mut(),
        };

        let color = &style.color;
        let background_color = &style.background_color;
        let quiet = style.quiet_zone;
        let (vb_width, vb_height, image_width, image_height) = self.image_sizes(style);
        let path = format!(
            r#"<path fill="{color}" transform="translate({quiet},{quiet})" fill-rule="evenodd" d="{path_string}"/>"#,
        );
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <svg xmlns="http://www.w3.org/2000/svg" version="1.1" width="{image_width}" height="{image_height}" viewBox="0 0 {vb_width} {vb_height}">
            <rect x="0" y="0" width="{vb_width}" height="{vb_height}" fill="{background_color}"/>
            {path}
            </svg>"#,
        )
    }
    /// Saves the QR to a SVG file.
    pub fn save_svg<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        style: &QrStyle,
    ) -> std::io::Result<()> {
        let svg_string = self.to_svg(style);
        std::fs::write(path, svg_string)
    }
}

impl QrCode {
    /// Converts the QR to a tiny-skia pixmap.
    pub fn to_pixmap(
        &self,
        style: &QrStyle,
    ) -> Result<resvg::tiny_skia::Pixmap, Box<dyn std::error::Error>> {
        let (_, _, width, height) = self.image_sizes(style);
        let svg_string = self.to_svg(style);
        let opt = resvg::usvg::Options::default();
        let tree = &resvg::usvg::TreeParsing::from_str(&svg_string, &opt)?;
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(width, height).ok_or("failed to create pixmap")?;
        resvg::Tree::from_usvg(tree)
            .render(resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());
        Ok(pixmap)
    }

    /// Saves the QR to a PNG file.
    pub fn save_png<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        style: &QrStyle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pixmap = self.to_pixmap(style)?;
        pixmap.save_png(path)?;
        Ok(())
    }

    /// Encodes QR into a PNG data.
    pub fn to_png(&self, style: &QrStyle) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let pixmap = self.to_pixmap(style)?;
        Ok(pixmap.encode_png()?)
    }
}

#[cfg(test)]
mod image_test {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn test_save_png() {
        let test_dir = TempDir::new("__test__").unwrap();
        let path = test_dir.path().join("rmqr.png");
        let code = QrCode::new(b"Hello, rmqr!").unwrap();
        let style = QrStyle::default();
        code.save_png(path, &style).unwrap();
    }
    #[test]
    fn test_save_svg() {
        let test_dir = TempDir::new("__test__").unwrap();
        let path = test_dir.path().join("rmqr.svg");
        let code = QrCode::new(b"Hello, rmqr!").unwrap();
        let style = QrStyle::default();
        code.save_svg(path, &style).unwrap();
    }
    #[test]
    fn test_save_svg2() {
        let test_dir = TempDir::new("__test__").unwrap();
        let path = test_dir.path().join("micro_qr_m3_l.svg");
        let code =
            QrCode::with_version("11111111111111111111", Version::Micro(3), EcLevel::L).unwrap();
        let style = QrStyle::default();
        code.save_svg(path, &style).unwrap();
    }
}
