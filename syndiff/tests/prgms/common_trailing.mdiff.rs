fn f(c: bool) -> i32 {
    unchanged![];
    if unchanged![] {
        unchanged![];
        deleted ! [ mv_conflict ! [ 0 , { mv ! [ 1 ] = mv ! [ 1 ] * mv ! [ 2 ] ; } , { mv ! [ 1 ] = g ( mv ! [ 1 ] ) * mv ! [ 2 ] ; } ] ; ];
    } else {
        unchanged![];
        deleted ! [ mv_conflict ! [ 0 , { mv ! [ 1 ] = mv ! [ 1 ] * mv ! [ 2 ] ; } , { mv ! [ 1 ] = g ( mv ! [ 1 ] ) * mv ! [ 2 ] ; } ] ; ];
    }
    inserted ! [ mv ! [ 0 ] ; ];
    unchanged![];
}
