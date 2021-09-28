// https://github.com/rust-lang/rust-bindgen/issues/628
// Maybe replace with: https://github.com/NeoCogi/rs-ctypes

pub type c_char = i8;
pub type c_schar = i8;
pub type c_uchar = u8;
pub type c_short = i16;
pub type c_ushort = u16;
pub type c_int = i32;
pub type c_uint = u32;

#[cfg(any(target_pointer_width = "32", windows))]
pub type c_long = i32;
#[cfg(any(target_pointer_width = "32", windows))]
pub type c_ulong = u32;

#[cfg(all(target_pointer_width = "64", not(windows)))]
pub type c_long = i64;
#[cfg(all(target_pointer_width = "64", not(windows)))]
pub type c_ulong = u64;

pub type c_longlong = i64;
pub type c_ulonglong = u64;
pub type c_float = f32;
pub type c_double = f64;

pub use core::ffi::c_void;
