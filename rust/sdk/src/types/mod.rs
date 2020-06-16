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
pub use fixnum::FixNum;

pub mod intern_arc {
    pub use intern_arc::*;
}

pub mod fixnum_types {
    //! collection of useful types for use with `FixNum`
    //!
    //! # Example
    //!
    //! ```rust
    //! use actyxos_sdk::types::{FixNum, fixnum_types::U32};
    //!
    //! // convert to fixed-point number with 32 bits fractional part, saturating on overflow
    //! let a: FixNum<U32> = FixNum::<U32>::saturating(12345);
    //!
    //! // convert it to a float
    //! let f = a.to_num_checked::<f64>().unwrap();
    //! assert_eq!(f, 12345f64);
    //!
    //! // convert a float to fixed-point, wrapping around on overflow
    //! let b: FixNum<U32> = FixNum::wrapping(13.7e250f64);
    //! ```
    #[doc(no_inline)]
    pub use fixed::traits::{FromFixed, LossyFrom, LossyInto, ToFixed};
    #[doc(no_inline)]
    pub use fixed::types::extra::*;
    #[doc(no_inline)]
    pub use fixed::FixedI128;
}
