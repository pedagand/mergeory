fn f(c: bool) -> i32 {
    unchanged![];
    if unchanged![] {
        unchanged![];
        deleted ! [ mv ! [ 0 ] = mv ! [ 0 ] * mv ! [ 1 ] ; ];
    } else {
        unchanged![];
        deleted ! [ mv ! [ 0 ] = mv ! [ 0 ] * mv ! [ 1 ] ; ];
    }
    inserted ! [ mv ! [ 0 ] = g ( mv ! [ 0 ] ) * mv ! [ 1 ] ; ];
    unchanged![];
}
