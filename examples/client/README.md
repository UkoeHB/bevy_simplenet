This demo client features a button that is deselected when any other client selects their own button. To promote seamless 'gameplay', we must handle some issues. The solutions in this example are only a demonstration of what it takes to build a robust networked experience, they are not necessarily optimal, scalable, or applicable to all use-cases.

1. Latency: There is a delay between sending a client input request and receiving an ack from the server.

To address this, we use client prediction. When you click a button, it is selected immediately instead of waiting for the server to acknowledge your input.

2. Client disconnects can cause client prediction to be erroneous.

If a client-side predicted input fails to send or receive a response, we need to roll back all predicted effects and apply the latest server-authoritative state. This means always tracking the current server-authoritative state.

If there is a reconnect sequence, it is possible that server messages were lost, leaving it unclear what state the server is in. After a reconnect, we always request the current world state to repair any desynchronization. If there are any pending predicted client inputs, we apply them on top of the repaired world state.


### Native client

1. `cd` to this directory
2. `cargo run`


### WASM client

1. setup [wasm-server-runner](https://github.com/jakobhellermann/wasm-server-runner)
2. `cd` to this directory
3. `cargo run --target wasm32-unknown-unknown`
4. Enter the address that appears in a new browser window.
