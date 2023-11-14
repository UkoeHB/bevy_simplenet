# Changelog

## [0.4.0]

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
- Fixed unhandled case where sending requests wouldn't be properly aborted when the client's internal handler was dropped. Sending requests are now marked `RequestStatus::Aborted`, and a `ServerVal::Aborted(request_id)` will be emitted by the client. The aborted request's status will eventually transition to either `RequestStatus::SendFailed` or `RequestStatus::RequestLost` depending on the final send status of the request.
- Fixed ambiguity in the `Client` API around when exactly the internal client backend shut down.


## [0.2.0]

### Added

- Added `ServerVal::SendFailed` and `ServerVal::ResponseLost` so you don't need to poll for request errors.


## [0.1.1]

### Fixed

- Example client not working on WASM.


## [0.1.0]

### Added

- Initial release.
