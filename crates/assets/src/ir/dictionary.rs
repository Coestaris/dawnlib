use glam::{Mat3, Mat4, Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum IRDictionaryEntry {
    String(String),
    Int(i64),
    UInt(u64),
    F32(f32),
    Bool(bool),
    Map(HashMap<String, IRDictionaryEntry>),
    Array(Vec<IRDictionaryEntry>),
    Vec2f([f32; 2]),
    Vec3f([f32; 3]),
    Vec4f([f32; 4]),
    Mat3f([f32; 3 * 3]),
    Mat4f([f32; 4 * 4]),
}

impl IRDictionaryEntry {
    pub fn as_string(&self) -> Option<&String> {
        if let IRDictionaryEntry::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        if let IRDictionaryEntry::Int(i) = self {
            Some(*i)
        } else {
            None
        }
    }

    pub fn as_uint(&self) -> Option<u64> {
        if let IRDictionaryEntry::UInt(u) = self {
            Some(*u)
        } else {
            None
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        if let IRDictionaryEntry::F32(f) = self {
            Some(*f)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let IRDictionaryEntry::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    pub fn as_map(&self) -> Option<&HashMap<String, IRDictionaryEntry>> {
        if let IRDictionaryEntry::Map(m) = self {
            Some(m)
        } else {
            None
        }
    }

    pub fn as_array(&self) -> Option<&Vec<IRDictionaryEntry>> {
        if let IRDictionaryEntry::Array(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_vec2f(&self) -> Option<Vec2> {
        if let IRDictionaryEntry::Vec2f(v) = self {
            Some(Vec2::from(*v))
        } else {
            None
        }
    }

    pub fn as_vec3f(&self) -> Option<Vec3> {
        if let IRDictionaryEntry::Vec3f(v) = self {
            Some(Vec3::from(*v))
        } else {
            None
        }
    }

    pub fn as_vec4f(&self) -> Option<Vec4> {
        if let IRDictionaryEntry::Vec4f(v) = self {
            Some(Vec4::from(*v))
        } else {
            None
        }
    }

    pub fn as_mat3f(&self) -> Option<Mat3> {
        if let IRDictionaryEntry::Mat3f(m) = self {
            Some(Mat3::from_cols_array(m))
        } else {
            None
        }
    }

    pub fn as_mat4f(&self) -> Option<Mat4> {
        if let IRDictionaryEntry::Mat4f(m) = self {
            Some(Mat4::from_cols_array(m))
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IRDictionary {
    pub entries: Vec<IRDictionaryEntry>,
}

impl IRDictionary {
    pub fn memory_usage(&self) -> usize {
        let sum = size_of::<IRDictionary>();
        sum
    }
}
