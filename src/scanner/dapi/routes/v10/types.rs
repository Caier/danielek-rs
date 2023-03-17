use crate::scanner::dapi::types::DApiVersion;

#[allow(non_camel_case_types)]
pub struct v10;
impl DApiVersion for v10 {
    const VER: &'static str = "v10";
}