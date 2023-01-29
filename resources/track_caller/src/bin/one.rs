use track_caller_demo::do_not_call_with_one;

fn code_written_by_crate_user() {
    do_not_call_with_one(1);
}

fn main() {
    code_written_by_crate_user();
}