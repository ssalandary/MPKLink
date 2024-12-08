#!/bin/bash
SIZES=(100 1000 10000 100000 1000000 10000000 100000000)

for size in ${SIZES[@]}; do
    touch tests/$size.in
    shuf -i 1-$size -o tests/$size.in
    tr '\n' ' ' < tests/$size.in > tests/$size.in.tmp
    mv tests/$size.in.tmp tests/$size.in
    echo "Generated tests/$size.in"
done
