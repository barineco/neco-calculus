use core::mem::size_of;

use neco_calculus::CalculusFailure;

const MAXIMUM_BYTES: usize = 96;

#[test]
fn calculus_failure_size_stays_within_bound() {
    let actual = size_of::<CalculusFailure>();
    assert!(
        actual <= MAXIMUM_BYTES,
        "CalculusFailure is {actual} bytes; the bound is {MAXIMUM_BYTES}"
    );
}
