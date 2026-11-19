# Third-party material

## NumPy (random number tables and algorithm)

`rust/src/dcf/ziggurat_tables.rs` has the ziggurat tables of numpy v2.5.3
(`numpy/random/src/distributions/ziggurat_constants.h`). `rust/src/dcf/nprandom.rs`
follows the algorithms of `distributions.c` (`random_standard_normal`), of
`bit_generator.pyx` (`SeedSequence`) and of `_pcg64.pyx`. The Rust files make the same
random numbers as numpy, so each Python result stays the same.
Source: https://github.com/numpy/numpy/tree/v2.5.3

NumPy is licensed under the BSD 3-Clause license:

    Copyright (c) 2005-2025, NumPy Developers.
    All rights reserved.

    Redistribution and use in source and binary forms, with or without
    modification, are permitted provided that the following conditions are
    met:

        * Redistributions of source code must retain the above copyright
           notice, this list of conditions and the following disclaimer.

        * Redistributions in binary form must reproduce the above
           copyright notice, this list of conditions and the following
           disclaimer in the documentation and/or other materials provided
           with the distribution.

        * Neither the name of the NumPy Developers nor the names of any
           contributors may be used to endorse or promote products derived
           from this software without specific prior written permission.

    THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
    "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
    LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
    A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
    OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
    SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
    LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
    DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
    THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
    (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
    OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

## PCG64 (bit generator)

The `PCG64` generator (XSL-RR 128/64, `src/pcg64/pcg64.h` and `pcg64.c` in numpy) is the
work of Melissa O'Neill (2014) and Robert Kern (2015), under the Apache License 2.0
(http://www.apache.org/licenses/LICENSE-2.0). `nprandom.rs` is a new Rust program of the
same algorithm: the multiplier, the step, the output function and the seeding.
