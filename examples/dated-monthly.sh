#!/bin/dash
if [[ -z $1 ]]; then
    exit 1
fi
date -d "$(stat -c '%y' "$1")" '+%b-%Y'
