# Changelog

## [0.14.0]

- Update to `bevy` v0.15.


## [0.13.2]

- Fix reconnect/close handling when failing to connect.


## [0.13.1]

- Shut down clients instead of attempting to reconnect after `AuthTokens` expire.


## [0.13.0]

- Implement `AuthToken` and `Authentication::Token`.


## [0.12.1]

### Fixed

- Compilation error when `tls-rustls` or `tls-openssl` features were enabled.


## [0.12.0]

### Changed

- Updated to `bevy` v0.14.
- Reworked example to use `bevy_cobweb_ui`.


## [0.11.1]

### Added

- `ServerFactory::new_server_with_router` method for building custom routers.


## [0.11.0]

### Changed

- Remove dependency on nightly.


## [0.10.1]

### Changed

- Minor cleanup.


## [0.10.0]

### Fixed

- The server/client authentication handshake is now done within the websocket channel instead of using the HTTP request. This avoids leaking auth requests when using TLS (they will still be leaked without TLS).


## [0.9.2]

### Added

- Trace statements when a client sends a message or request.


## [0.9.1]

### Changed

- Log an error when the client's internal lock fails.


## [0.9.0]

### Changed

- Update to Bevy v0.13


## [0.8.0]

### Fixed

- Race condition that would allow a client to send a message/request when they have just connected but before the connection event has been consumed, which would allow the message/request to be sent based on stale client state.
- Similar race condition in the server for server messages. Server requests were already synchronized.

### Changed

- `Client::next()` now requires mutable access in order to synchronize consuming connection events with sending messages/requests.
- `Server::next()` now requires mutable access in order to synchronize consuming connection events with sending messages.
- `Server` API no longer returns `Result`.
- `Client` API no longer returns `Result`. The minimum `ezsockets` dependency is now `v0.6.2`.


## [0.7.2]

### Changed

- Bump nightly.


## [0.7.1]

### Changed

- Docs updates.


## [0.7.0]

### Changed

- Rename: `SessionID` -> `SessionId`.
- Adjust: rearrange token before request in `ServerEvent`.


## [0.6.0]

### Added

- Re-export `ezsockets::CloseFrame`.

### Changed

- Updated `Server::close_session()` to take an optional close frame.
- Update dependencies.


## [0.5.3]

### Changed

- Update nightly dependency.

### Added

- More tracing.


## [0.5.2]

### Changed

- Updated examples to bevy v0.12.


## [0.5.1]

### Fixed

- Update: `hash_drain_filter` -> `hash_extract_if` nightly feature. This should fix docs.rs not compiling.


## [0.5.0]

### Changed

- Update to bevy v0.12.


## [0.4.0]

### Changed

- Major refactor to client/server API. Connection reports are now emitted alongside other client/server events. In particular, `ClientReport::Connected` and `ServerReport::Connected` synchronize with old requests failing, which means clients no longer need to manually synchronize with the server. Instead, the server can simply send its current state to the client as soon as it encounters a `ServerReport::Connected`.

### Fixed

- Race condition that would allow a server to send a response to a request from a dead session to a new session.


## [0.3.2]

### Fixed

- `Client::is_dead()` now properly synchronizes with the the client output stream so you can reliably drain the client after it returns true.


## [0.3.1]

### Fixed

- `RequestSignal` now stores its aborted flag as an `Arc<AtomicBool>` so changes are visibile between signal clones.


## [0.3.0]

### Fixed

- Fixed case where a client request could hang in state `RequestStatus::Waiting` if the request was sent right before a disconnect and the client failed to reconnect for a long time. Client requests are now marked as `RequestStatus::ResponseLost` when the client is disconnected. This improves responsiveness at the cost of minor edge cases (race conditions) where a response is not actually lost and we have to discard it.
- Fixed race condition between dropping a client handler and sending requests via `Client::request()` that could allow requests to hang.
- Fixed unhandled case where sending requests wouldn't be properly aborted when the client's internal handler was dropped. Sending requests are now marked `RequestStatus::Aborted`, and a `ClientEvent::Aborted(request_id)` will be emitted by the client. The aborted request's status will eventually transition to either `RequestStatus::SendFailed` or `RequestStatus::RequestLost` depending on the final send status of the request.
- Fixed ambiguity in the `Client` API around when exactly the internal client backend shut down.


## [0.2.0]

### Added

- Added `ClientEvent::SendFailed` and `ClientEvent::ResponseLost` so you don't need to poll for request errors.


## [0.1.1]

### Fixed

- Example client not working on WASM.


## [0.1.0]

### Added

- Initial release.
