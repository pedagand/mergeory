#!/bin/bash

git diff --no-index $1.dev_base_noindent.$2 $1.modif_noindent.$2
git diff --no-index $1.release_base_noindent.$2 $1.backported_noindent.$2
git diff --no-index $1.release_base_noindent.$2 $1.syndiff_noindent.$2
