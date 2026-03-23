#!/usr/bin/env bash

alias ncc="npx @vercel/ncc"

ncc build index.js -o dist
ncc build post.js --out dist/post
