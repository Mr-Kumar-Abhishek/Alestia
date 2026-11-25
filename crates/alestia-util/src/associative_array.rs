//! Dynamic property-map value tree matching Celestia's `AssociativeArray`.
//!
//! Used across `.ssc`, `.stc`, `.dsc`, `.cel`, and `celestia.cfg` parsers.

use std::collections::BTreeMap;

use crate::Color;

/// Discriminant for [`Value`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValueType {
    /// Floating-point numeric value.
    Number,
    /// UTF-8 string value.
    String,
    /// Ordered sequence of nested [`Value`] items.
    Array,
    /// Nested key-value [`AssociativeArray`].
    Hash,
    /// Boolean flag.
    Boolean,
}

/// Dynamically typed value stored inside an [`AssociativeArray`].
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Numeric value (`f64`).
    Number(f64),
    /// String value.
    String(String),
    /// Array of values.
    Array(Vec<Value>),
    /// Nested key-value map.
    Hash(AssociativeArray),
    /// Boolean value.
    Boolean(bool),
}

impl Value {
    /// Return the [`ValueType`] of this value.
    #[must_use]
    pub const fn value_type(&self) -> ValueType {
        match self {
            Self::Number(_) => ValueType::Number,
            Self::String(_) => ValueType::String,
            Self::Array(_) => ValueType::Array,
            Self::Hash(_) => ValueType::Hash,
            Self::Boolean(_) => ValueType::Boolean,
        }
    }

    /// Borrow as `f64` if this is a [`Value::Number`].
    #[must_use]
    pub const fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(v) => Some(*v),
            _ => None,
        }
    }

    /// Borrow as `&str` if this is a [`Value::String`].
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Borrow as `&[Value]` if this is a [`Value::Array`].
    #[must_use]
    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(arr) => Some(arr.as_slice()),
            _ => None,
        }
    }

    /// Borrow as `&AssociativeArray` if this is a [`Value::Hash`].
    #[must_use]
    pub const fn as_hash(&self) -> Option<&AssociativeArray> {
        match self {
            Self::Hash(h) => Some(h),
            _ => None,
        }
    }

    /// Borrow as `bool` if this is a [`Value::Boolean`].
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

/// Ordered string-keyed property container matching Celestia's `AssociativeArray`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AssociativeArray {
    entries: BTreeMap<String, Value>,
}

impl AssociativeArray {
    /// Create an empty [`AssociativeArray`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Insert or overwrite a key-value entry.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) {
        self.entries.insert(key.into(), value);
    }

    /// Retrieve a raw [`Value`] reference by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.get(key)
    }

    /// Number of entries in the map.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the map contains no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over `(key, value)` pairs in sorted key order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Retrieve an `f64` number by key.
    #[must_use]
    pub fn get_number(&self, key: &str) -> Option<f64> {
        self.get(key)?.as_number()
    }

    /// Retrieve an integer value by key if finite and within `i64` bounds.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "checked finite and bounded within [i64::MIN, i64::MAX]"
    )]
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        let v = self.get_number(key)?;
        if v.is_finite() && v >= (i64::MIN as f64) && v <= (i64::MAX as f64) {
            Some(v.trunc() as i64)
        } else {
            None
        }
    }

    /// Retrieve a string slice by key.
    #[must_use]
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.get(key)?.as_str()
    }

    /// Retrieve a boolean by key.
    #[must_use]
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key)?.as_bool()
    }

    /// Retrieve a 3-element numeric vector `[f64; 3]` by key.
    #[must_use]
    pub fn get_vec3(&self, key: &str) -> Option<[f64; 3]> {
        let arr = self.get(key)?.as_array()?;
        if arr.len() != 3 {
            return None;
        }
        Some([
            arr[0].as_number()?,
            arr[1].as_number()?,
            arr[2].as_number()?,
        ])
    }

    /// Retrieve a 4-element numeric vector `[f64; 4]` by key.
    #[must_use]
    pub fn get_vec4(&self, key: &str) -> Option<[f64; 4]> {
        let arr = self.get(key)?.as_array()?;
        if arr.len() != 4 {
            return None;
        }
        Some([
            arr[0].as_number()?,
            arr[1].as_number()?,
            arr[2].as_number()?,
            arr[3].as_number()?,
        ])
    }

    /// Retrieve a [`Color`] from a 3-element array, 4-element array, or `#rrggbb`/`#rrggbbaa` string.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Color stores normalized f32 RGBA channels matching Celestia's float Color API"
    )]
    pub fn get_color(&self, key: &str) -> Option<Color> {
        match self.get(key)? {
            Value::String(hex) => Color::parse_hex(hex),
            Value::Array(arr) if arr.len() == 3 => {
                let r = arr[0].as_number()? as f32;
                let g = arr[1].as_number()? as f32;
                let b = arr[2].as_number()? as f32;
                Some(Color::from_rgb(r, g, b))
            }
            Value::Array(arr) if arr.len() == 4 => {
                let r = arr[0].as_number()? as f32;
                let g = arr[1].as_number()? as f32;
                let b = arr[2].as_number()? as f32;
                let a = arr[3].as_number()? as f32;
                Some(Color::from_rgba(r, g, b, a))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "mirrors Celestia's unsigned char quantization formula in associativearray_test.cpp"
    )]
    fn quantize_ref(n: f32) -> f32 {
        f32::from((n * 255.99_f32) as u8) / 255.0_f32
    }

    #[test]
    fn color_from_vector3() {
        let mut h = AssociativeArray::new();
        h.insert(
            "color",
            Value::Array(vec![
                Value::Number(0.23),
                Value::Number(0.34),
                Value::Number(0.45),
            ]),
        );

        let val = h.get("color").unwrap();
        assert_eq!(val.value_type(), ValueType::Array);

        let vec = h.get_vec3("color").unwrap();
        assert_eq!(vec, [0.23, 0.34, 0.45]);

        let c = h.get_color("color").unwrap();
        assert!((c.red() - quantize_ref(0.23)).abs() < 1e-6);
        assert!((c.green() - quantize_ref(0.34)).abs() < 1e-6);
        assert!((c.blue() - quantize_ref(0.45)).abs() < 1e-6);
        assert!((c.alpha() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn color_from_vector4() {
        let mut h = AssociativeArray::new();
        h.insert(
            "color",
            Value::Array(vec![
                Value::Number(0.23),
                Value::Number(0.34),
                Value::Number(0.45),
                Value::Number(0.56),
            ]),
        );

        let vec = h.get_vec4("color").unwrap();
        assert_eq!(vec, [0.23, 0.34, 0.45, 0.56]);

        let c = h.get_color("color").unwrap();
        assert!((c.red() - quantize_ref(0.23)).abs() < 1e-6);
        assert!((c.green() - quantize_ref(0.34)).abs() < 1e-6);
        assert!((c.blue() - quantize_ref(0.45)).abs() < 1e-6);
        assert!((c.alpha() - quantize_ref(0.56)).abs() < 1e-6);
    }

    #[test]
    fn color_from_hex_strings() {
        let mut h = AssociativeArray::new();
        h.insert("color", Value::String(String::from("#123456")));
        let c = h.get_color("color").unwrap();
        assert!((c.red() - (f32::from(0x12_u8) / 255.0)).abs() < 1e-6);
        assert!((c.green() - (f32::from(0x34_u8) / 255.0)).abs() < 1e-6);
        assert!((c.blue() - (f32::from(0x56_u8) / 255.0)).abs() < 1e-6);
        assert!((c.alpha() - 1.0).abs() < 1e-6);

        h.insert("color", Value::String(String::from("#12345678")));
        let c4 = h.get_color("color").unwrap();
        assert!((c4.alpha() - (f32::from(0x78_u8) / 255.0)).abs() < 1e-6);
    }
}
