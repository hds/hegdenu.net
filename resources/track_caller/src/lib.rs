/// This function will return non-zero values passed to it.
///
/// ### Panics
///
/// This function will panic if the value passed is zero.
#[track_caller]
pub fn do_not_call_with_zero(val: u64) -> u64 {
    if val == 0 {
        panic!("We told you not to do that");
    }

    val
}

/// This function will return non-one values passed to it.
///
/// ### Panics
///
/// This function will panic if the value passed is one.
#[track_caller]
pub fn do_not_call_with_one(val: u64) -> u64 {
    panic_on_bad_value(val, 1);

    val
}

#[track_caller]
fn panic_on_bad_value(val: u64, bad: u64) {
    if val == bad {
        panic!("We told you not to provide bad value: {}", bad);
    }
}

/// Calls (prints) the `name` together with  the calling location.
#[track_caller]
pub fn call_me(name: &str) {
    let caller = std::panic::Location::caller();

    println!(
        "Calling '{name}' from {file}:{line}",
        name = name,
        file = caller.file(),
        line = caller.line(),
    );
}
