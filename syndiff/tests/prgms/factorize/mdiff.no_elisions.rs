fn ·· {DELETED![
    ]DELETE_CONFLICT![«let a = 2;» -/> «let a = 3;»]DELETED![
    ]DELETE_CONFLICT![«let b = 40;» -/> «let b = 42;»]DELETED![
    let x = a + b;]
    let · = CHANGED![«x» -> «answer()»] * CHANGED![«2» -> «5»];
    ·
}INSERTED![

fn answer() -> i32 {
    let a = 2;
    let b = 40;
    a + b
}]
