#![cfg_attr(not(debug_assertions), deny(unused_imports))]
#![cfg_attr(debug_assertions, allow(unused_imports))]

pub mod core;

pub mod upload;
pub mod provider;
pub mod db;
pub mod updater;
