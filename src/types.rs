use core::cmp::{Ordering, PartialOrd};
use core::fmt::{Display, Error, Formatter};
use core::ops::Not;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum QrError {
    /// The data is too long to encode into a QR code for the given version.
    DataTooLong,

    /// The provided version / error correction level combination is invalid.
    InvalidVersion,

    /// Some characters in the data cannot be supported by the provided QR code
    /// version.
    UnsupportedCharacterSet,

    /// The provided ECI designator is invalid. A valid designator should be
    /// between 0 and 999999.
    InvalidEciDesignator,

    /// A character not belonging to the character set is found.
    InvalidCharacter,
}

impl Display for QrError {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        let msg = match *self {
            QrError::DataTooLong => "data too long",
            QrError::InvalidVersion => "invalid version",
            QrError::UnsupportedCharacterSet => "unsupported character set",
            QrError::InvalidEciDesignator => "invalid ECI designator",
            QrError::InvalidCharacter => "invalid character",
        };
        fmt.write_str(msg)
    }
}

impl ::std::error::Error for QrError {}

/// `QrResult` is a convenient alias for a QR code generation result.
pub type QrResult<T> = Result<T, QrError>;

/// The color of a module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Color {
    /// The module is light colored.
    Light,
    /// The module is dark colored.
    Dark,
}

impl Color {
    /// Selects a value according to color of the module. Equivalent to
    /// `if self != Color::Light { dark } else { light }`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use qrqrpar::types::Color;
    /// assert_eq!(Color::Light.select(1, 0), 0);
    /// assert_eq!(Color::Dark.select("black", "white"), "black");
    /// ```
    pub fn select<T>(self, dark: T, light: T) -> T {
        match self {
            Color::Light => light,
            Color::Dark => dark,
        }
    }
}

impl Not for Color {
    type Output = Self;
    fn not(self) -> Self {
        match self {
            Color::Light => Color::Dark,
            Color::Dark => Color::Light,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EcLevel {
    /// Low error correction. Allows up to 7% of wrong blocks.
    L = 0,

    /// Medium error correction (default). Allows up to 15% of wrong blocks.
    M = 1,

    /// "Quartile" error correction. Allows up to 25% of wrong blocks.
    Q = 2,

    /// High error correction. Allows up to 30% of wrong blocks.
    H = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Version {
    /// A normal QR code version. The parameter should be between 1 and 40.
    Normal(u8),

    /// A Micro QR code version. The parameter should be between 1 and 4.
    Micro(u8),

    /// A Rmqr code version. The first parameter should be 7, 9, 11, 13, 15, or 17.
    /// The second parameter should be 27, 43, 59, 77, 99, or 139. 27 can only be used with 11, or 13.
    Rmqr(u8, u8),
}

impl Version {
    /// Get the number of "modules" on each size of the QR code, i.e. the width
    pub fn width(self) -> i16 {
        match self {
            Version::Normal(v) => v as i16 * 4 + 17,
            Version::Micro(v) => v as i16 * 2 + 9,
            Version::Rmqr(_, a) => a as i16,
        }
    }

    /// Get the height
    pub fn height(self) -> i16 {
        match self {
            Version::Rmqr(a, _) => a as i16,
            _ => self.width(),
        }
    }

    /// Get the area
    pub fn area(self) -> i16 {
        self.width() * self.height()
    }

    /// Obtains an object from a hard-coded table.
    ///
    /// The table must be a 76×4 array. The outer array represents the content
    /// for each version. The first 40 entry corresponds to QR code versions 1
    /// to 40, and the next 4 corresponds to Micro QR code version 1 to 4 and
    /// the next 32 corresponds to rMQR code. The inner array represents the
    /// content in each error correction level, in the order [L, M, Q, H].
    ///
    /// # Errors
    ///
    /// If the entry compares equal to the default value of `T`, this method
    /// returns `Err(QrError::InvalidVersion)`.
    pub fn fetch<T>(self, ec_level: EcLevel, table: &[[T; 4]]) -> QrResult<T>
    where
        T: PartialEq + Default + Copy,
    {
        match self {
            Version::Normal(v @ 1..=40) => {
                return Ok(table[(v - 1) as usize][ec_level as usize]);
            }
            Version::Micro(v @ 1..=4) => {
                let obj = table[(v + 39) as usize][ec_level as usize];
                if obj != T::default() {
                    return Ok(obj);
                }
            }
            Version::Rmqr(_, _) => {
                let index = self.rmqr_index()?;
                let obj = table[index + 44][ec_level as usize];
                if obj != T::default() {
                    return Ok(obj);
                }
            }

            _ => {}
        }
        Err(QrError::InvalidVersion)
    }

    /// The number of bits needed to encode the mode indicator.
    pub fn mode_bits_count(self) -> usize {
        match self {
            Version::Normal(_) => 4,
            Version::Micro(a) => (a - 1).into(),
            Version::Rmqr(_, _) => 3,
        }
    }

    /// Checks whether is version refers to a Micro QR code.
    pub fn is_micro(self) -> bool {
        matches!(self, Version::Micro(_))
    }

    /// Checks whether is version refers to a rMQR code.
    pub fn is_rmqr(self) -> bool {
        self.rmqr_index().is_ok()
    }

    /// Get the index of the version of the rMQR code.
    pub fn rmqr_index(self) -> QrResult<usize> {
        match self {
            Version::Rmqr(7, 43) => Ok(0),
            Version::Rmqr(7, 59) => Ok(1),
            Version::Rmqr(7, 77) => Ok(2),
            Version::Rmqr(7, 99) => Ok(3),
            Version::Rmqr(7, 139) => Ok(4),
            Version::Rmqr(9, 43) => Ok(5),
            Version::Rmqr(9, 59) => Ok(6),
            Version::Rmqr(9, 77) => Ok(7),
            Version::Rmqr(9, 99) => Ok(8),
            Version::Rmqr(9, 139) => Ok(9),
            Version::Rmqr(11, 27) => Ok(10),
            Version::Rmqr(11, 43) => Ok(11),
            Version::Rmqr(11, 59) => Ok(12),
            Version::Rmqr(11, 77) => Ok(13),
            Version::Rmqr(11, 99) => Ok(14),
            Version::Rmqr(11, 139) => Ok(15),
            Version::Rmqr(13, 27) => Ok(16),
            Version::Rmqr(13, 43) => Ok(17),
            Version::Rmqr(13, 59) => Ok(18),
            Version::Rmqr(13, 77) => Ok(19),
            Version::Rmqr(13, 99) => Ok(20),
            Version::Rmqr(13, 139) => Ok(21),
            Version::Rmqr(15, 43) => Ok(22),
            Version::Rmqr(15, 59) => Ok(23),
            Version::Rmqr(15, 77) => Ok(24),
            Version::Rmqr(15, 99) => Ok(25),
            Version::Rmqr(15, 139) => Ok(26),
            Version::Rmqr(17, 43) => Ok(27),
            Version::Rmqr(17, 59) => Ok(28),
            Version::Rmqr(17, 77) => Ok(29),
            Version::Rmqr(17, 99) => Ok(30),
            Version::Rmqr(17, 139) => Ok(31),
            _ => Err(QrError::InvalidVersion),
        }
    }

    /// Get the index in ascending order of width.
    pub fn rmqr_width_index(self) -> QrResult<usize> {
        match self {
            Version::Rmqr(_, 27) => Ok(0),
            Version::Rmqr(_, 43) => Ok(1),
            Version::Rmqr(_, 59) => Ok(2),
            Version::Rmqr(_, 77) => Ok(3),
            Version::Rmqr(_, 99) => Ok(4),
            Version::Rmqr(_, 139) => Ok(5),
            _ => Err(QrError::InvalidVersion),
        }
    }

    /// 27, 43, 59, 77, 99, 139
    pub fn rmqr_all_width() -> [u8; 6] {
        [27, 43, 59, 77, 99, 139]
    }

    /// 7, 9, 11, 13, 15, 17
    pub fn rmqr_all_height() -> [u8; 6] {
        [7, 9, 11, 13, 15, 17]
    }

    pub fn rmqr_all() -> [Version; 32] {
        [
            Version::Rmqr(7, 43),
            Version::Rmqr(7, 59),
            Version::Rmqr(7, 77),
            Version::Rmqr(7, 99),
            Version::Rmqr(7, 139),
            Version::Rmqr(9, 43),
            Version::Rmqr(9, 59),
            Version::Rmqr(9, 77),
            Version::Rmqr(9, 99),
            Version::Rmqr(9, 139),
            Version::Rmqr(11, 27),
            Version::Rmqr(11, 43),
            Version::Rmqr(11, 59),
            Version::Rmqr(11, 77),
            Version::Rmqr(11, 99),
            Version::Rmqr(11, 139),
            Version::Rmqr(13, 27),
            Version::Rmqr(13, 43),
            Version::Rmqr(13, 59),
            Version::Rmqr(13, 77),
            Version::Rmqr(13, 99),
            Version::Rmqr(13, 139),
            Version::Rmqr(15, 43),
            Version::Rmqr(15, 59),
            Version::Rmqr(15, 77),
            Version::Rmqr(15, 99),
            Version::Rmqr(15, 139),
            Version::Rmqr(17, 43),
            Version::Rmqr(17, 59),
            Version::Rmqr(17, 77),
            Version::Rmqr(17, 99),
            Version::Rmqr(17, 139),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Numeric,
    Alphanumeric,
    Byte,
    Kanji,
}

impl Mode {
    /// Computes the number of bits needed to encode the data length.
    ///
    ///     use qrqrpar::types::{Version, Mode};
    ///
    ///     assert_eq!(Mode::Numeric.length_bits_count(Version::Normal(1)), 10);
    ///
    /// This method will return `Err(QrError::UnsupportedCharacterSet)` if the
    /// mode is not supported in the given version.
    pub fn length_bits_count(self, version: Version) -> usize {
        match version {
            Version::Micro(a) => {
                let a = a.into();
                match self {
                    Mode::Numeric => 2 + a,
                    Mode::Alphanumeric | Mode::Byte => 1 + a,
                    Mode::Kanji => a,
                }
            }

            Version::Normal(1..=9) => match self {
                Mode::Numeric => 10,
                Mode::Alphanumeric => 9,
                Mode::Byte | Mode::Kanji => 8,
            },
            Version::Normal(10..=26) => match self {
                Mode::Numeric => 12,
                Mode::Alphanumeric => 11,
                Mode::Byte => 16,
                Mode::Kanji => 10,
            },
            Version::Normal(_) => match self {
                Mode::Numeric => 14,
                Mode::Alphanumeric => 13,
                Mode::Byte => 16,
                Mode::Kanji => 12,
            },
            Version::Rmqr(_, _) => {
                let index = version.rmqr_index().unwrap_or(31);
                match self {
                    Mode::Numeric => RMQR_LENGTH_BITS_COUNT[index][0],
                    Mode::Alphanumeric => RMQR_LENGTH_BITS_COUNT[index][1],
                    Mode::Byte => RMQR_LENGTH_BITS_COUNT[index][2],
                    Mode::Kanji => RMQR_LENGTH_BITS_COUNT[index][3],
                }
            }
        }
    }

    /// Computes the number of bits needed to some data of a given raw length.
    ///
    ///     use qrqrpar::types::Mode;
    ///
    ///     assert_eq!(Mode::Numeric.data_bits_count(7), 24);
    ///
    /// Note that in Kanji mode, the `raw_data_len` is the number of Kanjis,
    /// i.e. half the total size of bytes.
    pub fn data_bits_count(self, raw_data_len: usize) -> usize {
        match self {
            Mode::Numeric => (raw_data_len * 10 + 2) / 3,
            Mode::Alphanumeric => (raw_data_len * 11 + 1) / 2,
            Mode::Byte => raw_data_len * 8,
            Mode::Kanji => raw_data_len * 13,
        }
    }

    /// Find the lowest common mode which both modes are compatible with.
    ///
    ///     use qrqrpar::types::Mode;
    ///
    ///     let a = Mode::Numeric;
    ///     let b = Mode::Kanji;
    ///     let c = a.max(b);
    ///     assert!(a <= c);
    ///     assert!(b <= c);
    ///
    pub fn max(self, other: Self) -> Self {
        match self.partial_cmp(&other) {
            Some(Ordering::Less) | Some(Ordering::Equal) => other,
            Some(Ordering::Greater) => self,
            None => Mode::Byte,
        }
    }
}

impl PartialOrd for Mode {
    /// Defines a partial ordering between modes. If `a <= b`, then `b` contains
    /// a superset of all characters supported by `a`.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (*self, *other) {
            (Mode::Numeric, Mode::Alphanumeric)
            | (Mode::Numeric, Mode::Byte)
            | (Mode::Alphanumeric, Mode::Byte)
            | (Mode::Kanji, Mode::Byte) => Some(Ordering::Less),
            (Mode::Alphanumeric, Mode::Numeric)
            | (Mode::Byte, Mode::Numeric)
            | (Mode::Byte, Mode::Alphanumeric)
            | (Mode::Byte, Mode::Kanji) => Some(Ordering::Greater),
            (a, b) if a == b => Some(Ordering::Equal),
            _ => None,
        }
    }
}

/// The number of bits needed to encode the length of the data.
///
/// \[ Numeric, Alphanumeric, Byte, Kanji \]
static RMQR_LENGTH_BITS_COUNT: [[usize; 4]; 32] = [
    [4, 3, 3, 2], //R7x43
    [5, 5, 4, 3], //R7x59
    [6, 5, 5, 4], //R7x77
    [7, 6, 5, 5], //R7x99
    [7, 6, 6, 5], //R7x139
    [5, 5, 4, 3], //R9x43
    [6, 5, 5, 4], //R9x59
    [7, 6, 5, 5], //R9x77
    [7, 6, 6, 5], //R9x99
    [8, 7, 6, 6], //R9x139
    [4, 4, 3, 2], //R11x27
    [6, 5, 5, 4], //R11x43
    [7, 6, 5, 5], //R11x59
    [7, 6, 6, 5], //R11x77
    [8, 7, 6, 6], //R11x99
    [8, 7, 7, 6], //R11x139
    [5, 5, 4, 3], //R13x27
    [6, 6, 5, 5], //R13x43
    [7, 6, 6, 5], //R13x59
    [7, 7, 6, 6], //R13x77
    [8, 7, 7, 6], //R13x99
    [8, 8, 7, 7], //R13x139
    [7, 6, 6, 5], //R15x43
    [7, 7, 6, 5], //R15x59
    [8, 7, 7, 6], //R15x77
    [8, 7, 7, 6], //R15x99
    [9, 8, 7, 7], //R15x139
    [7, 6, 6, 5], //R17x43
    [8, 7, 6, 6], //R17x59
    [8, 7, 7, 6], //R17x77
    [8, 8, 7, 6], //R17x99
    [9, 8, 8, 7], //R17x139
];
