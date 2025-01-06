#!/bin/sh

cargo install fn

version=$(
    cat Cargo.toml | fn '$.match(/version = "(\d+\.\d+\.\d+)"/)[1]'
)

newest_version=$(
    curl https://crates.io/api/v1/crates/fn | fn -p '$.crate.newest_version'
)

if [ "${version}" = "${newest_version}" ]; then
  echo "Already published ${version}."
  exit
fi

git tag -a "${version}" -m "${version}"
git push --tags
cargo publish
