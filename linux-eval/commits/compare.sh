#!/bin/bash
shopt -s failglob

# COMPARE_SUFFIX can be used to compare files without indentation if set to
# "_noindent"
suffix=$COMPARE_SUFFIX

if (( $# == 1 )) && [ ! -f $1 ]
then
    files=*.$1.*
else
    files="${@:-*.syndiff$suffix.*}"
fi

for f in $files
do
    ext=${f##*.}
    base_name=${f%.*.$ext}

    git diff --no-index $base_name.dev_base$suffix.$ext $base_name.modif$suffix.$ext
    git diff --no-index $base_name.release_base$suffix.$ext $base_name.backported$suffix.$ext
    git diff --no-index $base_name.release_base$suffix.$ext $f
done
