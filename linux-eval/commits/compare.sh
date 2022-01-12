#!/bin/bash

git diff --no-index $1.dev_base.$2 $1.modif.$2
git diff --no-index $1.release_base.$2 $1.backported.$2
git diff --no-index $1.release_base.$2 $1.syndiff.$2
