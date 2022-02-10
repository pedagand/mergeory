use goldenfile::Mint;
use std::process::Command;

fn check_diff(test_name: &str, suffix: &str) {
    let mut mint = Mint::new(format!("tests/prgms/{}", test_name));
    let diff_file = mint.new_goldenfile(format!("diff{}.rs", suffix)).unwrap();

    let diff_out = Command::new(env!("CARGO_BIN_EXE_syndiff"))
        .arg(format!("tests/prgms/{}/orig.rs", test_name))
        .arg(format!("tests/prgms/{}/edit{}.rs", test_name, suffix))
        .stdout(diff_file)
        .output()
        .expect("Failed to launch syndiff");
    eprint!("{}", String::from_utf8_lossy(&diff_out.stderr));
    assert!(diff_out.status.success());
    assert!(diff_out.stderr.is_empty())
}

macro_rules! check_one_diff_tests {
    { $($test_name:ident,)* } => {
        $(#[test]
        fn $test_name() {
            check_diff(stringify!($test_name), "");
        })*
    }
}

check_one_diff_tests! {
    from_empty,
    remove_else,
    change_sig,
    trait_change,
    sparse_vec,
    change_and_move,
    same_change,
}

macro_rules! check_two_diffs_tests {
    { $($test_name:ident,)* } => {
        $(#[test]
        fn $test_name() {
            check_diff(stringify!($test_name), "_left");
            check_diff(stringify!($test_name), "_right");
        })*
    }
}

check_two_diffs_tests! {
    common_trailing,
    factorize,
    cross_del,
    cross_change,
    cross_del_and_ins,
    double_del,
    ord_conflict,
    print_macro,
    inlining,
    disjoint,
}
