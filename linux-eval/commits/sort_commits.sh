#!/bin/bash
cur_script_path=$(realpath $0)
cur_script_dir=$(dirname $cur_script_path)

for f in */*.syndiff$COMPARE_SUFFIX.*
do
    $cur_script_dir/compare.sh $f
    if read -e -p "Choose the sorting category for $(dirname $f): " sorting_dir
    then
        mkdir -p $sorting_dir
        mv $(dirname $f) $sorting_dir
    fi
done
