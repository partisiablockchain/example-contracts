//! ZK computation for statistics contract
use create_type_spec_derive::CreateTypeSpec;
use pbc_zk::*;

/// Input type. Each value is inputted as an i8 integer.
/// If a value outside the given range is inputted, it will not be counted.
#[derive(Clone, SecretBinary, CreateTypeSpec)]
pub struct EntryPoint {
    /// the age group to input. The following values can be inputted:
    /// 1 = age 0-19,
    /// 2 = age 20-39,
    /// 3 = age 40-59,
    /// 4 = age 60+
    pub age_choice: Sbi8,
    /// the gender to input. The following values can be inputted:
    /// 1 = Male,
    /// 2 = Female,
    /// 3 = Other
    pub gender_choice: Sbi8,
    /// the color to input. The following values can be inputted:
    /// 1 = Red,
    /// 2 = Blue,
    /// 3 = Green,
    /// 4 = Yellow
    pub color_choice: Sbi8,
}

/// Output data types.
#[derive(Clone, PartialEq)]
pub struct StatisticsOutput {
    pub age_counts: AgeCounts,
    pub gender_counts: GenderCounts,
    pub color_counts: ColorCounts,
}

#[derive(Clone, PartialEq, Debug)]
pub struct AgeCounts {
    pub age0to19: Sbi32,
    pub age20to39: Sbi32,
    pub age40to59: Sbi32,
    pub age60plus: Sbi32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct GenderCounts {
    pub male: Sbi32,
    pub female: Sbi32,
    pub other: Sbi32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ColorCounts {
    pub red: Sbi32,
    pub blue: Sbi32,
    pub green: Sbi32,
    pub yellow: Sbi32,
}

#[allow(clippy::needless_range_loop, dead_code)]
#[zk_compute(shortname = 0x40)]
pub fn compute_statistics() -> StatisticsOutput {
    // Initialize counts
    let mut age_count = [Sbi32::from(0); 4];
    let mut gender_count = [Sbi32::from(0); 3];
    let mut color_count = [Sbi32::from(0); 4];

    // Analyze input data
    for variable_id in secret_variable_ids() {
        let input = load_sbi::<EntryPoint>(variable_id);

        for idx in 0usize..4usize {
            if input.age_choice == Sbi8::from((idx + 1usize) as i8) {
                age_count[idx] = age_count[idx] + Sbi32::from(1)
            }
        }

        for idx in 0usize..3usize {
            if input.gender_choice == Sbi8::from((idx + 1usize) as i8) {
                gender_count[idx] = gender_count[idx] + Sbi32::from(1)
            }
        }

        for idx in 0usize..4usize {
            if input.color_choice == Sbi8::from((idx + 1usize) as i8) {
                color_count[idx] = color_count[idx] + Sbi32::from(1)
            }
        }
    }

    StatisticsOutput {
        age_counts: AgeCounts {
            age0to19: age_count[0],
            age20to39: age_count[1],
            age40to59: age_count[2],
            age60plus: age_count[3],
        },
        gender_counts: GenderCounts {
            male: gender_count[0],
            female: gender_count[1],
            other: gender_count[2],
        },
        color_counts: ColorCounts {
            red: color_count[0],
            blue: color_count[1],
            green: color_count[2],
            yellow: color_count[3],
        },
    }
}
