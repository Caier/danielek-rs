#![allow(non_camel_case_types, unused)]

use super::types::DApiVersion;

pub struct v10;
impl DApiVersion for v10 {
    const VER: &'static str = "v10";
}

pub struct v6;
impl DApiVersion for v6 {
    const VER: &'static str = "v6";
}
