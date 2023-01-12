//! Mu's core value type.
//!
//! Mu values are 64 bits, and utilize NaN boxing (also known as NaN tagging)
//! for packing both the type tag bits and value bits into a single 64-bit
//! package.
//!
//! A 64-bit floating-point number is stored by separating the exponent and
//! mantissa bits. The mantissa tells you the actual numerical value, and the
//! exponent tells you where the decimal point goes.
//!
//! Floating-point numbers have special values known as NaNs (Not a Number).
//! NaNs are used to represent invalid numbers, such as those produced when you
//! divide by zero. There are two main kinds of NaNs:
//!
//! - Signalling NaNs
//! - Quiet NaNs
//!
//! Doing arithmetic with a signalling NaN will result in the CPU aborting the
//! process, but doing so with a quiet NaN will simply produce another quiet
//! NaN.
//!
//! A quiet NaN looks like this:
//!
//! ```text,ignore
//!    ┌────────────────────────────────────────────────────────────────────────┐
//!    │01111111_11111100_00000000_00000000_00000000_00000000_00000000_000000000│
//!    └▲▲──────────▲▲▲───▲────────────────────────────────────────────────────▲┘
//!     │└┬─────────┘││   └──────┬─────────────────────────────────────────────┘
//!     │ └Exponent  ││          └Mantissa
//!     │            │└Intel FP Indef.
//!     └Sign bit    └Quiet bit
//! ```
//!
//! In the above representation, all of the zeroed bits may contain arbitrary
//! values.
//!
//! The second peculiarity that makes NaN boxing work is that no modern 64-bit
//! operating system actually uses the full 64-bit address space. Pointers only
//! ever use the low 48 bits, and the high 16 bits are zeroed out.
//!
//! If we overlap a 48-bit pointer pointing to the end of a 48-bit address range
//! with a quiet NaN, we get something that looks like this:
//!
//! ```text,ignore
//!    ┌───────────────────────────────────────────────────────────────────────┐
//!    │01111111_11111100_11111111_11111111_11111111_11111111_11111111_11111111│
//!    └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! What this means is that we have 3 bits which are completely unused in both
//! pointers and quiet NaNs.
//!
//! NaN boxing is a technique that takes advantage of this "hole" in the
//! possible bit patterns of pointers and quiet NaNs by storing the type of the
//! value in the free bits.
//!
//! Mu values have 5 possible types:
//! - Float
//! - Int
//! - None
//! - Bool
//! - Object
//!
//! ### Float
//!
//! If a value is not a quiet NaN, then it is a float. This means that floats do
//! not require any conversions before usage, only a type check.
//!
//! ```text,ignore
//!    Tag = Not a quiet NaN
//!   ┌┴───────────┐
//! ┌─▼────────────▼────────────────────────────────────────────────────────┐
//! │........_........_........_........_........_........_........_........│
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Int
//!
//! A quiet NaN with all free bits zeroed represents an integer. This makes
//! integers slightly more expensive to use, as the high 16 bits need to be
//! cleared after a type check. Because integers are encoded using two's
//! complement, we can store at most a 32-bit signed integer.
//!
//! ```text,ignore
//!   Tag = 000
//!  ┌┴─────────────┬┐
//! ┌▼──────────────▼▼──────────────────────────────────────────────────────┐
//! │01111111_11111100_00000000_00000000_00000000_00000000_00000000_00000000│
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Bool
//!
//! A quiet NaN with the free bit pattern `001` represents a boolean.
//!
//! ```text,ignore
//!   Tag = 001
//!  ┌┴─────────────┬┐                                      Value (0 or 1) ┐
//! ┌▼──────────────▼▼─────────────────────────────────────────────────────▼┐
//! │01111111_11111101_00000000_00000000_00000000_00000000_00000000_0000000v│
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### None
//!
//! A quiet NaN with the free bit pattern `010` represents a `None` value.
//!
//! ```text,ignore
//!   Tag = 010
//!  ┌┴─────────────┬┐
//! ┌▼──────────────▼▼──────────────────────────────────────────────────────┐
//! │01111111_11111110_00000000_00000000_00000000_00000000_00000000_0000000=│
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Object
//!
//! A quiet NaN with the free bit pattern `011` represents an `Object` pointer.
//!
//! ```text,ignore
//!   Tag = 011
//!  ┌┴─────────────┬┐
//! ┌▼──────────────▼▼──────────────────────────────────────────────────────┐
//! │01111111_11111111_00000000_00000000_00000000_00000000_00000000_00000000│
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Implementation inspired by <https://github.com/Marwes/nanbox> and <http://craftinginterpreters.com/optimization.html>.
//! See the last link for more info.

// TODO: object representation
// maybe just a trait?

pub mod object;
pub mod ptr;
pub mod value;

pub use object::Object;
pub use ptr::Ptr;
pub use value::Value;
