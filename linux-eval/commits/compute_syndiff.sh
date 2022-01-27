#!/bin/bash
SYNDIFF=${SYNDIFF:-../../../syndiff/target/release/syndiff}
out_name=${1:-syndiff}

for f in */*.dev_base.{c,h}
do
    ext=${f: -1}
    base_name=${f%.dev_base.$ext}
    $SYNDIFF -m $base_name.dev_base.$ext $base_name.modif.$ext $base_name.release_base.$ext > $base_name.$out_name.$ext
done

