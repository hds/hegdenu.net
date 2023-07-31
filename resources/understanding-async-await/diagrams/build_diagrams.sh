#!/bin/sh

for d2 in $(ls hello-*.d2); do
    d2 \
        --layout=elk \
        --theme=200 \
        --pad=30 \
        "$d2" \
        "../../../static/img/understanding-async-await-1/${d2%.*}.svg"
done

for d2 in $(ls ready-*.d2 pending-*.d2 yield_now-*.d2); do
    d2 \
        --layout=elk \
        --theme=200 \
        --pad=30 \
        "$d2" \
        "../../../static/img/understanding-async-await-2/${d2%.*}.svg"
done

for d2 in $(ls hold_mutex_guard-*.d2 spawn_*.d2 mutex-*.d2); do
    d2 \
        --layout=elk \
        --theme=200 \
        --pad=30 \
        "$d2" \
        "../../../static/img/understanding-async-await-3/${d2%.*}.svg"
done