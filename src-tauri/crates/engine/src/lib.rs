#![cfg_attr(not(debug_assertions), deny(unused_imports))]
#![cfg_attr(debug_assertions, allow(unused_imports))]

#[cfg(feature = "zip")]
extern crate zip;

pub mod integrity;
pub mod zip_utils;
