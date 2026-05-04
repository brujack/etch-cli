use std::{
    cmp::Ordering,
    ffi::OsString,
    fmt::{Debug, Display},
    path::PathBuf,
};

use serde_json::Value as JsonValue;

use serde::{
    de::{Error as SError, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};

#[derive(Clone, PartialEq, PartialOrd)]
pub enum Value {
    Null,
    String(String),
    Number(Number),
    List(Vec<Value>),
}

#[derive(Clone, PartialEq, PartialOrd)]
pub struct Number {
    inner: NumberVariant,
}

#[derive(Clone, Copy)]
enum NumberVariant {
    Unsigned(u64),
    Signed(i64),
    Float(f64),
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Number(n) => n.serialize(serializer),
            Value::String(s) => serializer.serialize_str(s),
            Value::List(seq) => seq.serialize(serializer),
        }
    }
}

impl Serialize for Number {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        {
            match self.inner {
                NumberVariant::Unsigned(u) => serializer.serialize_u64(u),
                NumberVariant::Signed(s) => serializer.serialize_i64(s),
                NumberVariant::Float(f) => serializer.serialize_f64(f),
            }
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("any etch context value")
            }

            fn visit_i64<E>(self, i: i64) -> Result<Value, E>
            where
                E: SError,
            {
                Ok(Value::Number(Number {
                    inner: NumberVariant::Signed(i),
                }))
            }

            fn visit_u64<E>(self, u: u64) -> Result<Value, E>
            where
                E: SError,
            {
                Ok(Value::Number(Number {
                    inner: NumberVariant::Unsigned(u),
                }))
            }

            fn visit_f64<E>(self, f: f64) -> Result<Value, E>
            where
                E: SError,
            {
                Ok(Value::Number(Number {
                    inner: NumberVariant::Float(f),
                }))
            }

            fn visit_str<E>(self, s: &str) -> Result<Value, E>
            where
                E: SError,
            {
                Ok(Value::String(s.to_owned()))
            }

            fn visit_string<E>(self, s: String) -> Result<Value, E>
            where
                E: SError,
            {
                Ok(Value::String(s))
            }

            fn visit_unit<E>(self) -> Result<Value, E>
            where
                E: SError,
            {
                Ok(Value::Null)
            }

            fn visit_none<E>(self) -> Result<Value, E>
            where
                E: SError,
            {
                Ok(Value::Null)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }

            fn visit_seq<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let mut vec = Vec::new();

                while let Some(element) = visitor.next_element()? {
                    vec.push(element);
                }

                Ok(Value::List(vec))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

impl Debug for Value {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => formatter.write_str("Null"),
            Value::String(string) => write!(formatter, "String({string:?})"),
            Value::Number(number) => write!(formatter, "Number({number})"),
            Value::List(list) => {
                formatter.write_str("List ")?;
                formatter.debug_list().entries(list).finish()
            }
        }
    }
}

impl Debug for Number {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Number({self})")
    }
}

impl Display for Number {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner {
            NumberVariant::Unsigned(number) => Display::fmt(&number, formatter),
            NumberVariant::Signed(number) => Display::fmt(&number, formatter),
            NumberVariant::Float(number) => Display::fmt(&number, formatter),
        }
    }
}

impl NumberVariant {
    fn total_cmp(&self, other: &Self) -> Ordering {
        match (*self, *other) {
            (NumberVariant::Unsigned(a), NumberVariant::Unsigned(b)) => a.cmp(&b),
            (NumberVariant::Signed(a), NumberVariant::Signed(b)) => a.cmp(&b),
            (NumberVariant::Unsigned(a), NumberVariant::Signed(b)) => (a as i64).cmp(&b),
            (NumberVariant::Signed(a), NumberVariant::Unsigned(b)) => a.cmp(&(b as i64)),
            (NumberVariant::Float(a), NumberVariant::Float(b)) => a.total_cmp(&b),
            (NumberVariant::Signed(a), NumberVariant::Float(b)) => (a as f64).total_cmp(&b),
            (NumberVariant::Unsigned(a), NumberVariant::Float(b)) => (a as f64).total_cmp(&b),
            (NumberVariant::Float(a), NumberVariant::Signed(b)) => a.total_cmp(&(b as f64)),
            (NumberVariant::Float(a), NumberVariant::Unsigned(b)) => a.total_cmp(&(b as f64)),
        }
    }
}

impl PartialEq for NumberVariant {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (NumberVariant::Unsigned(a), NumberVariant::Unsigned(b)) => a == b,
            (NumberVariant::Signed(a), NumberVariant::Signed(b)) => a == b,
            (NumberVariant::Float(a), NumberVariant::Float(b)) => a == b,
            (NumberVariant::Unsigned(a), NumberVariant::Signed(b)) => (a as i64) == b,
            (NumberVariant::Signed(a), NumberVariant::Unsigned(b)) => a == (b as i64),
            (NumberVariant::Unsigned(a), NumberVariant::Float(b)) => (a as f64) == b,
            (NumberVariant::Signed(a), NumberVariant::Float(b)) => (a as f64) == b,
            (NumberVariant::Float(a), NumberVariant::Unsigned(b)) => a == (b as f64),
            (NumberVariant::Float(a), NumberVariant::Signed(b)) => a == (b as f64),
        }
    }
}

impl PartialOrd for NumberVariant {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.total_cmp(other))
    }
}

impl From<String> for Value {
    fn from(from: String) -> Self {
        Value::String(from)
    }
}

impl<'a> From<&'a str> for Value {
    fn from(from: &'a str) -> Self {
        Value::String(from.to_string())
    }
}

impl<'a> From<std::borrow::Cow<'a, str>> for Value {
    fn from(from: std::borrow::Cow<'a, str>) -> Self {
        Value::String(from.to_string())
    }
}

impl From<OsString> for Value {
    fn from(from: OsString) -> Self {
        Value::String(from.to_str().unwrap_or("unknown").to_string())
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Number(Number {
            inner: NumberVariant::Signed(value),
        })
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Self::Number(Number {
            inner: NumberVariant::Unsigned(value),
        })
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(Number {
            inner: NumberVariant::Float(value),
        })
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Number(Number {
            inner: NumberVariant::Unsigned(value as u64),
        })
    }
}

impl From<PathBuf> for Value {
    fn from(from: PathBuf) -> Self {
        Value::String(from.display().to_string())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(from: Vec<T>) -> Self {
        Value::List(from.into_iter().map(Into::into).collect())
    }
}

impl TryFrom<JsonValue> for Value {
    type Error = anyhow::Error;

    fn try_from(from: JsonValue) -> Result<Self, Self::Error> {
        let value = match from {
            JsonValue::Null => Self::Null,
            JsonValue::Bool(b) => b.into(),
            JsonValue::Number(number) => {
                if number.is_u64() {
                    number.as_u64().expect("is_u64 guarantees Some").into()
                } else if number.is_i64() {
                    number.as_i64().expect("is_i64 guarantees Some").into()
                } else {
                    number
                        .as_f64()
                        .expect("as_f64 is Some for all finite JSON numbers")
                        .into()
                }
            }
            JsonValue::String(s) => Self::String(s),
            JsonValue::Array(a) => Self::List(
                a.into_iter()
                    .map(TryInto::try_into)
                    .filter_map(Result::ok)
                    .collect(),
            ),
            JsonValue::Object(o) => Self::List(
                o.values()
                    .cloned()
                    .map(TryInto::try_into)
                    .filter_map(Result::ok)
                    .collect(),
            ),
        };

        Ok(value)
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Value::Null => "null".to_string(),
                Value::String(string) => string.to_owned(),
                Value::Number(number) => number.to_string(),
                Value::List(list) => list
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<String>>()
                    .join(","),
            }
        )
    }
}

#[cfg(test)]
mod test {
    use std::{borrow::Cow, ffi::OsString, path::PathBuf};

    use crate::values::{Number, NumberVariant, Value};
    use anyhow::Ok;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn from_string_tests() -> anyhow::Result<()> {
        assert_eq!(
            Value::from("John Sheppard"),
            Value::String("John Sheppard".to_string())
        );

        assert_eq!(
            Value::from("Elizabeth Weir".to_string()),
            Value::String("Elizabeth Weir".to_string())
        );

        assert_eq!(Value::from(PathBuf::new()), Value::String("".to_string()));

        assert_eq!(
            Value::from(Cow::from("Samantha Carter")),
            Value::String("Samantha Carter".to_string())
        );

        assert_eq!(
            Value::from(OsString::from("Jennifer Keller")),
            Value::String("Jennifer Keller".to_string())
        );

        Ok(())
    }

    #[test]
    fn from_vec_test() -> anyhow::Result<()> {
        assert_eq!(
            Value::from(vec!["Aiden Ford", "Rodney McKay", "Ronon Dex"]),
            Value::List(vec![
                Value::String("Aiden Ford".to_string()),
                Value::String("Rodney McKay".to_string()),
                Value::String("Ronon Dex".to_string())
            ])
        );

        Ok(())
    }

    #[test]
    fn number_compare_test() -> anyhow::Result<()> {
        // unsigned
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            }) == Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(3)
            }) > Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            }) < Value::Number(Number {
                inner: NumberVariant::Unsigned(3)
            })
        );

        // signed
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(2)
            }) == Value::Number(Number {
                inner: NumberVariant::Signed(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(3)
            }) > Value::Number(Number {
                inner: NumberVariant::Signed(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(2)
            }) < Value::Number(Number {
                inner: NumberVariant::Signed(3)
            })
        );

        // float
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            }) == Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Float(3.0)
            }) > Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            }) < Value::Number(Number {
                inner: NumberVariant::Float(3.0)
            })
        );

        // unsigned with signed
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            }) == Value::Number(Number {
                inner: NumberVariant::Signed(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(3)
            }) > Value::Number(Number {
                inner: NumberVariant::Signed(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            }) < Value::Number(Number {
                inner: NumberVariant::Signed(3)
            })
        );

        // signed with unsigned
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(2)
            }) == Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(3)
            }) > Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(2)
            }) < Value::Number(Number {
                inner: NumberVariant::Unsigned(3)
            })
        );

        // signed with float
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(2)
            }) == Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(3)
            }) > Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Signed(2)
            }) < Value::Number(Number {
                inner: NumberVariant::Float(3.0)
            })
        );

        // unsigned with float
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            }) == Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(3)
            }) > Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            }) < Value::Number(Number {
                inner: NumberVariant::Float(3.0)
            })
        );

        Ok(())
    }

    #[test]
    fn debug_tests() -> anyhow::Result<()> {
        assert_eq!(format!("{:?}", Value::Null), "Null".to_string());

        assert_eq!(
            format!("{:?}", Value::String("Richard Woolsey".to_string())),
            "String(\"Richard Woolsey\")".to_string()
        );

        assert_eq!(
            format!(
                "{:?}",
                Value::List(vec![
                    Value::String("Aiden Ford".to_string()),
                    Value::String("Rodney McKay".to_string()),
                    Value::String("Ronon Dex".to_string())
                ])
            ),
            "List [String(\"Aiden Ford\"), String(\"Rodney McKay\"), String(\"Ronon Dex\")]"
                .to_string()
        );

        assert_eq!(
            format!(
                "{:?}",
                Value::Number(Number {
                    inner: NumberVariant::Unsigned(2)
                })
            ),
            "Number(2)".to_string()
        );

        assert_eq!(
            format!(
                "{:?}",
                Value::Number(Number {
                    inner: NumberVariant::Signed(2)
                })
            ),
            "Number(2)".to_string()
        );

        assert_eq!(
            format!(
                "{:?}",
                Value::Number(Number {
                    inner: NumberVariant::Float(2.0)
                })
            ),
            "Number(2)".to_string()
        );

        Ok(())
    }

    #[test]
    fn display_tests() -> anyhow::Result<()> {
        assert_eq!(format!("{}", Value::Null), "null".to_string());

        assert_eq!(
            format!("{}", Value::String("Carson Beckett".to_string())),
            "Carson Beckett".to_string()
        );

        assert_eq!(
            format!(
                "{}",
                Value::Number(Number {
                    inner: NumberVariant::Unsigned(42)
                })
            ),
            "42".to_string()
        );

        assert_eq!(
            format!(
                "{}",
                Value::Number(Number {
                    inner: NumberVariant::Signed(-7)
                })
            ),
            "-7".to_string()
        );

        assert_eq!(
            format!(
                "{}",
                Value::Number(Number {
                    inner: NumberVariant::Float(2.71)
                })
            ),
            "2.71".to_string()
        );

        assert_eq!(
            format!(
                "{}",
                Value::List(vec![
                    Value::String("a".to_string()),
                    Value::String("b".to_string()),
                ])
            ),
            "a,b".to_string()
        );

        assert_eq!(format!("{}", Value::List(vec![])), "".to_string());

        Ok(())
    }

    #[test]
    fn serialize_deserialize_tests() -> anyhow::Result<()> {
        // Serialize and deserialize Value::Null
        let null_val = Value::Null;
        let serialized = serde_json::to_value(&null_val)?;
        assert_eq!(serialized, json!(null));

        // Serialize and deserialize Value::String
        let str_val = Value::String("Teyla Emmagan".to_string());
        let serialized = serde_json::to_value(&str_val)?;
        assert_eq!(serialized, json!("Teyla Emmagan"));

        // Deserialize from string
        let deserialized: Value = serde_json::from_str("\"Halling\"")?;
        assert_eq!(deserialized, Value::String("Halling".to_string()));

        // Serialize unsigned number
        let u_val = Value::Number(Number {
            inner: NumberVariant::Unsigned(99),
        });
        let serialized = serde_json::to_value(&u_val)?;
        assert_eq!(serialized, json!(99u64));

        // Serialize signed number
        let s_val = Value::Number(Number {
            inner: NumberVariant::Signed(-5),
        });
        let serialized = serde_json::to_value(&s_val)?;
        assert_eq!(serialized, json!(-5i64));

        // Serialize float number
        let f_val = Value::Number(Number {
            inner: NumberVariant::Float(2.5),
        });
        let serialized = serde_json::to_value(&f_val)?;
        assert_eq!(serialized, json!(2.5f64));

        // Serialize list
        let list_val = Value::List(vec![
            Value::String("x".to_string()),
            Value::String("y".to_string()),
        ]);
        let serialized = serde_json::to_value(&list_val)?;
        assert_eq!(serialized, json!(["x", "y"]));

        // Deserialize i64
        let deserialized: Value = serde_json::from_str("-42")?;
        assert_eq!(
            deserialized,
            Value::Number(Number {
                inner: NumberVariant::Signed(-42)
            })
        );

        // Deserialize u64
        let deserialized: Value = serde_json::from_str("100")?;
        assert!(matches!(
            deserialized,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(100)
            })
        ));

        // Deserialize f64
        let deserialized: Value = serde_json::from_str("1.5")?;
        assert!(matches!(
            deserialized,
            Value::Number(Number {
                inner: NumberVariant::Float(_)
            })
        ));

        // Deserialize null
        let deserialized: Value = serde_json::from_str("null")?;
        assert_eq!(deserialized, Value::Null);

        // Deserialize sequence
        let deserialized: Value = serde_json::from_str("[\"a\", \"b\"]")?;
        assert_eq!(
            deserialized,
            Value::List(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
            ])
        );

        Ok(())
    }

    #[test]
    fn try_from_json_value_tests() -> anyhow::Result<()> {
        use serde_json::Value as JsonValue;
        use std::convert::TryInto;

        // Null
        let v: Value = JsonValue::Null.try_into()?;
        assert_eq!(v, Value::Null);

        // Bool true -> unsigned 1
        let v: Value = JsonValue::Bool(true).try_into()?;
        assert_eq!(
            v,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(1)
            })
        );

        // Bool false -> unsigned 0
        let v: Value = JsonValue::Bool(false).try_into()?;
        assert_eq!(
            v,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(0)
            })
        );

        // u64 number
        let v: Value = json!(42u64).try_into()?;
        assert!(matches!(
            v,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(42)
            })
        ));

        // i64 number (negative)
        let v: Value = json!(-10i64).try_into()?;
        assert!(matches!(
            v,
            Value::Number(Number {
                inner: NumberVariant::Signed(-10)
            })
        ));

        // f64 number
        let v: Value = json!(2.71f64).try_into()?;
        assert!(matches!(
            v,
            Value::Number(Number {
                inner: NumberVariant::Float(_)
            })
        ));

        // String
        let v: Value = json!("hello").try_into()?;
        assert_eq!(v, Value::String("hello".to_string()));

        // Array
        let v: Value = json!(["a", "b"]).try_into()?;
        assert_eq!(
            v,
            Value::List(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
            ])
        );

        // Object -> List of values
        let v: Value = json!({"key": "val"}).try_into()?;
        assert!(matches!(v, Value::List(_)));

        Ok(())
    }

    #[test]
    fn float_cross_type_compare_tests() -> anyhow::Result<()> {
        // float with signed
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            }) == Value::Number(Number {
                inner: NumberVariant::Signed(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Float(3.0)
            }) > Value::Number(Number {
                inner: NumberVariant::Signed(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Float(1.0)
            }) < Value::Number(Number {
                inner: NumberVariant::Signed(2)
            })
        );

        // float with unsigned
        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Float(2.0)
            }) == Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Float(3.0)
            }) > Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            })
        );

        assert_eq!(
            true,
            Value::Number(Number {
                inner: NumberVariant::Float(1.0)
            }) < Value::Number(Number {
                inner: NumberVariant::Unsigned(2)
            })
        );

        Ok(())
    }

    #[test]
    fn nan_comparison_tests() {
        // NaN comparisons - test the fallback paths in total_cmp
        let nan = Value::Number(Number {
            inner: NumberVariant::Float(f64::NAN),
        });
        let two = Value::Number(Number {
            inner: NumberVariant::Float(2.0),
        });

        // NaN comparisons - just verify they don't panic
        let _ = nan.partial_cmp(&two);
        let _ = two.partial_cmp(&nan);
        let _ = nan.partial_cmp(&nan);

        // Float(NaN) vs Signed
        let signed = Value::Number(Number {
            inner: NumberVariant::Signed(2),
        });
        let _ = nan.partial_cmp(&signed);
        let _ = signed.partial_cmp(&nan);

        // Float(NaN) vs Unsigned
        let unsigned = Value::Number(Number {
            inner: NumberVariant::Unsigned(2),
        });
        let _ = nan.partial_cmp(&unsigned);
        let _ = unsigned.partial_cmp(&nan);

        // Test NumberVariant equality cross-type
        let nv_f = NumberVariant::Float(2.0);
        let nv_u = NumberVariant::Unsigned(2);
        let nv_s = NumberVariant::Signed(2);
        assert!(nv_f == nv_u);
        assert!(nv_f == nv_s);
        assert!(nv_u == nv_s);
        assert!(nv_s == nv_u);
        assert!(nv_f == NumberVariant::Float(2.0));

        // NaN float inequality
        let nv_nan = NumberVariant::Float(f64::NAN);
        let _ = nv_nan == nv_f;
        let _ = nv_nan == nv_u;
        let _ = nv_nan == nv_s;
    }

    #[test]
    fn debug_for_number() {
        let n = Number {
            inner: NumberVariant::Signed(42),
        };
        let s = format!("{n:?}");
        assert!(s.contains("42"));
    }

    #[test]
    fn deserialize_via_serde_yaml() -> anyhow::Result<()> {
        // serde_yaml_ng uses visit_str and visit_unit, among others
        let null_val: Value = serde_yaml_ng::from_str("null")?;
        assert_eq!(null_val, Value::Null);

        let str_val: Value = serde_yaml_ng::from_str("\"hello\"")?;
        assert_eq!(str_val, Value::String("hello".to_string()));

        let num_val: Value = serde_yaml_ng::from_str("42")?;
        assert!(matches!(num_val, Value::Number(_)));

        let list_val: Value = serde_yaml_ng::from_str("- a\n- b\n")?;
        assert!(matches!(list_val, Value::List(_)));

        Ok(())
    }

    #[test]
    fn deserialize_option_via_serde_json() -> anyhow::Result<()> {
        // Option<Value> deserialization exercises visit_none and visit_some
        let none_val: Option<Value> = serde_json::from_str("null")?;
        assert!(none_val.is_none());

        let some_val: Option<Value> = serde_json::from_str("\"hello\"")?;
        assert_eq!(some_val, Some(Value::String("hello".to_string())));

        Ok(())
    }

    #[test]
    fn deserialize_visit_string_via_serde_yaml() -> anyhow::Result<()> {
        // serde_yaml_ng calls visit_string (owned String) for quoted strings
        let v: Value = serde_yaml_ng::from_str("'Daniel Jackson'")?;
        assert_eq!(v, Value::String("Daniel Jackson".to_string()));

        let v: Value = serde_yaml_ng::from_str("\"Jack O'Neill\"")?;
        assert_eq!(v, Value::String("Jack O'Neill".to_string()));

        Ok(())
    }

    #[test]
    fn from_numeric_types_tests() -> anyhow::Result<()> {
        let v: Value = 42i64.into();
        assert_eq!(
            v,
            Value::Number(Number {
                inner: NumberVariant::Signed(42)
            })
        );

        let v: Value = 100u64.into();
        assert_eq!(
            v,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(100)
            })
        );

        let v: Value = 1.5f64.into();
        assert_eq!(
            v,
            Value::Number(Number {
                inner: NumberVariant::Float(1.5)
            })
        );

        let v: Value = true.into();
        assert_eq!(
            v,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(1)
            })
        );

        let v: Value = false.into();
        assert_eq!(
            v,
            Value::Number(Number {
                inner: NumberVariant::Unsigned(0)
            })
        );

        Ok(())
    }
}
