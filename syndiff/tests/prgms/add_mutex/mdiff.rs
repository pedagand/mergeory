fn ·· {
    ·;DELETED![

    $0
    $1]

    let · = CHANGED![«$2» -> «answer()»];
    ·
}INSERTED![

fn answer() -> i32 {
    mutex_lock();
    $0
    $1
    mutex_unlock();
    $2
}]
