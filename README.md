# Senzu SDK

Senzu's contract math, bit-exact with the Solidity in [senzu-fi/senzu-core](https://github.com/senzu-fi/senzu-core): health
checks, share and interest math, liquidations, lens searches, and the previews for opening, editing and closing a
position.

| Package | What it is |
|---|---|
| [`rust-sdk/core`](rust-sdk/core) | `senzu-sdk-core`, the math in Rust (`alloy-primitives`, `no_std`-friendly, a `wasm` feature) |
| [`ts-sdk/core`](ts-sdk/core) | `@senzu/sdk-core`, the same crate compiled to WebAssembly, with TypeScript types |

Every function is checked against vectors that `contracts/script/SdkVectors.s.sol` runs through the real contracts.
`contracts/` is the core repository as a submodule; the vectors in `rust-sdk/core/tests/vectors` must equal its
`formal/vectors`, which also tie the Lean proofs to the same contracts.

## Build and test

```
git submodule update --init      # the contracts, for the vectors check
just check                       # fmt, clippy, tests, docs, MSRV, wasm, vectors in sync
just wasm-pkg                    # rebuild ts-sdk/core/pkg after a Rust change (needs wasm-pack)
```

## License

MIT, see [LICENSE](LICENSE).
