//! Data types for the zk computation output.
//! Similar structs can be found in zk_compute.rs

use create_type_spec_derive::CreateTypeSpec;
use read_write_state_derive::ReadWriteState;

#[derive(ReadWriteState, CreateTypeSpec, Clone)]
pub struct StatisticsResult {
    pub age_counts: AgeCounts,
    pub gender_counts: GenderCounts,
    pub color_counts: ColorCounts,
}

#[derive(ReadWriteState, CreateTypeSpec, Clone)]
pub struct AgeCounts {
    pub age0to19: i32,
    pub age20to39: i32,
    pub age40to59: i32,
    pub age60plus: i32,
}

#[derive(ReadWriteState, CreateTypeSpec, Clone)]
pub struct GenderCounts {
    pub male: i32,
    pub female: i32,
    pub other: i32,
}

#[derive(ReadWriteState, CreateTypeSpec, Clone)]
pub struct ColorCounts {
    pub red: i32,
    pub blue: i32,
    pub green: i32,
    pub yellow: i32,
}
