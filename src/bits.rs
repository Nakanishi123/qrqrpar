//! The `bits` module encodes binary data into raw bits used in a QR code.
use core::cmp::min;

use crate::{
    coding::{total_encoded_len, Optimizer, Parser, Segment},
    types::{EcLevel, Mode, QrError, QrResult, Version},
};

/// The `Bits` structure stores the encoded data for a QR code.
pub struct Bits {
    data: Vec<u8>,
    bit_offset: usize,
    version: Version,
}

impl Bits {
    /// Constructs a new, empty bits structure.
    pub fn new(version: Version) -> Self {
        Self {
            data: Vec::new(),
            bit_offset: 0,
            version,
        }
    }

    /// Pushes an N-bit big-endian integer to the end of the bits.
    ///
    /// Note: It is up to the developer to ensure that `number` really only is
    /// `n` bit in size. Otherwise the excess bits may stomp on the existing
    /// ones.
    fn push_number(&mut self, n: usize, number: u16) {
        debug_assert!(
            n == 16 || n < 16 && number < (1 << n),
            "{} is too big as a {}-bit number",
            number,
            n
        );

        let b = self.bit_offset + n;
        let last_index = self.data.len().wrapping_sub(1);
        match (self.bit_offset, b) {
            (0, 0..=8) => {
                self.data.push((number << (8 - b)) as u8);
            }
            (0, _) => {
                self.data.push((number >> (b - 8)) as u8);
                self.data.push((number << (16 - b)) as u8);
            }
            (_, 0..=8) => {
                self.data[last_index] |= (number << (8 - b)) as u8;
            }
            (_, 9..=16) => {
                self.data[last_index] |= (number >> (b - 8)) as u8;
                self.data.push((number << (16 - b)) as u8);
            }
            _ => {
                self.data[last_index] |= (number >> (b - 8)) as u8;
                self.data.push((number >> (b - 16)) as u8);
                self.data.push((number << (24 - b)) as u8);
            }
        }
        self.bit_offset = b & 7;
    }

    /// Pushes an N-bit big-endian integer to the end of the bits, and check
    /// that the number does not overflow the bits.
    ///
    /// Returns `Err(QrError::DataTooLong)` on overflow.
    pub fn push_number_checked(&mut self, n: usize, number: usize) -> QrResult<()> {
        if n > 16 || number >= (1 << n) {
            Err(QrError::DataTooLong)
        } else {
            self.push_number(n, number as u16);
            Ok(())
        }
    }

    /// Reserves `n` extra bits of space for pushing.
    fn reserve(&mut self, n: usize) {
        let extra_bytes = (n + (8 - self.bit_offset) % 8) / 8;
        self.data.reserve(extra_bytes);
    }

    /// Convert the bits into a bytes vector.
    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    /// Total number of bits currently pushed.
    pub fn len(&self) -> usize {
        if self.bit_offset == 0 {
            self.data.len() * 8
        } else {
            (self.data.len() - 1) * 8 + self.bit_offset
        }
    }

    /// Whether there are any bits pushed.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The maximum number of bits allowed by the provided QR code version and
    /// error correction level.
    ///
    /// # Errors
    ///
    /// Returns `Err(QrError::InvalidVersion)` if it is not valid to use the
    /// `ec_level` for the given version (e.g. `Version::Micro(1)` with
    /// `EcLevel::H`).
    pub fn max_len(&self, ec_level: EcLevel) -> QrResult<usize> {
        self.version.fetch(ec_level, &DATA_LENGTHS)
    }

    /// Version of the QR code.
    pub fn version(&self) -> Version {
        self.version
    }

    /// Push the mode indicator to the end of the bits.
    ///
    /// # Errors
    ///
    /// If the mode is not supported in the provided version, this method
    /// returns `Err(QrError::UnsupportedCharacterSet)`.
    pub fn push_mode_indicator(&mut self, mode: Mode) -> QrResult<()> {
        let number = match (self.version, mode) {
            (Version::Micro(1), Mode::Numeric) => return Ok(()),
            (Version::Micro(_), Mode::Numeric) => 0,
            (Version::Micro(_), Mode::Alphanumeric) => 1,
            (Version::Micro(_), Mode::Byte) => 0b10,
            (Version::Micro(_), Mode::Kanji) => 0b11,
            (Version::Rmqr(_, _), Mode::Numeric) => 0b001,
            (Version::Rmqr(_, _), Mode::Alphanumeric) => 0b010,
            (Version::Rmqr(_, _), Mode::Byte) => 0b011,
            (Version::Rmqr(_, _), Mode::Kanji) => 0b100,
            (Version::Normal(_), Mode::Numeric) => 0b0001,
            (Version::Normal(_), Mode::Alphanumeric) => 0b0010,
            (Version::Normal(_), Mode::Byte) => 0b0100,
            (Version::Normal(_), Mode::Kanji) => 0b1000,
        };
        let bits = self.version.mode_bits_count();
        self.push_number_checked(bits, number)
            .or(Err(QrError::UnsupportedCharacterSet))
    }
}

#[test]
fn test_push_number() {
    let mut bits = Bits::new(Version::Normal(1));

    bits.push_number(3, 0b010); // 0:0 .. 0:3
    bits.push_number(3, 0b110); // 0:3 .. 0:6
    bits.push_number(3, 0b101); // 0:6 .. 1:1
    bits.push_number(7, 0b001_1010); // 1:1 .. 2:0
    bits.push_number(4, 0b1100); // 2:0 .. 2:4
    bits.push_number(12, 0b1011_0110_1101); // 2:4 .. 4:0
    bits.push_number(10, 0b01_1001_0001); // 4:0 .. 5:2
    bits.push_number(15, 0b111_0010_1110_0011); // 5:2 .. 7:1

    let bytes = bits.into_bytes();

    assert_eq!(
        bytes,
        vec![
            0b0101_1010, // 90
            0b1001_1010, // 154
            0b1100_1011, // 203
            0b0110_1101, // 109
            0b0110_0100, // 100
            0b0111_1001, // 121
            0b0111_0001, // 113
            0b1000_0000, // 128
        ]
    );
}

/// Mode::Numeric mode
impl Bits {
    fn push_header(&mut self, mode: Mode, raw_data_len: usize) -> QrResult<()> {
        let length_bits = mode.length_bits_count(self.version);
        self.reserve(length_bits + 4 + mode.data_bits_count(raw_data_len));
        self.push_mode_indicator(mode)?;
        self.push_number_checked(length_bits, raw_data_len)?;
        Ok(())
    }

    /// Encodes a numeric string to the bits.
    ///
    /// The data should only contain the characters 0 to 9.
    ///
    /// # Errors
    ///
    /// Returns `Err(QrError::DataTooLong)` on overflow.
    pub fn push_numeric_data(&mut self, data: &[u8]) -> QrResult<()> {
        self.push_header(Mode::Numeric, data.len())?;
        for chunk in data.chunks(3) {
            let number = chunk
                .iter()
                .map(|b| u16::from(*b - b'0'))
                .fold(0, |a, b| a * 10 + b);
            let length = chunk.len() * 3 + 1;
            self.push_number(length, number);
        }
        Ok(())
    }
}

#[cfg(test)]
mod numeric_tests {
    use crate::bits::Bits;
    use crate::types::{QrError, Version};

    #[test]
    fn test_iso_18004_2006_example_1() {
        let mut bits = Bits::new(Version::Normal(1));
        assert_eq!(bits.push_numeric_data(b"01234567"), Ok(()));
        assert_eq!(
            bits.into_bytes(),
            vec![
                0b0001_0000,
                0b0010_0000,
                0b00001100,
                0b01010110,
                0b01_100001,
                0b1000_0000
            ]
        );
    }

    #[test]
    fn test_iso_18004_2000_example_2() {
        let mut bits = Bits::new(Version::Normal(1));
        assert_eq!(bits.push_numeric_data(b"0123456789012345"), Ok(()));
        assert_eq!(
            bits.into_bytes(),
            vec![
                0b0001_0000,
                0b0100_0000,
                0b00001100,
                0b01010110,
                0b01_101010,
                0b0110_1110,
                0b0001_0100,
                0b11101010,
                0b0101_0000,
            ]
        );
    }

    #[test]
    fn test_iso_18004_2006_example_2() {
        let mut bits = Bits::new(Version::Micro(3));
        assert_eq!(bits.push_numeric_data(b"0123456789012345"), Ok(()));
        assert_eq!(
            bits.into_bytes(),
            vec![
                0b0010_0000,
                0b00000110,
                0b0_0101011,
                0b001_10101,
                0b0011_0111,
                0b0000_1010,
                0b01110101,
                0b0010_1000,
            ]
        );
    }

    #[test]
    fn test_data_too_long_error() {
        let mut bits = Bits::new(Version::Micro(1));
        assert_eq!(
            bits.push_numeric_data(b"12345678"),
            Err(QrError::DataTooLong)
        );
    }
}

/// Mode::Alphanumeric mode

/// In QR code `Mode::Alphanumeric` mode, a pair of alphanumeric characters will
/// be encoded as a base-45 integer. `alphanumeric_digit` converts each
/// character into its corresponding base-45 digit.
///
/// The conversion is specified in ISO/IEC 18004:2006, §8.4.3, Table 5.
#[inline]
fn alphanumeric_digit(character: u8) -> u16 {
    match character {
        b'0'..=b'9' => u16::from(character - b'0'),
        b'A'..=b'Z' => u16::from(character - b'A') + 10,
        b' ' => 36,
        b'$' => 37,
        b'%' => 38,
        b'*' => 39,
        b'+' => 40,
        b'-' => 41,
        b'.' => 42,
        b'/' => 43,
        b':' => 44,
        _ => 0,
    }
}

impl Bits {
    /// Encodes an alphanumeric string to the bits.
    ///
    /// The data should only contain the charaters A to Z (excluding lowercase),
    /// 0 to 9, space, `$`, `%`, `*`, `+`, `-`, `.`, `/` or `:`.
    ///
    /// # Errors
    ///
    /// Returns `Err(QrError::DataTooLong)` on overflow.
    pub fn push_alphanumeric_data(&mut self, data: &[u8]) -> QrResult<()> {
        self.push_header(Mode::Alphanumeric, data.len())?;
        for chunk in data.chunks(2) {
            let number = chunk
                .iter()
                .map(|b| alphanumeric_digit(*b))
                .fold(0, |a, b| a * 45 + b);
            let length = chunk.len() * 5 + 1;
            self.push_number(length, number);
        }
        Ok(())
    }
}

#[cfg(test)]
mod alphanumeric_tests {
    use crate::bits::Bits;
    use crate::types::{QrError, Version};

    #[test]
    fn test_iso_18004_2006_example() {
        let mut bits = Bits::new(Version::Normal(1));
        assert_eq!(bits.push_alphanumeric_data(b"AC-42"), Ok(()));
        assert_eq!(
            bits.into_bytes(),
            vec![
                0b0010_0000,
                0b0010_1001,
                0b11001110,
                0b11100111,
                0b001_00001,
                0b0000_0000
            ]
        );
    }

    #[test]
    fn test_micro_qr_unsupported() {
        let mut bits = Bits::new(Version::Micro(1));
        assert_eq!(
            bits.push_alphanumeric_data(b"A"),
            Err(QrError::UnsupportedCharacterSet)
        );
    }

    #[test]
    fn test_data_too_long() {
        let mut bits = Bits::new(Version::Micro(2));
        assert_eq!(
            bits.push_alphanumeric_data(b"ABCDEFGH"),
            Err(QrError::DataTooLong)
        );
    }
}

/// Mode::Byte mode

impl Bits {
    /// Encodes 8-bit byte data to the bits.
    ///
    /// # Errors
    ///
    /// Returns `Err(QrError::DataTooLong)` on overflow.
    pub fn push_byte_data(&mut self, data: &[u8]) -> QrResult<()> {
        self.push_header(Mode::Byte, data.len())?;
        for b in data {
            self.push_number(8, u16::from(*b));
        }
        Ok(())
    }
}

#[cfg(test)]
mod byte_tests {
    use crate::bits::Bits;
    use crate::types::{QrError, Version};

    #[test]
    fn test() {
        let mut bits = Bits::new(Version::Normal(1));
        assert_eq!(
            bits.push_byte_data(b"\x12\x34\x56\x78\x9a\xbc\xde\xf0"),
            Ok(())
        );
        assert_eq!(
            bits.into_bytes(),
            vec![
                0b0100_0000,
                0b1000_0001,
                0b0010_0011,
                0b0100_0101,
                0b0110_0111,
                0b1000_1001,
                0b1010_1011,
                0b1100_1101,
                0b1110_1111,
                0b0000_0000,
            ]
        );
    }

    #[test]
    fn test_micro_qr_unsupported() {
        let mut bits = Bits::new(Version::Micro(2));
        assert_eq!(
            bits.push_byte_data(b"?"),
            Err(QrError::UnsupportedCharacterSet)
        );
    }

    #[test]
    fn test_data_too_long() {
        let mut bits = Bits::new(Version::Micro(3));
        assert_eq!(
            bits.push_byte_data(b"0123456701234567"),
            Err(QrError::DataTooLong)
        );
    }
}

/// Mode::Kanji mode

impl Bits {
    /// Encodes Shift JIS double-byte data to the bits.
    ///
    /// # Errors
    ///
    /// Returns `Err(QrError::DataTooLong)` on overflow.
    ///
    /// Returns `Err(QrError::InvalidCharacter)` if the data is not Shift JIS
    /// double-byte data (e.g. if the length of data is not an even number).
    pub fn push_kanji_data(&mut self, data: &[u8]) -> QrResult<()> {
        self.push_header(Mode::Kanji, data.len() / 2)?;
        for kanji in data.chunks(2) {
            if kanji.len() != 2 {
                return Err(QrError::InvalidCharacter);
            }
            let cp = u16::from(kanji[0]) * 256 + u16::from(kanji[1]);
            let bytes = if cp < 0xe040 {
                cp - 0x8140
            } else {
                cp - 0xc140
            };
            let number = (bytes >> 8) * 0xc0 + (bytes & 0xff);
            self.push_number(13, number);
        }
        Ok(())
    }
}

#[cfg(test)]
mod kanji_tests {
    use crate::bits::Bits;
    use crate::types::{QrError, Version};

    #[test]
    fn test_iso_18004_example() {
        let mut bits = Bits::new(Version::Normal(1));
        assert_eq!(bits.push_kanji_data(b"\x93\x5f\xe4\xaa"), Ok(()));
        assert_eq!(
            bits.into_bytes(),
            vec![
                0b1000_0000,
                0b0010_0110,
                0b11001111,
                0b1_1101010,
                0b1010_1000
            ]
        );
    }

    #[test]
    fn test_micro_qr_unsupported() {
        let mut bits = Bits::new(Version::Micro(2));
        assert_eq!(
            bits.push_kanji_data(b"?"),
            Err(QrError::UnsupportedCharacterSet)
        );
    }

    #[test]
    fn test_data_too_long() {
        let mut bits = Bits::new(Version::Micro(3));
        assert_eq!(
            bits.push_kanji_data(b"\x93_\x93_\x93_\x93_\x93_\x93_\x93_\x93_"),
            Err(QrError::DataTooLong)
        );
    }
}

// This table is copied from ISO/IEC 18004:2006 §6.4.10, Table 7.
static DATA_LENGTHS: [[usize; 4]; 76] = [
    // Normal versions
    [152, 128, 104, 72],
    [272, 224, 176, 128],
    [440, 352, 272, 208],
    [640, 512, 384, 288],
    [864, 688, 496, 368],
    [1088, 864, 608, 480],
    [1248, 992, 704, 528],
    [1552, 1232, 880, 688],
    [1856, 1456, 1056, 800],
    [2192, 1728, 1232, 976],
    [2592, 2032, 1440, 1120],
    [2960, 2320, 1648, 1264],
    [3424, 2672, 1952, 1440],
    [3688, 2920, 2088, 1576],
    [4184, 3320, 2360, 1784],
    [4712, 3624, 2600, 2024],
    [5176, 4056, 2936, 2264],
    [5768, 4504, 3176, 2504],
    [6360, 5016, 3560, 2728],
    [6888, 5352, 3880, 3080],
    [7456, 5712, 4096, 3248],
    [8048, 6256, 4544, 3536],
    [8752, 6880, 4912, 3712],
    [9392, 7312, 5312, 4112],
    [10208, 8000, 5744, 4304],
    [10960, 8496, 6032, 4768],
    [11744, 9024, 6464, 5024],
    [12248, 9544, 6968, 5288],
    [13048, 10136, 7288, 5608],
    [13880, 10984, 7880, 5960],
    [14744, 11640, 8264, 6344],
    [15640, 12328, 8920, 6760],
    [16568, 13048, 9368, 7208],
    [17528, 13800, 9848, 7688],
    [18448, 14496, 10288, 7888],
    [19472, 15312, 10832, 8432],
    [20528, 15936, 11408, 8768],
    [21616, 16816, 12016, 9136],
    [22496, 17728, 12656, 9776],
    [23648, 18672, 13328, 10208],
    // Micro versions
    [20, 0, 0, 0],
    [40, 32, 0, 0],
    [84, 68, 0, 0],
    [128, 112, 80, 0],
    // rMQR versions
    [0, 48, 0, 24],
    [0, 96, 0, 56],
    [0, 160, 0, 80],
    [0, 224, 0, 112],
    [0, 352, 0, 192],
    [0, 96, 0, 56],
    [0, 168, 0, 88],
    [0, 248, 0, 136],
    [0, 336, 0, 176],
    [0, 504, 0, 264],
    [0, 56, 0, 40],
    [0, 152, 0, 88],
    [0, 248, 0, 120],
    [0, 344, 0, 184],
    [0, 456, 0, 232],
    [0, 672, 0, 336],
    [0, 96, 0, 56],
    [0, 216, 0, 104],
    [0, 304, 0, 160],
    [0, 424, 0, 232],
    [0, 584, 0, 280],
    [0, 848, 0, 432],
    [0, 264, 0, 120],
    [0, 384, 0, 208],
    [0, 536, 0, 248],
    [0, 704, 0, 384],
    [0, 1016, 0, 552],
    [0, 312, 0, 168],
    [0, 448, 0, 224],
    [0, 624, 0, 304],
    [0, 800, 0, 448],
    [0, 1216, 0, 608],
];

impl Bits {
    /// Pushes the ending bits to indicate no more data.
    ///
    /// # Errors
    ///
    /// Returns `Err(QrError::DataTooLong)` on overflow.
    ///
    /// Returns `Err(QrError::InvalidVersion)` if it is not valid to use the
    /// `ec_level` for the given version (e.g. `Version::Micro(1)` with
    /// `EcLevel::H`).
    pub fn push_terminator(&mut self, ec_level: EcLevel) -> QrResult<()> {
        let terminator_size = match self.version {
            Version::Micro(a) => a * 2 + 1,
            Version::Rmqr(_, _) => 3,
            _ => 4,
        };

        let cur_length = self.len();
        let data_length = self.max_len(ec_level)?;
        if cur_length > data_length {
            return Err(QrError::DataTooLong);
        }

        let terminator_size = min(terminator_size as usize, data_length - cur_length);
        if terminator_size > 0 {
            self.push_number(terminator_size, 0);
        }

        if self.len() < data_length {
            const PADDING_BYTES: &[u8] = &[0b1110_1100, 0b0001_0001];

            self.bit_offset = 0;
            let data_bytes_length = data_length / 8;
            let padding_bytes_count = data_bytes_length.saturating_sub(self.data.len());
            let padding = PADDING_BYTES
                .iter()
                .cloned()
                .cycle()
                .take(padding_bytes_count);
            self.data.extend(padding);
        }

        if self.len() < data_length {
            self.data.push(0);
        }

        Ok(())
    }
}

impl Bits {
    /// Push a segmented data to the bits, and then terminate it.
    ///
    /// # Errors
    ///
    /// Returns `Err(QrError::DataTooLong)` on overflow.
    ///
    /// Returns `Err(QrError::InvalidData)` if the segment refers to incorrectly
    /// encoded byte sequence.
    pub fn push_segments<I>(&mut self, data: &[u8], segments_iter: I) -> QrResult<()>
    where
        I: Iterator<Item = Segment>,
    {
        for segment in segments_iter {
            let slice = &data[segment.begin..segment.end];
            match segment.mode {
                Mode::Numeric => self.push_numeric_data(slice),
                Mode::Alphanumeric => self.push_alphanumeric_data(slice),
                Mode::Byte => self.push_byte_data(slice),
                Mode::Kanji => self.push_kanji_data(slice),
            }?;
        }
        Ok(())
    }

    /// Pushes the data the bits, using the optimal encoding.
    ///
    /// # Errors
    ///
    /// Returns `Err(QrError::DataTooLong)` on overflow.
    pub fn push_optimal_data(&mut self, data: &[u8]) -> QrResult<()> {
        let segments = Parser::new(data).optimize(self.version);
        self.push_segments(data, segments)
    }
}

/// Auto version minimization

/// Automatically determines the minimum version to store the data, and encode
/// the result.
///
/// This method will not consider any Micro QR code or rMQR versions.
///
/// # Errors
///
/// Returns `Err(QrError::DataTooLong)` if the data is too long to fit even the
/// highest QR code version.
pub fn encode_auto(data: &[u8], ec_level: EcLevel) -> QrResult<Bits> {
    let segments = Parser::new(data).collect::<Vec<Segment>>();
    for version in &[Version::Normal(9), Version::Normal(26), Version::Normal(40)] {
        let opt_segments = Optimizer::new(segments.iter().copied(), *version).collect::<Vec<_>>();
        let total_len = total_encoded_len(&opt_segments, *version);
        let data_capacity = version
            .fetch(ec_level, &DATA_LENGTHS)
            .expect("invalid DATA_LENGTHS");
        if total_len <= data_capacity {
            let min_version = find_min_version(total_len, ec_level);
            let mut bits = Bits::new(min_version);
            bits.reserve(total_len);
            bits.push_segments(data, opt_segments.into_iter())?;
            bits.push_terminator(ec_level)?;
            return Ok(bits);
        }
    }
    Err(QrError::DataTooLong)
}

/// Finds the smallest version (QR code only) that can store N bits of data
/// in the given error correction level.
fn find_min_version(length: usize, ec_level: EcLevel) -> Version {
    let mut base = 0_usize;
    let mut size = 39;
    while size > 1 {
        let half = size / 2;
        let mid = base + half;
        // mid is always in [0, size).
        // mid >= 0: by definition
        // mid < size: mid = size / 2 + size / 4 + size / 8 ...
        base = if DATA_LENGTHS[mid][ec_level as usize] > length {
            base
        } else {
            mid
        };
        size -= half;
    }
    // base is always in [0, mid) because base <= mid.
    base = if DATA_LENGTHS[base][ec_level as usize] >= length {
        base
    } else {
        base + 1
    };
    Version::Normal((base + 1) as u8)
}

/// Auto rMQR's version minimization strategy
#[derive(Debug, Clone, Copy)]
pub enum RmqrStrategy {
    /// minimize the width
    Width,
    /// minimize the height
    Height,
    /// minimize the area
    Area,
}

/// Auto rMQR's version minimization

/// Automatically determines the minimum version to store the data, and encode
/// the result.
///
/// This method will not consider QR or Micro QR code versions.
///
/// # Errors
///
/// Returns `Err(QrError::DataTooLong)` if the data is too long to fit even the
/// highest rMQR code version.
pub fn encode_auto_rmqr(data: &[u8], ec_level: EcLevel, strategy: RmqrStrategy) -> QrResult<Bits> {
    let segments = Parser::new(data).collect::<Vec<Segment>>();
    let mut possible_versions = vec![];
    for width in Version::rmqr_all_width() {
        for height in Version::rmqr_all_height() {
            let version = Version::Rmqr(height, width);
            if !version.is_rmqr() {
                continue;
            }
            let opt_segments =
                Optimizer::new(segments.iter().copied(), version).collect::<Vec<_>>();
            let total_len = total_encoded_len(&opt_segments, version);
            let data_capacity = version.fetch(ec_level, &DATA_LENGTHS)?;
            if total_len <= data_capacity {
                possible_versions.push(version);
                break;
            }
        }
    }

    let min_version = match strategy {
        RmqrStrategy::Width => possible_versions.first(), // possible_versions is already sorted by width
        RmqrStrategy::Height => possible_versions.iter().min_by_key(|v| v.height()),
        RmqrStrategy::Area => possible_versions.iter().min_by_key(|v| v.area()),
    };

    if let Some(version) = min_version {
        let mut bits = Bits::new(*version);
        let opt_segments = Optimizer::new(segments.iter().copied(), *version).collect::<Vec<_>>();
        bits.reserve(total_encoded_len(&opt_segments, *version));
        bits.push_segments(data, opt_segments.into_iter())?;
        bits.push_terminator(ec_level)?;
        return Ok(bits);
    }
    Err(QrError::DataTooLong)
}
