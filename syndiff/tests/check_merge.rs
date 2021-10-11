use goldenfile::Mint;
use std::process::Command;

fn check_merge(test_name: &str, nb: usize) {
    let mut mint = Mint::new("tests/prgms");
    let diff_file = mint
        .new_goldenfile(format!("{}.mdiff.rs", test_name))
        .unwrap();

    let diff_out = Command::new(env!("CARGO_BIN_EXE_syndiff"))
        .arg(format!("tests/prgms/{}.orig.rs", test_name))
        .args((0..nb).map(|i| format!("tests/prgms/{}.edit{}.rs", test_name, i)))
        .stdout(diff_file)
        .output()
        .expect("Failed to launch syndiff");
    eprint!("{}", String::from_utf8_lossy(&diff_out.stderr));
    assert!(diff_out.status.code().unwrap() >= 0);
    assert!(diff_out.stderr.is_empty())
}

macro_rules! check_merge_tests {
    { $($test_name:ident ($nb:expr),)* } => {
        $(#[test]
        fn $test_name() {
            check_merge(stringify!($test_name), $nb);
        })*
    }
}

check_merge_tests! {
    common_trailing(2),
    factorize(2),
    cross_del(2),
    cross_change(2),
    cross_del_and_ins(2),
    same_change(4),
    double_del(2),
    print_macro(2),
}
