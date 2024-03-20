use std::{borrow::Cow, sync::Arc};

use anyhow::Context;
use ax_aql::{Label, NonEmptyVec, Type, TypeAtom};
use cbor_data::{value::Number, CborBuilder, CborValue, Encoder};

// NOTE: check tarpaulin for test coverage, its always hard to ensure every path was checked and
// in this case, it would be really useful: https://github.com/xd009642/tarpaulin

/// Check if a CBOR value is null.
fn validate_null(value: &CborValue) -> anyhow::Result<()> {
    if value.is_null() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be null"))
    }
}

/// Check if a CBOR value is a boolean or a boolean refinement (i.e. `true` or `false`).
fn validate_bool(value: &CborValue, bool_refinement: &Option<bool>) -> anyhow::Result<()> {
    if let Some(value) = value.as_bool() {
        if let Some(bool_refinement) = bool_refinement {
            if value == *bool_refinement {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "type mismatch, expected value to be {}, received {} instead",
                    bool_refinement,
                    value
                ))
            }
        } else {
            Ok(())
        }
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be a boolean"))
    }
}

/// Check if a CBOR value is an integer or an integer refinement (e.g. `10`).
fn validate_number(value: &CborValue, number_refinement: &Option<u64>) -> anyhow::Result<()> {
    if let Some(Number::Int(value)) = value.as_number() {
        if let Ok(value) = u64::try_from(*value) {
            if let Some(number_refinement) = number_refinement {
                if value == *number_refinement {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "type mismatch, expected value to be {}, received {} instead",
                        number_refinement,
                        value
                    ))
                }
            } else {
                Ok(())
            }
        } else {
            Err(anyhow::anyhow!(
                "type mismatch, expected value to be a 64-bit integer but received a 128-bit integer instead"
            ))
        }
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be an integer"))
    }
}

/// Check if a CBOR value is a timestamp.
fn validate_timestamp(value: &CborValue) -> anyhow::Result<()> {
    if value.as_timestamp().is_some() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be a timestamp"))
    }
}

/// Check if a CBOR value is a string or a string refinement (e.g. "Hello").
fn validate_string(value: &CborValue, string_refinement: &Option<String>) -> anyhow::Result<()> {
    if let Some(value) = value.as_str() {
        if let Some(refinement) = string_refinement {
            if value == refinement {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "type mismatch, expected value to be {}, received {} instead",
                    refinement,
                    value
                ))
            }
        } else {
            Ok(())
        }
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be a string"))
    }
}

/// Check if a CBOR value is an array. Can also be used to check for tuples (following RFC 7049).
fn validate_array(value: &CborValue, ty: &Type) -> anyhow::Result<()> {
    if let Some(values) = value.as_array() {
        for (i, value) in values.iter().enumerate() {
            if let Err(err) = validate(&value.decode(), ty) {
                return Err(anyhow::anyhow!("type mismatch, element {} is not of type {:?}", i, ty)).context(err);
            }
        }
        Ok(())
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be an array"))
    }
}

fn validate_tuple(value: &CborValue, ty: &[Type]) -> anyhow::Result<()> {
    if let Some(array) = value.as_array() {
        if array.len() != ty.len() {
            return Err(anyhow::anyhow!(
                "type mismatch, expected tuple to have length {} got {} instead",
                ty.len(),
                array.len()
            ));
        }
        for (i, (value, ty)) in array.iter().zip(ty.iter()).enumerate() {
            if let Err(err) = validate(&value.decode(), ty) {
                return Err(anyhow::anyhow!(
                    "type mismatch, expected element {} of the tuple to be {:?}",
                    i,
                    ty
                ))
                .context(err);
            }
        }
        Ok(())
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be an array"))
    }
}

/// Check if a CBOR value is a dictionary.
fn validate_dict(value: &CborValue, ty: &Type) -> anyhow::Result<()> {
    let key_type = Type::Union(Arc::new((
        Type::Atom(TypeAtom::String(None)),
        Type::Atom(TypeAtom::Number(None)),
    )));
    if let Some(dict) = value.as_dict() {
        for (k, v) in dict {
            let decoded_key = k.decode();
            if let Err(err) = validate(&decoded_key, &key_type) {
                return Err(anyhow::anyhow!(
                    "type mismatch, dict keys can only be numbers or strings"
                ))
                .context(err);
            }
            if let Err(err) = validate(&v.decode(), ty) {
                return Err(anyhow::anyhow!(
                    "type mismatch, dict value for key {:?} is not {:?}",
                    decoded_key,
                    ty
                ))
                .context(err);
            }
        }
        Ok(())
    } else {
        Err(anyhow::anyhow!("type mismatch, expected value to be a dictionary"))
    }
}

// The idea is that this function will only check for Atoms,
// the issue is that a Record can nest arbitrarily deep and may use non-atom types
// ideally, I would like to _not_ use recursion since we already had the stack issue in other places;
// so we either ignore records here and handle them somewhere else or we handle them here, some how
fn validate_atom(value: &CborValue, ty: &TypeAtom) -> anyhow::Result<()> {
    match ty {
        TypeAtom::Null => validate_null(value),
        TypeAtom::Bool(refinement) => validate_bool(value, refinement),
        TypeAtom::Number(refinement) => validate_number(value, refinement),
        TypeAtom::Timestamp => validate_timestamp(value),
        TypeAtom::String(refinement) => validate_string(value, refinement),
        TypeAtom::Universal => Ok(()),
    }
}

fn validate_record(value: &CborValue, ty: &NonEmptyVec<(Label, Type)>) -> anyhow::Result<()> {
    if let Some(value) = value.as_dict() {
        for (label, ty) in ty.iter() {
            match label {
                Label::String(string) => {
                    let cbor = CborBuilder::new().encode_str(string);
                    let cbor = Cow::Owned(cbor);
                    if let Some(value) = value.get(&cbor) {
                        validate(&value.decode(), ty)?;
                    } else {
                        return Err(anyhow::anyhow!("label {} does not exist in record", string));
                    }
                }
                Label::Number(number) => {
                    let number = i128::from(*number);
                    let number = Number::Int(number);
                    let cbor = CborBuilder::new().encode_number(&number);
                    let cbor = Cow::Owned(cbor);
                    if let Some(value) = value.get(&cbor) {
                        validate(&value.decode(), ty)?;
                    } else {
                        return Err(anyhow::anyhow!("label {:?} does not exist in record", number));
                    }
                }
            }
        }
        Ok(())
    } else {
        Err(anyhow::anyhow!("expected a record (dict)"))
    }
}

fn validate(value: &CborValue, ty: &Type) -> anyhow::Result<()> {
    match ty {
        Type::Atom(atom) => validate_atom(value, atom),
        Type::Array(inner_ty) => validate_array(value, inner_ty),
        Type::Dict(inner_ty) => validate_dict(value, inner_ty),
        Type::Tuple(tuple) => validate_tuple(value, tuple),
        Type::Record(record) => validate_record(value, record),
        // We can make this much more efficient by checking more things beforehand
        // like intersecting types before decoding anything
        Type::Union(union) => validate(value, &union.0).or_else(|err| validate(value, &union.1).context(err)),
        Type::Intersection(intersection) => {
            validate(value, &intersection.0).and_then(|_| validate(value, &intersection.1))
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use super::{
        validate, validate_array, validate_atom, validate_bool, validate_dict, validate_null, validate_number,
        validate_record, validate_string, validate_timestamp, validate_tuple,
    };

    use ax_aql::{Label, Type, TypeAtom};
    use cbor_data::{
        value::{Precision, Timestamp},
        CborBuilder, Encoder,
    };

    #[test]
    fn test_validate_null() {
        let cbor = CborBuilder::new().encode_null();
        let cbor = cbor.decode();
        validate_null(&cbor).unwrap();
    }

    #[test]
    fn test_validate_null_fail() {
        let cbor = CborBuilder::new().encode_i64(100);
        let cbor = cbor.decode();
        validate_null(&cbor).unwrap_err();
    }

    #[test]
    fn test_validate_bool() {
        let cbor = CborBuilder::new().encode_bool(false);
        let cbor = cbor.decode();
        validate_bool(&cbor, &None).unwrap();

        validate_bool(&cbor, &Some(false)).unwrap();
        validate_bool(&cbor, &Some(true)).unwrap_err();
    }

    #[test]
    fn test_validate_bool_fail() {
        let cbor = CborBuilder::new().encode_i64(10);
        let cbor = cbor.decode();
        validate_bool(&cbor, &None).unwrap_err();

        validate_bool(&cbor, &Some(false)).unwrap_err();
    }

    #[test]
    fn test_validate_number() {
        let cbor = CborBuilder::new().encode_i64(10);
        let cbor = cbor.decode();
        validate_number(&cbor, &None).unwrap();
        validate_number(&cbor, &Some(10)).unwrap();
        validate_number(&cbor, &Some(50)).unwrap_err();
    }

    #[test]
    fn test_validate_number_fail() {
        let cbor = CborBuilder::new().encode_f64(0.100);
        let cbor = cbor.decode();
        validate_number(&cbor, &None).unwrap_err();

        validate_number(&cbor, &Some(100)).unwrap_err();
    }

    #[test]
    fn test_validate_timestamp() {
        let timestamp = Timestamp::new(876523558, 0, 0);
        let cbor = CborBuilder::new().encode_timestamp(timestamp, Precision::Seconds);
        let cbor = cbor.decode();
        validate_timestamp(&cbor).unwrap();
    }

    #[test]
    fn test_validate_timestamp_fail() {
        let cbor = CborBuilder::new().encode_str("value");
        let cbor = cbor.decode();
        validate_timestamp(&cbor).unwrap_err();
    }

    #[test]
    fn test_validate_string() {
        let cbor = CborBuilder::new().encode_str("Olá mundo!");
        let cbor = cbor.decode();
        validate_string(&cbor, &None).unwrap();
        validate_string(&cbor, &Some("Olá mundo!".to_string())).unwrap();
        validate_string(&cbor, &Some("Adeus mundo!".to_string())).unwrap_err();
    }

    #[test]
    fn test_validate_string_fail() {
        let cbor = CborBuilder::new().encode_i64(64);
        let cbor = cbor.decode();
        validate_string(&cbor, &None).unwrap_err();
        validate_string(&cbor, &Some("Adeus mundo!".to_string())).unwrap_err();
    }

    #[test]
    fn test_validate_universal() {
        let cbor = CborBuilder::new().encode_null();
        let cbor = cbor.decode();
        validate_atom(&cbor, &TypeAtom::Universal).unwrap();

        let cbor = CborBuilder::new().encode_bool(true);
        let cbor = cbor.decode();
        validate_atom(&cbor, &TypeAtom::Universal).unwrap();

        let cbor = CborBuilder::new().encode_i64(100);
        let cbor = cbor.decode();
        validate_atom(&cbor, &TypeAtom::Universal).unwrap();

        let timestamp = Timestamp::new(876523558, 0, 0);
        let cbor = CborBuilder::new().encode_timestamp(timestamp, Precision::Seconds);
        let cbor = cbor.decode();
        validate_atom(&cbor, &TypeAtom::Universal).unwrap();

        let cbor = CborBuilder::new().encode_str("Hello!");
        let cbor = cbor.decode();
        validate_atom(&cbor, &TypeAtom::Universal).unwrap();
    }

    #[test]
    fn test_validate_array() {
        let cbor = CborBuilder::new().encode_array(|builder| {
            builder.encode_u64(10).encode_u64(100);
        });
        let cbor = cbor.decode();

        validate_array(&cbor, &Type::Atom(TypeAtom::Number(None))).unwrap();
    }

    #[test]
    fn test_validate_array_fail_elements() {
        let cbor = CborBuilder::new().encode_array(|builder| {
            builder.encode_str("hello").encode_str("world");
        });
        let cbor = cbor.decode();

        validate_array(&cbor, &Type::Atom(TypeAtom::Number(None))).unwrap_err();
    }

    #[test]
    fn test_validate_array_fail_type() {
        let cbor = CborBuilder::new().encode_dict(|builder| {
            builder.with_key("hello", |b| b.encode_str("world"));
        });
        let cbor = cbor.decode();

        validate_array(&cbor, &Type::Atom(TypeAtom::Number(None))).unwrap_err();
    }

    #[test]
    fn test_validate_tuple() {
        let cbor = CborBuilder::new().encode_array(|builder| {
            builder.encode_u64(10).encode_u64(100);
        });
        let cbor = cbor.decode();

        validate_tuple(
            &cbor,
            &[
                Type::Atom(TypeAtom::Number(None)),
                Type::Atom(TypeAtom::Number(Some(100))),
            ],
        )
        .unwrap();
    }

    #[test]
    fn test_validate_tuple_fail() {
        let cbor = CborBuilder::new().encode_dict(|builder| {
            builder.with_key("hello", |b| b.encode_str("world"));
        });
        let cbor = cbor.decode();

        validate_tuple(
            &cbor,
            &[
                Type::Atom(TypeAtom::Number(None)),
                Type::Atom(TypeAtom::Number(None)),
                Type::Atom(TypeAtom::Number(None)),
            ],
        )
        .unwrap_err();
    }

    #[test]
    fn test_validate_tuple_length_fail() {
        let cbor = CborBuilder::new().encode_array(|builder| {
            builder.encode_u64(10).encode_u64(100);
        });
        let cbor = cbor.decode();

        validate_tuple(
            &cbor,
            &[
                Type::Atom(TypeAtom::Number(Some(10))),
                Type::Atom(TypeAtom::Number(Some(100))),
                Type::Atom(TypeAtom::Number(None)),
            ],
        )
        .unwrap_err();
    }

    #[test]
    fn test_validate_tuple_type_fail() {
        let cbor = CborBuilder::new().encode_array(|builder| {
            builder.encode_u64(10).encode_u64(100);
        });
        let cbor = cbor.decode();

        validate_tuple(
            &cbor,
            &[Type::Atom(TypeAtom::Number(None)), Type::Atom(TypeAtom::String(None))],
        )
        .unwrap_err();
    }

    #[test]
    fn test_validate_dict() {
        let cbor = CborBuilder::new().encode_dict(|builder| {
            builder.with_key("hello", |b| b.encode_str("world"));
        });
        let cbor = cbor.decode();

        validate_dict(&cbor, &Type::Atom(TypeAtom::String(None))).unwrap();
    }

    #[test]
    fn test_validate_dict_fail() {
        let cbor = CborBuilder::new().encode_array(|builder| {
            builder.encode_u64(10).encode_u64(100);
        });
        let cbor = cbor.decode();

        validate_dict(&cbor, &Type::Atom(TypeAtom::String(None))).unwrap_err();
    }

    #[test]
    fn test_validate_dict_key_fail() {
        let cbor = CborBuilder::new().encode_dict(|builder| {
            builder.with_cbor_key(|b| b.encode_null(), |b| b.encode_str("world"));
        });
        let cbor = cbor.decode();

        validate_dict(&cbor, &Type::Atom(TypeAtom::String(None))).unwrap_err();
    }

    #[test]
    fn test_validate_dict_value_fail() {
        let cbor = CborBuilder::new().encode_dict(|builder| {
            builder.with_key("hello", |b| b.encode_u64(1997));
        });
        let cbor = cbor.decode();

        validate_dict(&cbor, &Type::Atom(TypeAtom::String(None))).unwrap_err();
    }

    #[test]
    fn test_validate_record() {
        // { "temperature": 100, { "coordinates": { "x": 10, "y": -10 } } }
        let cbor = CborBuilder::new().encode_dict(|builder| {
            builder.with_key("temperature", |builder| builder.encode_i64(100));
            builder.with_key("coordinates", |builder| {
                builder.encode_dict(|builder| {
                    builder.with_key("x", |builder| builder.encode_i64(10));
                    builder.with_key("y", |builder| builder.encode_i64(10));
                })
            });
        });
        let cbor = cbor.decode();
        let ty = vec![
            (
                Label::String("temperature".to_string().try_into().expect("non-empty string")),
                Type::Atom(TypeAtom::Number(None)),
            ),
            (
                Label::String("coordinates".to_string().try_into().expect("non-empty string")),
                Type::Record(
                    vec![
                        (
                            Label::String("x".to_string().try_into().expect("non-empty string")),
                            Type::Atom(TypeAtom::Number(Some(10))),
                        ),
                        (
                            Label::String("y".to_string().try_into().expect("non-empty string")),
                            Type::Atom(TypeAtom::Number(None)),
                        ),
                    ]
                    .try_into()
                    .expect("proper type"),
                ),
            ),
        ]
        .try_into()
        .expect("proper type");

        validate_record(&cbor, &ty).unwrap();
    }

    #[test]
    fn test_validate_record_number_key() {
        // { 10: "value"}
        let cbor = CborBuilder::new().encode_dict(|builder| {
            builder.with_cbor_key(|b| b.encode_i64(10), |b| b.encode_str("value"));
        });
        let cbor = cbor.decode();
        let ty = vec![(
            Label::Number(10),
            Type::Atom(TypeAtom::String(Some("value".to_string()))),
        )]
        .try_into()
        .expect("proper type");

        validate_record(&cbor, &ty).unwrap();
    }

    #[test]
    fn test_validate_union_simple() {
        let cbor = CborBuilder::new().encode_null();
        let cbor = cbor.decode();
        let left = Type::Atom(TypeAtom::Null);
        let right = Type::Atom(TypeAtom::Timestamp);

        validate(&cbor, &Type::Union(Arc::new((left.clone(), right.clone())))).unwrap();
        validate(&cbor, &Type::Union(Arc::new((right.clone(), left.clone())))).unwrap();
    }

    #[test]
    fn test_validate_union_nested_bi() {
        let left = {
            let left = Type::Atom(TypeAtom::Null);
            let right = Type::Atom(TypeAtom::Timestamp);
            Arc::new((left, right))
        };

        let right = {
            let left = Type::Atom(TypeAtom::Bool(Some(false)));
            let right = Type::Atom(TypeAtom::String(Some("Olá!".to_string())));
            Arc::new((left, right))
        };

        let cbor = CborBuilder::new().encode_str("Olá!");
        let cbor = cbor.decode();

        validate(&cbor, &Type::Union(left.clone())).unwrap_err();
        validate(&cbor, &Type::Union(right.clone())).unwrap();
        validate(&cbor, &Type::Union(Arc::new((Type::Union(left), Type::Union(right))))).unwrap();
    }

    #[test]
    fn test_validate_union_nested_leaning() {
        let cbor = CborBuilder::new().encode_null();
        let cbor = cbor.decode();

        let first_union = {
            let left = Type::Atom(TypeAtom::Null);
            let right = Type::Atom(TypeAtom::Timestamp);
            Type::Union(Arc::new((left, right)))
        };
        validate(&cbor, &first_union).unwrap();

        let second_union = {
            let left = Type::Atom(TypeAtom::Bool(Some(false)));
            Type::Union(Arc::new((left, first_union)))
        };
        validate(&cbor, &second_union).unwrap();

        let third_union = {
            let left = Type::Atom(TypeAtom::String(Some("Olá".to_string())));
            Type::Union(Arc::new((left, second_union)))
        };
        validate(&cbor, &third_union).unwrap();
    }

    #[test]
    fn test_validate_union_nested_leaning_fail() {
        let cbor = CborBuilder::new().encode_null();
        let cbor = cbor.decode();

        let first_union = {
            let left = Type::Atom(TypeAtom::Number(Some(10)));
            let right = Type::Atom(TypeAtom::Timestamp);
            Type::Union(Arc::new((left, right)))
        };
        validate(&cbor, &first_union).unwrap_err();

        let second_union = {
            let left = Type::Atom(TypeAtom::Bool(Some(false)));
            Type::Union(Arc::new((left, first_union)))
        };
        validate(&cbor, &second_union).unwrap_err();

        let third_union = {
            let left = Type::Atom(TypeAtom::String(Some("Olá".to_string())));
            Type::Union(Arc::new((left, second_union)))
        };
        validate(&cbor, &third_union).unwrap_err();
    }

    #[test]
    fn test_validate_intersection() {
        let cbor = CborBuilder::new().encode_i64(10);
        let cbor = cbor.decode();

        let left = Type::Atom(TypeAtom::Number(None));
        let right = Type::Atom(TypeAtom::Number(Some(10)));
        let intersection = Type::Intersection(Arc::new((left, right)));
        validate(&cbor, &intersection).unwrap();
    }

    #[test]
    fn test_validate_intersection_fail() {
        let cbor = CborBuilder::new().encode_i64(10);
        let cbor = cbor.decode();

        let left = Type::Atom(TypeAtom::Null);
        let right = Type::Atom(TypeAtom::Timestamp);
        let intersection = Type::Intersection(Arc::new((left, right)));
        validate(&cbor, &intersection).unwrap_err();
    }

    #[test]
    fn test_validate_intersection_nested() {
        let cbor = CborBuilder::new().encode_i64(10);
        let cbor = cbor.decode();

        let first_intersection = {
            let left = Type::Atom(TypeAtom::Number(Some(10)));
            let right = Type::Atom(TypeAtom::Number(None));
            Type::Intersection(Arc::new((left, right)))
        };
        validate(&cbor, &first_intersection).unwrap();

        let second_intersection = {
            let left = Type::Atom(TypeAtom::Universal);
            Type::Intersection(Arc::new((left, first_intersection)))
        };
        validate(&cbor, &second_intersection).unwrap();
    }

    #[test]
    fn test_validate_intersection_nested_fail() {
        let cbor = CborBuilder::new().encode_i64(10);
        let cbor = cbor.decode();

        let first_intersection = {
            let left = Type::Atom(TypeAtom::Number(Some(10)));
            let right = Type::Atom(TypeAtom::Number(None));
            Type::Intersection(Arc::new((left, right)))
        };
        validate(&cbor, &first_intersection).unwrap();

        let second_intersection = {
            let left = Type::Atom(TypeAtom::Universal);
            Type::Intersection(Arc::new((left, first_intersection)))
        };
        validate(&cbor, &second_intersection).unwrap();

        let third_intersection = {
            let left = Type::Atom(TypeAtom::Null);
            Type::Intersection(Arc::new((left, second_intersection)))
        };
        validate(&cbor, &third_intersection).unwrap_err();
    }

    #[test]
    fn test_intersection_of_unions() {
        let left_union = {
            let left = Type::Atom(TypeAtom::Number(Some(10)));
            let right = Type::Atom(TypeAtom::Null);
            Type::Union(Arc::new((left, right)))
        };

        let right_union = {
            let left = Type::Atom(TypeAtom::Number(None));
            let right = Type::Atom(TypeAtom::Bool(None));
            Type::Union(Arc::new((left, right)))
        };

        let intersection = Type::Intersection(Arc::new((left_union, right_union)));

        let cbor = CborBuilder::new().encode_i64(10);
        let cbor = cbor.decode();
        validate(&cbor, &intersection).unwrap();

        let cbor = CborBuilder::new().encode_null();
        let cbor = cbor.decode();
        validate(&cbor, &intersection).unwrap_err();

        let cbor = CborBuilder::new().encode_bool(false);
        let cbor = cbor.decode();
        validate(&cbor, &intersection).unwrap_err();
    }

    #[test]
    fn test_union_of_intersections() {
        let left_intersection = {
            let left = Type::Atom(TypeAtom::Number(Some(10)));
            let right = Type::Atom(TypeAtom::Number(None));
            Type::Intersection(Arc::new((left, right)))
        };

        let right_intersection = {
            let left = Type::Atom(TypeAtom::Null);
            let right = Type::Atom(TypeAtom::Bool(None));
            Type::Intersection(Arc::new((left, right)))
        };

        let union = Type::Union(Arc::new((left_intersection, right_intersection)));

        let cbor = CborBuilder::new().encode_i64(10);
        let cbor = cbor.decode();
        validate(&cbor, &union).unwrap();

        let cbor = CborBuilder::new().encode_i64(101);
        let cbor = cbor.decode();
        validate(&cbor, &union).unwrap_err();

        let cbor = CborBuilder::new().encode_null();
        let cbor = cbor.decode();
        validate(&cbor, &union).unwrap_err();

        let cbor = CborBuilder::new().encode_bool(false);
        let cbor = cbor.decode();
        validate(&cbor, &union).unwrap_err();
    }
}
