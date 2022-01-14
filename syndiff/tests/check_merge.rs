use goldenfile::Mint;
use std::process::Command;

fn check_merge(test_name: &str) {
    let mut mint = Mint::new(format!("tests/prgms/{}", test_name));
    let diff_file = mint.new_goldenfile("mdiff.rs").unwrap();

    let diff_out = Command::new(env!("CARGO_BIN_EXE_syndiff"))
        .arg(format!("tests/prgms/{}/orig.rs", test_name))
        .arg(format!("tests/prgms/{}/edit0.rs", test_name))
        .arg(format!("tests/prgms/{}/edit1.rs", test_name))
        .stdout(diff_file)
        .output()
        .expect("Failed to launch syndiff");
    eprint!("{}", String::from_utf8_lossy(&diff_out.stderr));
    assert!(diff_out.status.code().unwrap() >= 0);
    assert!(diff_out.stderr.is_empty())
}

macro_rules! check_merge_tests {
    { $($test_name:ident,)* } => {
        $(#[test]
        fn $test_name() {
            check_merge(stringify!($test_name));
        })*
    }
}

check_merge_tests! {
    common_trailing,
    factorize,
    cross_del,
    cross_change,
    cross_del_and_ins,
    same_change,
    double_del,
    print_macro,
    inlining,
}
