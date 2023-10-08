This demo client features a button that is deselected when any other client selects their own button. We use pseudo-client prediction to select the button immediately when pressed instead of waiting to see if the select is acked by the server. We auto-deselect whenever the connection fails, then pessimistically ignore selection acks that occur after that (it is safe to do this since the server does not track client states). In a real application you'd need more a rigorous state synchronization protocol that handles the possibility of server messages failing to send due to client disconnects (one way is for the client to poll for its current state after reconnecting).


### Native client

`cargo run --release --example client`


### WASM client

