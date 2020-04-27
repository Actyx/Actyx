/*
 * Copyright 2020 Actyx AG
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
//! Types that you may want to use in describing the event payload data

mod arcval;
mod fixnum;

pub use arcval::ArcVal;
pub use fixnum::{fixnum, FixNum};

pub mod fixnum_types {
    //! collection of useful types for use with [`fix_num`](../fn.fixnum.html)
    //!
    //! # Example
    //!
    //! ```rust
    //! use actyxos_sdk::types::{FixNum, fixnum, fixnum_types::*};
    //!
    //! // create a fixed-point number with 32 bits fractional part
    //! let a: FixNum<U32> = fixnum::<U32, _>(12345);
    //! ```
    pub use fixed::types::extra::*;
    pub use fixed::FixedI128;
}
