use crate::u128_division_ceil;

#[test]
pub fn test_u128_division_ceil() {
    // Division by 0 cases is guarded against by u128 type and the source code

    let div1 = u128_division_ceil(10, 2);
    let div2 = u128_division_ceil(999, 66);
    let div3 = u128_division_ceil(15, 4);

    assert_eq!(div1, Ok(5));
    assert_eq!(div2, Ok(16));
    assert_eq!(div3, Ok(4));
    assert_eq!(u128_division_ceil(15, 0), Err("Division by zero"));
}

#[test]
pub fn test_u128_division_ceil_2() {
    let a: u128 = 0xDEADBEEF;
    let b: u128 = 0xC0FFEE;
    let k: u128 = a * b;
    assert_eq!(u128_division_ceil(k, a), Ok(b));
    assert_eq!(u128_division_ceil(k, b), Ok(a));
}
