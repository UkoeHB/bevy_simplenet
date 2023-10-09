This demo client features a button that is deselected when any other client selects their own button. To promote seamless 'gameplay', we must handle several issues. The solutions here are only a demonstration of what it takes to build a robust networked experience, they are not necessarily optimal, scalable, or applicable to all use-cases.

1. Latency: There is a delay between sending a client message and receiving an ack from the server.

To address this, we use client prediction. When you click a button, it is selected immediately instead of waiting for the server to acknowledge your input.

2. Client-side disconnect: When you submit a client message, it will fail to send if the client is/becomes disconnected.

We track the `bevy_simplenet::MessageStatus` of a client select message. If the status is `MessageStatus::Failed`, then we roll back the predicted state of the button (we deselect it). We also completely reject inputs when the client is disconnected, rather than buffering them.

3. Server-side disconnect: When the server sends a message to a client, it will fail to send if the client is/becomes disconnected.

Since client inputs are predicted, when a client sends a select message we want to ignore all deselects from the server until the client's select is acked by the server. That way the client's UI synchronizes with the server state. However, if the client disconnects after sending a select then the server ack may fail to send.

It is not safe to assume that the client is selected on the server when a select is successfully sent, because if the client was disconnected then the ack **and** subsequent deselects may have failed to send. To get around this issue, when a client reconnects to the server, the server sends the client's current state in addition to the client's last acked message id. If the message id equals the last sent select, then the client sets its state equal to the state it received from the server. Otherwise it does nothing, because the last sent select is still a predicted input waiting for an ack.


### Native client

`cargo run --release --example client`


### WASM client

