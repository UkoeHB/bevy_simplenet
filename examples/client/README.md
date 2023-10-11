This demo client features a button that is deselected when any other client selects their own button. To promote seamless 'gameplay', we must handle some issues. The solutions here are only a demonstration of what it takes to build a robust networked experience, they are not necessarily optimal, scalable, or applicable to all use-cases.

1. Latency: There is a delay between sending a client input request and receiving an ack from the server.

To address this, we use client prediction. When you click a button, it is selected immediately instead of waiting for the server to acknowledge your input.

2. Client disconnects can cause client prediction to fail.

If a client-side predicted input fails in any way, we need to roll back all predicted effects and apply the latest server-authoritative state. This means always tracking the current server-authoritative state.

If there is a reconnect sequence, it is possible that server messages were lost, leaving it unclear what state the server is in. After a reconnect, we always request the current world state to repair any desynchronization. If there are any waiting client requests, we apply them on top of the repaired world state.


### Native client

`cd` to this directory, then:

```
cargo run --release
```


### WASM client

We defined a custom `release-wasm` target to demonstrate an optimized WASM build.

1. Prep tooling
- `rustup target install wasm32-unknown-unknown`
- `cargo install wasm-pack`
- install [`wasm-opt`](https://github.com/webassembly/binaryen)
- setup [wasm-server-runner](https://github.com/jakobhellermann/wasm-server-runner)

2. `cd` to this directory, then:

```
wasm-pack build --target no-modules --mode no-install &&
cargo build --profile=release-wasm --target wasm32-unknown-unknown &&
wasm-bindgen --out-dir ./pkg --target no-modules ./../../target/wasm32-unknown-unknown/release-wasm/bevy_simplenet-client.wasm &&
wasm-opt -Os ./pkg/bevy_simplenet-client_bg.wasm -o ./pkg/bevy_simplenet-client_bg.wasm
```

Note that this workflow builds the binary twice. Since `wasm-pack` doesn't allow custom profiles, we need to rebuild the second time to get the optimized `release-wasm` binary.

3. Run the wasm binary locally:

```
wasm-server-runner ./pkg/bevy_simplenet-client_bg.wasm
```
