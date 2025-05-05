use crate::types::sealed::Sealed;
use byteorder::{ByteOrder, LittleEndian};
use std::fmt::Debug;
use std::ops::RangeInclusive;

// Create the base values which everything comes from
pub type BeacnValue = [u8; 4];
pub struct MessageValue<T>(pub T);

// This deals with key packing on various nodes
pub struct PackedEnumKey<B, K>(pub B, pub K);
impl<B, K> PackedEnumKey<B, K>
where
    B: Copy + Into<u8> + strum::IntoEnumIterator,
    K: Copy + Into<u8> + strum::IntoEnumIterator,
{
    pub fn from_encoded(encoded: u8) -> Option<Self> {
        let upper = (encoded & 0xf0) >> 4;
        let lower = encoded & 0x0f;

        let upper = B::iter().find(|b| (*b).into() == upper)?;
        let lower = K::iter().find(|k| (*k).into() == lower)?;

        Some(Self(upper, lower))
    }

    pub fn to_encoded(&self) -> u8 {
        (self.0.into() << 4) | (self.1.into() & 0x0f)
    }

    pub fn get_upper(&self) -> B {
        self.0
    }

    pub fn get_lower(&self) -> K {
        self.1
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RGB {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

pub(crate) mod sealed {
    use crate::types::RGB;

    pub trait Sealed {}
    impl Sealed for bool {}

    impl Sealed for u8 {}
    impl Sealed for u32 {}

    impl Sealed for i8 {}
    impl Sealed for i32 {}

    impl Sealed for f32 {}

    impl Sealed for RGB {}
}

pub trait FromInner<U>: Sized {
    fn from_inner(value: U) -> Self;
}
pub trait ToInner<U> {
    fn to_inner(&self) -> U;
}

pub trait WriteBeacn: Sealed {
    fn write_beacn(&self) -> BeacnValue;
}
pub trait ReadBeacn: Sized {
    fn read_beacn(buf: &BeacnValue) -> Self;
}

pub trait HasRange<T> {
    fn range() -> RangeInclusive<T>;
}

// -----------------------------------------------------------------------------------------------

impl WriteBeacn for bool {
    fn write_beacn(&self) -> BeacnValue {
        let mut value = [0; 4];
        LittleEndian::write_u32(&mut value, *self as u32);
        value
    }
}
impl ReadBeacn for bool {
    fn read_beacn(buf: &BeacnValue) -> Self {
        let value = LittleEndian::read_u32(buf);
        if (0..=1).contains(&value) {
            return value == 1;
        }
        panic!("Incorrect Boolean Received: {}", value);
    }
}

// -----------------------------------------------------------------------------------------------

impl WriteBeacn for u8 {
    fn write_beacn(&self) -> BeacnValue {
        let mut value = [0; 4];
        LittleEndian::write_u32(&mut value, *self as u32);
        value
    }
}
impl ReadBeacn for u8 {
    fn read_beacn(buf: &BeacnValue) -> Self {
        // We'll just grab the last byte
        buf[3]
    }
}
impl HasRange<u8> for u8 {
    fn range() -> RangeInclusive<u8> {
        0..=u8::MAX
    }
}
impl FromInner<u8> for u8 {
    fn from_inner(value: u8) -> Self {
        value
    }
}
impl ToInner<u8> for u8 {
    fn to_inner(&self) -> u8 {
        *self
    }
}

// -----------------------------------------------------------------------------------------------

impl WriteBeacn for u32 {
    fn write_beacn(&self) -> BeacnValue {
        let mut value = [0; 4];
        LittleEndian::write_u32(&mut value, *self);
        value
    }
}

impl ReadBeacn for u32 {
    fn read_beacn(buf: &BeacnValue) -> Self {
        LittleEndian::read_u32(buf)
    }
}
impl HasRange<u32> for u32 {
    fn range() -> RangeInclusive<u32> {
        0..=u32::MAX
    }
}
impl FromInner<u32> for u32 {
    fn from_inner(value: u32) -> Self {
        value
    }
}
impl ToInner<u32> for u32 {
    fn to_inner(&self) -> u32 {
        *self
    }
}

// -----------------------------------------------------------------------------------------------

impl WriteBeacn for i8 {
    fn write_beacn(&self) -> BeacnValue {
        let mut value = [0; 4];
        LittleEndian::write_i32(&mut value, *self as i32);
        value
    }
}
impl ReadBeacn for i8 {
    fn read_beacn(buf: &BeacnValue) -> Self {
        buf[3] as i8
    }
}
impl HasRange<i8> for i8 {
    fn range() -> RangeInclusive<i8> {
        i8::MIN..=i8::MAX
    }
}
impl FromInner<i8> for i8 {
    fn from_inner(value: i8) -> Self {
        value
    }
}
impl ToInner<i8> for i8 {
    fn to_inner(&self) -> i8 {
        *self
    }
}

// -----------------------------------------------------------------------------------------------

impl WriteBeacn for i32 {
    fn write_beacn(&self) -> BeacnValue {
        let mut value = [0; 4];
        LittleEndian::write_i32(&mut value, *self);
        value
    }
}
impl ReadBeacn for i32 {
    fn read_beacn(buf: &BeacnValue) -> Self {
        LittleEndian::read_i32(buf)
    }
}

impl HasRange<i32> for i32 {
    fn range() -> RangeInclusive<i32> {
        i32::MIN..=i32::MAX
    }
}
impl FromInner<i32> for i32 {
    fn from_inner(value: i32) -> Self {
        value
    }
}
impl ToInner<i32> for i32 {
    fn to_inner(&self) -> i32 {
        *self
    }
}

// -----------------------------------------------------------------------------------------------

impl WriteBeacn for f32 {
    fn write_beacn(&self) -> BeacnValue {
        let mut value = [0; 4];
        LittleEndian::write_f32(&mut value, *self);
        value
    }
}
impl ReadBeacn for f32 {
    fn read_beacn(buf: &BeacnValue) -> Self {
        LittleEndian::read_f32(buf)
    }
}
impl HasRange<f32> for f32 {
    fn range() -> RangeInclusive<f32> {
        f32::MIN..=f32::MAX
    }
}
impl FromInner<f32> for f32 {
    fn from_inner(value: f32) -> Self {
        value
    }
}
impl ToInner<f32> for f32 {
    fn to_inner(&self) -> f32 {
        *self
    }
}

// -----------------------------------------------------------------------------------------------

impl WriteBeacn for RGB {
    fn write_beacn(&self) -> BeacnValue {
        [self.blue, self.green, self.red, 0]
    }
}

impl ReadBeacn for RGB {
    fn read_beacn(buf: &BeacnValue) -> Self {
        Self {
            red: buf[2],
            green: buf[1],
            blue: buf[0],
            alpha: buf[3],
        }
    }
}

// -----------------------------------------------------------------------------------------------
// Timeframe is used for most Attack / Release values

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TimeFrame(pub f32);
impl HasRange<f32> for TimeFrame {
    fn range() -> RangeInclusive<f32> {
        1.0..=2000.0
    }
}
impl FromInner<f32> for TimeFrame {
    fn from_inner(value: f32) -> Self {
        Self(value)
    }
}
impl ToInner<f32> for TimeFrame {
    fn to_inner(&self) -> f32 {
        self.0
    }
}

// -----------------------------------------------------------------------------------------------
// Make-up Gain is used in a couple of places

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MakeUpGain(pub f32);
impl HasRange<f32> for MakeUpGain {
    fn range() -> RangeInclusive<f32> {
        0.0..=12.0
    }
}
impl FromInner<f32> for MakeUpGain {
    fn from_inner(value: f32) -> Self {
        Self(value)
    }
}
impl ToInner<f32> for MakeUpGain {
    fn to_inner(&self) -> f32 {
        self.0
    }
}

// -----------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Percent(pub f32);
impl HasRange<f32> for Percent {
    fn range() -> RangeInclusive<f32> {
        0.0..=100.0
    }
}
impl FromInner<f32> for Percent {
    fn from_inner(value: f32) -> Self {
        Self(value)
    }
}
impl ToInner<f32> for Percent {
    fn to_inner(&self) -> f32 {
        self.0
    }
}

// -----------------------------------------------------------------------------------------------

/// This function will read a value which has an implemented range limitation and spit
/// out the original type. So for example, if you're converting a BeacnValue which is
/// intended to be a float into a specific format (eg. HeadphoneLevel), you'd call:
/// ready_value::<f32, HeadphoneLevel>(bytes)
///
/// This will then read the Beacn value into an f32, and do a range check using HeadphoneLevel
/// before returning the HeadphoneLevel.
///
/// This code is configured to panic! if something goes wrong, we shouldn't be sending or receiving
/// bad data, so we'll just crash.
pub fn read_value<T, U>(bytes: &BeacnValue) -> T
where
    U: ReadBeacn + PartialOrd + Copy + Debug,
    T: HasRange<U> + FromInner<U>,
{
    let inner: U = U::read_beacn(bytes);
    let range = T::range();
    if !range.contains(&inner) {
        panic!("Value {:?} is out of expected range {:?}", inner, range);
    }
    T::from_inner(inner)
}

/// Similar to above, except for writing values, you pass in <HeadphoneLevel, f32>, it'll convert
/// and validate the range, before writing the final value.
pub fn write_value<T, U>(value: &T) -> BeacnValue
where
    T: HasRange<U> + ToInner<U>,
    U: WriteBeacn + PartialOrd + Copy + Debug,
{
    let inner = value.to_inner();
    if !T::range().contains(&inner) {
        panic!(
            "Attempted to write value {:?} outside of valid range {:?}",
            inner,
            T::range()
        );
    }
    U::write_beacn(&inner)
}

impl From<BeacnValue> for MessageValue<RGB> {
    fn from(value: BeacnValue) -> Self {
        Self(RGB {
            red: value[2],
            green: value[1],
            blue: value[0],
            alpha: value[3],
        })
    }
}

impl From<MessageValue<RGB>> for BeacnValue {
    fn from(value: MessageValue<RGB>) -> Self {
        // The format for this is ARGB, but little endian..
        [value.0.blue, value.0.green, value.0.red, 0]
    }
}

#[macro_export]
macro_rules! generate_range {
    ($name:ident, $type:ty, $range:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct $name(pub $type);

        impl $crate::types::HasRange<$type> for $name {
            fn range() -> std::ops::RangeInclusive<$type> {
                $range
            }
        }

        impl $crate::types::FromInner<$type> for $name {
            fn from_inner(value: $type) -> Self {
                Self(value)
            }
        }

        impl $crate::types::ToInner<$type> for $name {
            fn to_inner(&self) -> $type {
                self.0
            }
        }
    };
}
