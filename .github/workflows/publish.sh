#!/bin/sh

cargo install fn

version=$(cat Cargo.toml | fn '$.match(/version = "(\d+\.\d+\.\d+)"/)[1]')

already_published=$(
    curl https://crates.io/api/v1/crates/fn |
        version="${version}" fn -p '$.versions.map(a => a.num).includes($version)'
)

if [ "${already_published}" = "true" ]; then
    echo "Already published ${version}."
    exit
fi

gh release create "v${version}" --generate-notes
cargo publish
