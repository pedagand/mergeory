fn ·· {
    CHANGED![«loop {
        // Infinite print
        $0!(".");
    }» -> «CONFLICT![«for _ in (0..5) {
        // Five times
        $0!(".");
    }», «loop {
        // Infinite print
        $0!("x");
    }»]»]
}
