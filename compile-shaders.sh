#!/bin/sh

GREEN="\033[38;5;2m"
RESET="\033[0m"

for file in shaders/*
do
    filename=$(basename $file)
    basename=${filename%.*}
    format=${filename#*.}
    out="compiled-shaders/$basename-$format.spv"
    echo "== Compiling $GREEN${file}$RESET to $GREEN${out}$RESET =="
    glslc $file -o $out
    if [ $? -ne 0 ] ; then exit 1; fi
done
