#!/bin/bash
SYNDIFF=${SYNDIFF:-../../../syndiff/target/release/syndiff}
REMOVE_INDENT=${REMOVE_INDENT:-python3 ../../remove_indent.py}
out_name=${1:-syndiff_noindent}

for f in */*.dev_base.{c,h}
do
    ext=${f: -1}
    base_name=${f%.dev_base.$ext}
    $REMOVE_INDENT $f
    $REMOVE_INDENT $base_name.modif.$ext
    $REMOVE_INDENT $base_name.release_base.$ext
    $REMOVE_INDENT $base_name.backported.$ext
    $SYNDIFF -m $base_name.dev_base_noindent.$ext $base_name.modif_noindent.$ext $base_name.release_base_noindent.$ext > $base_name.$out_name.$ext
done

