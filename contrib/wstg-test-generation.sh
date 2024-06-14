#!/bin/bash

CWD=$(pwd)

pushd $(mktemp -d)

git clone https://github.com/OWASP/wstg

find wstg/document/4-Web_Application_Security_Testing -name "*.md" -type f | grep -v "README.md" | grep -vE '\/[0-9]+\.[0-9]-' > wstg-test-documents.txt

for doc in $(cat wstg-test-documents.txt);
do
    printf '%s\n%s\n%s' "https://github.com/OWASP/wstg/blob/master/document/4-Web_Application_Security_Testing/$(basename $(dirname $doc))/$(basename $doc)" "$(basename $(dirname $doc) | sed -E 's/[-0-9]+//g' | sed 's/_/ /g')" "$(head -n 5 $doc)" \
        | sed '/^[[:space:]]*$/d' \
        | awk 'BEGIN{ RS = ""; FS = "\n"; ORS = "\n"; OFS = ","  } { print $6,$2,$3,$1 }' \
        | sed -E 's/(# |\|)//g' >> wstg-tests.csv
done

cp wstg-tests.csv "$CWD/wstg-tests.csv"

popd
