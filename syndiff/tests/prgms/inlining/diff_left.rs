fn ·· {
    CHANGED![«loop {
        // Infinite print
        $0;
    }» -> «for _ in (0..5) {
        // Five times
        $0;
    }»]
}
