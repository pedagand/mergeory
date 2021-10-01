fn test() {
    if unchanged![] {
        unchanged![0];
        changed![{ x }, { y }]
    } else {
        deleted ! [mv ! [0] ;];
        unchanged![];
        unchanged![];
        inserted ! [mv ! [0] ;];
    }
}
