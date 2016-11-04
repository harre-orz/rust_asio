#!/bin/sh -e

TAGNAME=`git tag --contains=HEAD`
if [ "$TAGNAME" == "" ]; then
    echo "bad git tag!"
    exit 1
fi
cargo clean && cargo doc --no-deps
git branch -D gh-pages | :
git checkout -b gh-pages github/gh-pages
ls | grep -v target | xargs rm -rf
mv target/doc/* .
rm -rf target
git add *
git commit -m "update Document $TAGNAME"
git checkout master

