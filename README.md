[![](https://docs.rs/bufrng/badge.svg)](https://docs.rs/bufrng/)
[![](https://img.shields.io/crates/v/bufrng.svg)](https://crates.io/crates/bufrng)
[![](https://img.shields.io/crates/d/bufrng.svg)](https://crates.io/crates/bufrng)
[![Build Status](https://dev.azure.com/fitzgen/bufrng/_apis/build/status/fitzgen.bufrng?branchName=master)](https://dev.azure.com/fitzgen/bufrng/_build/latest?definitionId=2&branchName=master)

# `bufrng`


`BufRng` is a "random" number generator that simply yields pre-determined values
from a buffer, and yields `0`s once the buffer is exhausted.

<div align="center">

  <p>⚠⚠⚠</p>

  <p><strong>This RNG is not suitable for anything other than testing and
  fuzzing! It is not suitable for cryptography! It is not suitable for
  generating pseudo-random numbers!</strong></p>

  <p>⚠⚠⚠</p>

</div>

### Why?

`BufRng` is useful for reinterpreting raw input bytes from
[libFuzzer](https://rust-fuzz.github.io/book/cargo-fuzz.html) or
[AFL](https://rust-fuzz.github.io/book/afl.html) as an RNG that is used with
structure-aware test case generators (e.g.
[`quickcheck::Arbitrary`](https://docs.rs/quickcheck/0.9.0/quickcheck/trait.Arbitrary.html)). This
combines the power of coverage-guided fuzzing with structure-aware fuzzing.

### Example

Let's say we are developing a crate to convert back and forth between RGB and
HSL color representations.

First, we can implement `quickcheck::Arbitrary` for our color types to get
structure-aware test case generators. Then, we can use these with `quickcheck`'s
own test runner infrastructure to assert various properties about our code (such
as it never panics, or that RGB -> HSL -> RGB is the identity function) and
`quickcheck` will generate random instances of `Rgb` and `Hsl` to check this
property against.

```rust
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

```rust
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

