use goldenfile::Mint;
use std::process::Command;

fn check_diff(test_name: &str) {
    let origin_filename = format!("tests/prgms/{}.orig.rs", test_name);
    let edited_filename = format!("tests/prgms/{}.edit.rs", test_name);

    let mut mint = Mint::new("tests/prgms");
    let diff_file = mint
        .new_goldenfile(format!("{}.diff.rs", test_name))
        .unwrap();

    let diff_out = Command::new(env!("CARGO_BIN_EXE_syndiff"))
        .arg(origin_filename)
        .arg(edited_filename)
        .stdout(diff_file)
        .output()
        .expect("Failed to launch syndiff");
    assert!(diff_out.status.success());
    eprint!("{}", String::from_utf8_lossy(&diff_out.stderr));
    assert!(diff_out.stderr.is_empty())
}

macro_rules! check_diff_tests {
    { $($test_name: ident,)* } => {
        $(#[test]
        fn $test_name() {
            check_diff(stringify!($test_name));
        })*
    }
}

check_diff_tests! {
    from_empty,
    common_trailing,
    remove_else,
    change_sig,
    trait_change,
    sparse_vec,
}
