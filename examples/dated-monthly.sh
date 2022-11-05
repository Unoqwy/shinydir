#!/bin/dash
if [[ -z $1 ]]; then
    exit 1
fi
echo "$(date -d "$(stat -c '%y' "$1")" '+%b-%Y')/$(basename "$1")"
