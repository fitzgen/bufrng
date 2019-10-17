/*!

`BufRng` is a "random" number generator that simply yields pre-determined values
from a buffer, and yields `0`s once the buffer is exhausted.

<div align="center">

  <p>⚠⚠⚠</p>

  <p><strong>This RNG is not suitable for anything other than testing and
  fuzzing! It is not suitable for cryptography! It is not suitable for
  generating pseudo-random numbers!</strong></p>

  <p>⚠⚠⚠</p>

</div>

## Why?

`BufRng` is useful for reinterpreting raw input bytes from
[libFuzzer](https://rust-fuzz.github.io/book/cargo-fuzz.html) or
[AFL](https://rust-fuzz.github.io/book/afl.html) as an RNG that is used with
structure-aware test case generators (e.g.
[`quickcheck::Arbitrary`](https://docs.rs/quickcheck/0.9.0/quickcheck/trait.Arbitrary.html)). This
combines the power of coverage-guided fuzzing with structure-aware fuzzing.

## Example

Let's say we are developing a crate to convert back and forth between RGB and
HSL color representations.

First, we can implement `quickcheck::Arbitrary` for our color types to get
structure-aware test case generators. Then, we can use these with `quickcheck`'s
own test runner infrastructure to assert various properties about our code (such
as it never panics, or that RGB -> HSL -> RGB is the identity function) and
`quickcheck` will generate random instances of `Rgb` and `Hsl` to check this
property against.

```no_run
/// A color represented with RGB.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn to_hsl(&self) -> Hsl {
        // ...
#       unimplemented!()
    }
}

/// A color represented with HSL.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsl {
    pub h: f64,
    pub s: f64,
    pub l: f64,
}

impl Hsl {
    pub fn to_rgb(&self) -> Rgb {
        // ...
#       unimplemented!()
    }
}

// Implementations of `quickcheck::Arbitrary` to create structure-aware test
// case generators for `Rgb` and `Hsl`.

use rand::prelude::*;
use quickcheck::{Arbitrary, Gen};

impl Arbitrary for Rgb {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Rgb {
            r: g.gen(),
            g: g.gen(),
            b: g.gen(),
        }
    }
}

impl Arbitrary for Hsl {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Hsl {
            h: g.gen_range(0.0, 360.0),
            s: g.gen_range(0.0, 1.0),
            l: g.gen_range(0.0, 1.0),
        }
    }
}

// Properties that we can have `quickcheck` assert for us.

pub fn rgb_to_hsl_doesnt_panic(rgb: Rgb) {
    let _ = rgb.to_hsl();
}

pub fn rgb_to_hsl_to_rgb_is_identity(rgb: Rgb) {
    assert_eq!(rgb, rgb.to_hsl().to_rgb());
}

#[cfg(test)]
mod tests {
    quickcheck::quickcheck! {
        fn rgb_to_hsl_doesnt_panic(rgb: Rgb) -> bool {
            super::rgb_to_hsl_doesnt_panic(rgb);
            true
        }
    }

    quickcheck::quickcheck! {
        fn rgb_to_hsl_to_rgb_is_identity(rgb: Rgb) -> bool {
            super::rgb_to_hsl_to_rgb_is_identity(rgb);
            true
        }
    }
}
```

Finally, we can *reuse* our existing structure-aware test case generators (the
`Arbitrary` impls) with libFuzzer of AFL inputs with `BufRng`. Thus we can
leverage coverage-guided fuzzing &mdash; where the fuzzer is observing code
coverage while tests are running, and trying to maximize the paths the inputs
cover &mdash; with our existing structure-aware generators.

The following snippet is with [`cargo fuzz` and
libFuzzer](https://rust-fuzz.github.io/book/cargo-fuzz.html), but the concepts
would apply equally well to AFL, for example.

```ignore
// my-rgb-to-hsl-crate/fuzz/fuzz_targets/rgb.rs

#![no_main]

#[macro_use]
extern crate libfuzzer_sys;

use bufrng::BufRng;
use my_rgb_to_hsl_crate::{rgb_to_hsl_doesnt_panic, rgb_to_hsl_to_rgb_is_identity, Rgb};
use quickcheck::Arbitrary;

fuzz_target!(|data: &[u8]| {
    // Create a `BufRng` from the raw data given to us by the fuzzer.
    let mut rng = BufRng::new(data);

    // Generate an `Rgb` instance with it.
    let rgb = Rgb::arbitrary(&mut rng);

    // Assert our properties!
    rgb_to_hsl_doesnt_panic(rgb);
    rgb_to_hsl_to_rgb_is_identity(rgb);
});
```

 */

use rand_core::{Error, RngCore};
use std::slice;

/// A "random" number generator that yields values from a given buffer (and then
/// zeros after the buffer is exhausted).
///
/// See the module documentation for details.
pub struct BufRng<'a> {
    iter: slice::Iter<'a, u8>,
}

impl BufRng<'_> {
    /// Construct a new `BufRng` that yields from the given `data` buffer.
    ///
    /// # Example
    ///
    /// ```
    /// use bufrng::BufRng;
    /// use rand::prelude::*;
    ///
    /// // Create a new `BufRng` by giving it a buffer.
    /// let mut rng = BufRng::new(&[1, 2, 3, 4]);
    ///
    /// // It will generate "random" values, which are copied from the buffer.
    /// assert_eq!(rng.gen::<u32>(), (1 << 24) | (2 << 16) | (3 << 8) | 4);
    ///
    /// // Once the buffer is exhausted, the RNG will keep yielding `0`.
    /// assert_eq!(rng.gen::<u32>(), 0);
    /// assert_eq!(rng.gen::<u32>(), 0);
    /// assert_eq!(rng.gen::<u32>(), 0);
    /// ```
    pub fn new(data: &[u8]) -> BufRng {
        BufRng {
            iter: data.iter(),
        }
    }
    
    // Retrieve next byte from underlying iterator
    // or zero if it is exhausted and convert it into u32.
    fn next(&mut self) -> u32 {
        self.iter.next().cloned().unwrap_or(0).into()
    }
}

// NB: all `RngCore` get a blanket `Rng` implementation.
impl RngCore for BufRng<'_> {
    fn next_u32(&mut self) -> u32 {
        let a = self.next();
        let b = self.next();
        let c = self.next();
        let d = self.next();
        (a << 24) | (b << 16) | (c << 8) | d
    }

    fn next_u64(&mut self) -> u64 {
        rand_core::impls::next_u64_via_u32(self)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        rand_core::impls::fill_bytes_via_next(self, dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        Ok(self.fill_bytes(dest))
    }
}
