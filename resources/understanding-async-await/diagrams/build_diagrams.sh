#!/bin/sh

for d2 in $(ls *.d2); do
    d2 \
        --layout=elk \
        --theme=200 \
        --pad=30 \
        "$d2" \
        "../../../static/img/understanding-async-await-1/${d2%.*}.svg"
done