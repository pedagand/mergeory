#!/bin/bash
SYNDIFF=../../../syndiff/target/release/syndiff
SYNDIFF_OPT=--no-elisions

for f in */*.dev_base.c; do $SYNDIFF $SYNDIFF_OPT -m $f ${f%.dev_base.c}.modif.c ${f%.dev_base.c}.release_base.c > ${f%.dev_base.c}.syndiff.c; done
for f in */*.dev_base.h; do $SYNDIFF $SYNDIFF_OPT -m $f ${f%.dev_base.h}.modif.h ${f%.dev_base.h}.release_base.h > ${f%.dev_base.h}.syndiff.h; done
