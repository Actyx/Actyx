/*
 * Copyright 2021 Actyx AG
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

//! Implementation of [multiformats/unsigned-varint](https://github.com/multiformats/unsigned-varint)

macro_rules! declare_varint {
    ($id:ident, $len:literal) => {
        pub mod $id {
            #[derive(Debug, Clone, Copy)]
            pub struct Varint {
                len: u8,
                bytes: [u8; $len],
            }

            impl AsRef<[u8]> for Varint {
                fn as_ref(&self) -> &[u8] {
                    &self.bytes[0..self.len as usize]
                }
            }

            pub fn encode(mut v: $id) -> Varint {
                let mut bytes = [0u8; $len];
                let mut len = 0;

                loop {
                    bytes[len as usize] = (v & 127) as u8 | 128;
                    v >>= 7;
                    len += 1;
                    if v == 0 {
                        break;
                    }
                }
                bytes[len as usize - 1] &= 127;

                Varint { len, bytes }
            }

            pub fn decode(bytes: &[u8]) -> Option<$id> {
                let mut v: $id = 0;
                let mut bits = 0;

                for b in bytes {
                    v = v.checked_add(((*b as $id) & 127) << bits)?;
                    bits += 7;
                    if *b < 128 {
                        if *b == 0 && bits > 7 {
                            // not minimal encoding
                            return None;
                        }
                        if bytes.len() * 7 > bits {
                            // trailing garbage
                            return None;
                        }
                        break;
                    }
                }

                Some(v)
            }
        }
    };
}

declare_varint!(u8, 2);
declare_varint!(u16, 3);
declare_varint!(u32, 5);
declare_varint!(u64, 10);
declare_varint!(u128, 19);

#[cfg(test)]
mod tests {
    use quickcheck::TestResult;
    use std::fmt::Debug;

    use super::*;

    fn should_equal<T: Debug + PartialEq>(left: T, right: T) -> TestResult {
        if left == right {
            TestResult::passed()
        } else {
            TestResult::error(format!("{:?} does not equal {:?}", left, right))
        }
    }

    #[test]
    fn test() {
        assert_eq!(u32::encode(1234).as_ref(), &[210, 9]);
        assert_eq!(u32::encode(128).as_ref(), &[128, 1]);
        assert_eq!(u32::decode(&[128, 0]), None); // not minimal
        assert_eq!(u32::decode(&[128, 1, 0]), None); // trailing garbage
    }

    quickcheck::quickcheck! {
        fn size_u8(v: u8) -> TestResult {
            let bits = 1.max(8 - v.leading_zeros() as usize);
            let bytes = (bits + 6) / 7;
            let res = u8::encode(v);
            should_equal(res.as_ref().len(), bytes)
        }
        fn size_u16(v: u16) -> TestResult {
            let bits = 1.max(16 - v.leading_zeros() as usize);
            let bytes = (bits + 6) / 7;
            let res = u16::encode(v);
            should_equal(res.as_ref().len(), bytes)
        }
        fn size_u32(v: u32) -> TestResult {
            let bits = 1.max(32 - v.leading_zeros() as usize);
            let bytes = (bits + 6) / 7;
            let res = u32::encode(v);
            should_equal(res.as_ref().len(), bytes)
        }
        fn size_u64(v: u64) -> TestResult {
            let bits = 1.max(64 - v.leading_zeros() as usize);
            let bytes = (bits + 6) / 7;
            let res = u64::encode(v);
            should_equal(res.as_ref().len(), bytes)
        }
        fn size_u128(v: u128) -> TestResult {
            let bits = 1.max(128 - v.leading_zeros() as usize);
            let bytes = (bits + 6) / 7;
            let res = u128::encode(v);
            should_equal(res.as_ref().len(), bytes)
        }

        fn roundtrip_u8(v: u8) -> TestResult {
            should_equal(u8::decode(u8::encode(v).as_ref()), Some(v))
        }
        fn roundtrip_u16(v: u16) -> TestResult {
            should_equal(u16::decode(u16::encode(v).as_ref()), Some(v))
        }
        fn roundtrip_u32(v: u32) -> TestResult {
            should_equal(u32::decode(u32::encode(v).as_ref()), Some(v))
        }
        fn roundtrip_u64(v: u64) -> TestResult {
            should_equal(u64::decode(u64::encode(v).as_ref()), Some(v))
        }
        fn roundtrip_u128(v: u128) -> TestResult {
            should_equal(u128::decode(u128::encode(v).as_ref()), Some(v))
        }
    }
}
