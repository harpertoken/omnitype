#!/bin/bash

# Rewrite commit messages to enforce conventional standards:
# - Lowercase first line
# - Truncate first line to <=60 characters

git filter-branch --force --msg-filter '
msg=$(cat)
first_line=$(echo "$msg" | head -1 | tr "[:upper:]" "[:lower:]")
if [ ${#first_line} -gt 60 ]; then
    first_line=${first_line:0:60}
fi
echo "$first_line"
echo "$msg" | tail -n +2
' -- --all
