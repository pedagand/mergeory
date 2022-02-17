use goldenfile::Mint;
use std::process::Command;

fn check_merge(test_name: &str, suffix: &str, extra_options: &[&str]) {
    let mut mint = Mint::new(format!("tests/prgms/{}", test_name));
    let diff_file = mint.new_goldenfile(format!("mdiff{}.rs", suffix)).unwrap();

    let diff_out = Command::new(env!("CARGO_BIN_EXE_syndiff"))
        .args(extra_options)
        .arg(format!("tests/prgms/{}/orig.rs", test_name))
        .arg(format!("tests/prgms/{}/edit_left.rs", test_name))
        .arg(format!("tests/prgms/{}/edit_right.rs", test_name))
        .stdout(diff_file)
        .output()
        .expect("Failed to launch syndiff");
    eprint!("{}", String::from_utf8_lossy(&diff_out.stderr));
    let exit_code = diff_out.status.code().unwrap();
    assert!(exit_code >= 0);
    assert!(diff_out.stderr.is_empty());

    if exit_code == 0 {
        let merged_file = mint.new_goldenfile(format!("merged{}.rs", suffix)).unwrap();
        let diff_out = Command::new(env!("CARGO_BIN_EXE_syndiff"))
            .arg("--merge-files")
            .args(extra_options)
            .arg(format!("tests/prgms/{}/orig.rs", test_name))
            .arg(format!("tests/prgms/{}/edit_left.rs", test_name))
            .arg(format!("tests/prgms/{}/edit_right.rs", test_name))
            .stdout(merged_file)
            .output()
            .expect("Failed to launch syndiff");
        eprint!("{}", String::from_utf8_lossy(&diff_out.stderr));
        assert!(diff_out.status.code().unwrap() == 0);
        assert!(diff_out.stderr.is_empty());
    }
}

macro_rules! check_merge_tests {
    { $($test_name:ident,)* } => {
        $(#[test]
        fn $test_name() {
            check_merge(stringify!($test_name), "", &[]);
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
    ord_conflict,
    print_macro,
    inlining,
    disjoint,
}

macro_rules! check_merge_tests_with_opt {
    { $($test_name:ident: $folder:ident $suffix:ident [$($opt: expr),*],)* } => {
        $(#[test]
        fn $test_name() {
            check_merge(stringify!($folder), concat!(".", stringify!($suffix)), &[$($opt),*]);
        })*
    }
}

check_merge_tests_with_opt! {
    factorize_without_elisions: factorize no_elisions ["--no-elisions"],
    double_del_allow_nested: double_del allow_nested_del ["--allow-nested-deletions"],
    ordered_conflict: ord_conflict ordered ["--ordered-insertions"],
}
