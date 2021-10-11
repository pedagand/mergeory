use goldenfile::Mint;
use std::process::Command;

fn check_diff(test_name: &str, i: usize) {
    let mut mint = Mint::new("tests/prgms");
    let diff_file = mint
        .new_goldenfile(format!("{}.diff{}.rs", test_name, i))
        .unwrap();

    let diff_out = Command::new(env!("CARGO_BIN_EXE_syndiff"))
        .arg(format!("tests/prgms/{}.orig.rs", test_name))
        .arg(format!("tests/prgms/{}.edit{}.rs", test_name, i))
        .stdout(diff_file)
        .output()
        .expect("Failed to launch syndiff");
    eprint!("{}", String::from_utf8_lossy(&diff_out.stderr));
    assert!(diff_out.status.success());
    assert!(diff_out.stderr.is_empty())
}

fn check_merge_with_identity(test_name: &str, i: usize) {
    let mut mint = Mint::new("tests/prgms");
    let diff_file = mint
        .new_goldenfile(format!("{}.diff{}.rs", test_name, i))
        .unwrap();

    let diff_out = Command::new(env!("CARGO_BIN_EXE_syndiff"))
        .arg(format!("tests/prgms/{}.orig.rs", test_name))
        .arg(format!("tests/prgms/{}.orig.rs", test_name))
        .arg(format!("tests/prgms/{}.edit{}.rs", test_name, i))
        .stdout(diff_file)
        .output()
        .expect("Failed to launch syndiff");
    eprint!("{}", String::from_utf8_lossy(&diff_out.stderr));
    assert!(diff_out.status.code().unwrap() >= 0);
    assert!(diff_out.stderr.is_empty())
}

macro_rules! check_diff_tests {
    { $($test_name:ident ($nb:expr),)* } => {
        $(#[test]
        fn $test_name() {
            for i in 0..$nb {
                check_diff(stringify!($test_name), i);
                check_merge_with_identity(stringify!($test_name), i);
            }
        })*
    }
}

check_diff_tests! {
    from_empty(1),
    common_trailing(2),
    remove_else(1),
    change_sig(1),
    trait_change(1),
    sparse_vec(1),
    change_and_move(1),
    factorize(2),
    cross_del(2),
    cross_change(2),
    cross_del_and_ins(2),
    same_change(1),
    double_del(2),
    print_macro(2),
}
