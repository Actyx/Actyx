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
                    v = v.checked_add((*b as $id) << bits)?;
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
    use super::*;

    #[test]
    fn test() {
        assert_eq!(u32::encode(1234).as_ref(), &[210, 9]);
        assert_eq!(u32::decode(&[128, 0]), None); // not minimal
        assert_eq!(u32::decode(&[128, 1, 0]), None); // trailing garbage
    }

    quickcheck::quickcheck! {
        fn size_u8(v: u8) -> bool {
            let bits = 1.max(8 - v.leading_zeros() as usize);
            let bytes = (bits + 6) / 7;
            let res = u8::encode(v);
            res.as_ref().len() == bytes
        }
        fn size_u16(v: u16) -> bool {
            let bits = 1.max(16 - v.leading_zeros() as usize);
            let bytes = (bits + 6) / 7;
            let res = u16::encode(v);
            res.as_ref().len() == bytes
        }
        fn size_u32(v: u32) -> bool {
            let bits = 1.max(32 - v.leading_zeros() as usize);
            let bytes = (bits + 6) / 7;
            let res = u32::encode(v);
            res.as_ref().len() == bytes
        }
        fn size_u64(v: u64) -> bool {
            let bits = 1.max(64 - v.leading_zeros() as usize);
            let bytes = (bits + 6) / 7;
            let res = u64::encode(v);
            res.as_ref().len() == bytes
        }
        fn size_u128(v: u128) -> bool {
            let bits = 1.max(128 - v.leading_zeros() as usize);
            let bytes = (bits + 6) / 7;
            let res = u128::encode(v);
            res.as_ref().len() == bytes
        }

        fn roundtrip_u8(v: u8) -> bool {
            u8::decode(u8::encode(v).as_ref()) == Some(v)
        }
        fn roundtrip_u16(v: u16) -> bool {
            u16::decode(u16::encode(v).as_ref()) == Some(v)
        }
        fn roundtrip_u32(v: u32) -> bool {
            u32::decode(u32::encode(v).as_ref()) == Some(v)
        }
        fn roundtrip_u64(v: u64) -> bool {
            u64::decode(u64::encode(v).as_ref()) == Some(v)
        }
        fn roundtrip_u128(v: u128) -> bool {
            u128::decode(u128::encode(v).as_ref()) == Some(v)
        }
    }
}
