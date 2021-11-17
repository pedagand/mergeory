fn ·· {DELETED![
    let $0 = 2;
    let $1 = 40;
    let x = $2;]
    let · = CHANGED![«x» -> «answer()»] * CHANGED![«2» -> «5»];
    ·
}INSERTED![

fn answer() -> i32 {
    let $0 = 3;
    let $1 = 42;
    $2
}]
