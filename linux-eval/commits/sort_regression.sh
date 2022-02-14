#!/bin/bash
SYNDIFF=${SYNDIFF:-../../../syndiff/target/release/syndiff}

for f in */*.dev_base.{c,h}
do
    ext=${f: -1}
    base_name=${f%.dev_base.$ext}
    if $SYNDIFF -c $f $base_name.modif.$ext $base_name.release_base.$ext > $base_name.diff.$ext
    then
        echo "Solved regression for $(dirname $f)"
        mkdir -p solved
        mv $(dirname $f) solved
    else
        less -rF $base_name.diff.$ext
        if read -e -p "Choose the sorting category for $(dirname $f): " sorting_dir
        then
            mkdir -p $sorting_dir
            mv $(dirname $f) $sorting_dir
        fi
    fi
done
