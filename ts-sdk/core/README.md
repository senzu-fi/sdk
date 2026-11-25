# @senzu/sdk-core

The Rust core (`rust-sdk/core`) compiled to WebAssembly with `wasm-pack --target web`. Same math as the
contracts, bit-exact: health checks, liquidation prices, max borrow and leverage, the swap into a range, previews.

```ts
import init, { evaluateHealth, maxLeverage } from "@senzu/sdk-core";

await init(); // once, loads the .wasm
const leverage = maxLeverage({ collateral, borrow: "pairRatio", params, borrowFeeWad: "0x0" });
```

Integers wider than `u8` are strings (hex on output); `BigInt(value)` reads them. `npm run build` regenerates `pkg/`
from the Rust crate.
