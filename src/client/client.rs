//local shortcuts
use crate::*;

//third-party shortcuts
use bincode::Options;

//standard shortcuts
use core::fmt::Debug;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};

//-------------------------------------------------------------------------------------------------------------------

/// A client for communicating with a [`Server`].
///
/// Use a [`ClientFactory`] to produce a new client.
///
/// It is safe to drop a client, however if you need a complete shut-down procedure then follow these steps:
/// 1) Call [`Client::close()`].
/// 2) Wait for [`Client::is_dead()`] to return true. Note that on WASM targets you cannot busy-wait since doing so
///    will block the client backend.
/// 3) Call [`Client::next()`] to drain any lingering events. [`ClientReport::IsDead`] will be the last event.
/// 4) Drop the client.
#[derive(Debug)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::system::Resource))]
pub struct Client<Channel: ChannelPack>
{
    /// this client's id
    client_id: u128,
    /// core websockets client
    client: ezsockets::Client<ClientHandler<Channel>>,
    /// sender for client events
    client_event_sender: crossbeam::channel::Sender<ClientEventFrom<Channel>>,
    /// receiver for client events
    client_event_receiver: crossbeam::channel::Receiver<ClientEventFrom<Channel>>,
    /// synchronized tracker for pending requests
    pending_requests: Arc<Mutex<PendingRequestTracker>>,
    /// signal for the number of internal disconnects encountered without handled connection events
    client_disconnected_count: Arc<AtomicU16>,
    /// signal for when the internal client is shut down
    client_closed_signal: Arc<AtomicBool>,
    /// flag indicating the client closed itself
    closed_by_self: Arc<AtomicBool>,
}

impl<Channel: ChannelPack> Client<Channel>
{
    /// Sends a one-shot message to the server.
    ///
    /// Returns `Ok(MessageSignal)` on success. The signal can be used to track the message status. Messages
    /// will fail if the underlying client becomes disconnected.
    pub fn send(&self, msg: Channel::ClientMsg) -> MessageSignal
    {
        // lock pending requests
        let Ok(_pending_requests) = self.pending_requests.lock()
        else
        {
            tracing::error!("the client experienced a critical internal error");
            return MessageSignal::new(MessageStatus::Failed);
        };

        // check if connected
        // - We do this after locking the pending requests cache in order to synchronize with dropping the internal
        //   client handler, and to synchronize with disconnect events in the client backend.
        if !self.is_connected()
        {
            tracing::warn!("tried to send message to disconnected client");
            return MessageSignal::new(MessageStatus::Failed);
        }

        // forward message to server
        let Ok(ser_msg) = bincode::DefaultOptions::new().serialize(&ServerMetaEventFrom::<Channel>::Msg(msg))
        else
        {
            tracing::error!("failed serializing client message");
            return MessageSignal::new(MessageStatus::Failed);
        };

        match self.client.binary(ser_msg)
        {
            Ok(signal) =>
            {
                tracing::trace!("sending message to server");
                signal
            }
            Err(_) =>
            {
                tracing::warn!("tried to send message to dead client");
                MessageSignal::new(MessageStatus::Failed)
            }
        }
    }

    /// Sends a request to the server.
    ///
    /// Returns `RequestSignal`. The signal can be used to track the message status.
    /// Requests will fail if the underlying client is or becomes disconnected.
    ///
    /// Failed requests will always emit a client event unless the client has a critical internal error.
    pub fn request(&self, request: Channel::ClientRequest) -> RequestSignal
    {
        // lock pending requests
        let Ok(mut pending_requests) = self.pending_requests.lock()
        else
        {
            tracing::error!("the client experienced a critical internal error");
            return RequestSignal::new(u64::MAX, MessageSignal::new(MessageStatus::Failed));
        };

        // prep request id
        let request_id = pending_requests.reserve_id();

        // check if connected
        // - We do this after locking the pending requests cache in order to synchronize with dropping the internal
        //   client handler, and to synchronize with disconnect events in the client backend.
        if !self.is_connected()
        {
            tracing::warn!("tried to send request to disconnected client");
            return pending_requests.add_request(request_id, MessageSignal::new(MessageStatus::Failed));
        };

        // forward message to server
        let Ok(ser_msg) = bincode::DefaultOptions::new().serialize(
                &ServerMetaEventFrom::<Channel>::Request(request, request_id)
            )
        else
        {
            tracing::error!("failed serializing client request");
            return pending_requests.add_request(request_id, MessageSignal::new(MessageStatus::Failed));
        };

        match self.client.binary(ser_msg)
        {
            Ok(signal) =>
            {
                tracing::trace!("sending request to server");
                pending_requests.add_request(request_id, signal)
            }
            Err(_) =>
            {
                tracing::warn!("tried to send request to dead client");
                pending_requests.add_request(request_id, MessageSignal::new(MessageStatus::Failed))
            }
        }
    }

    /// Tries to get the next client event.
    ///
    /// When the client dies, the last event emitted will be `ClientEvent::Report(ClientReport::IsDead))`.
    ///
    /// After a client disconnects, [`Self::is_connected`] will return `false` until all the
    /// `ClientEvent::Report(ClientReport::Connected))` events for the most recent reconnect has been consumed.
    /// Note that multiple disconnect/reconnect cycles can occur in the backend, so consuming one connected event
    /// does not guarantee the client will be considered connected.
    ///
    /// Note: This method is mutable so that message sending synchronizes with setting the connection signal. We
    ///       expect the caller will handle connection events atomically without interleaving unrelated messages.
    pub fn next(&mut self) -> Option<ClientEventFrom<Channel>>
    {
        let Ok(msg) = self.client_event_receiver.try_recv() else { return None; };

        // If we connected to the server, mark the client as connected.
        // - We do this when consuming the connection report so client messages and requests cannot be sent
        //   when the client has not yet realized it is connected.
        // - Without this, it is possible for client messages to be sent based on client state that is derived from
        //   an old session. We want client messages to be synchronized with `Connected` events.
        if let ClientEventFrom::<Channel>::Report(ClientReport::Connected) = &msg
        {
            self.client_disconnected_count.fetch_sub(1u16, Ordering::Release);
        }

        Some(msg)
    }

    /// Access this client's id.
    pub fn id(&self) -> u128
    {
        self.client_id
    }

    /// Tests if the client is connected.
    ///
    /// Messages and requests cannot be submitted when the client is not connected.
    pub fn is_connected(&self) -> bool
    {
        self.client_disconnected_count.load(Ordering::Acquire) == 0 && !self.is_closed()
    }

    /// Tests if the client is dead (no longer connected to the server and won't reconnect).
    /// - Note that [`ClientReport::IsDead`] will be emitted by [`Client::next()`] when the client backend dies.
    ///
    /// Once this returns true you can drain the client by calling [`Client::next()`] until no more values appear.
    /// After [`ClientReport::IsDead`] appears, [`Client::next()`] will always return `None`.
    pub fn is_dead(&self) -> bool
    {
        self.client_closed_signal.load(Ordering::Acquire)
    }

    /// Tests if the client is closed.
    ///
    /// Returns true after [`Client::close()`] has been called, or once the internal client dies.
    ///
    /// Messages and requests cannot be submitted once the client is closed.
    pub fn is_closed(&self) -> bool
    {
        self.closed_by_self.load(Ordering::Acquire) || self.is_dead()
    }

    /// Closes the client.
    ///
    /// Any in-progress messages may or may not fail once this method is called. New messages and requests cannot be
    /// sent after this method is called.
    ///
    /// The client will eventually emit [`ClientReport::IsDead`] once this method has been called.
    pub fn close(&self)
    {
        // sanity check
        if self.is_closed() { tracing::warn!("tried to close an already closed client"); return; }
        tracing::info!("client closing self");

        // close the client
        let closure_frame =
            ezsockets::CloseFrame{
                code   : ezsockets::CloseCode::Normal,
                reason : String::from("client done")
            };
        if self.client.close(Some(closure_frame)).is_err()
        {
            tracing::warn!("tried to close an already dead client");
            return;
        }

        // forward event to other end of channel
        if let Err(err) = self.client_event_sender.send(ClientEventFrom::<Channel>::Report(ClientReport::ClosedBySelf))
        {
            tracing::error!(?err, "failed to forward connection event to client");
        }

        // note: request failures will be emitted for all pending requests when the internal client is dropped

        // mark the client as closed
        self.closed_by_self.store(true, Ordering::Release);
    }
}

impl<Channel: ChannelPack> Drop for Client<Channel>
{
    fn drop(&mut self)
    {
        if self.is_dead() { return; }
        self.close();
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Factory for producing [`Client`]s that all bake in the same protocol version.
//todo: use const generics on the protocol version instead (currently broken, async methods cause compiler errors)
#[derive(Debug, Clone)]
pub struct ClientFactory<Channel: ChannelPack>
{
    protocol_version : &'static str,
    _phantom         : PhantomData<Channel>,
}

impl<Channel: ChannelPack> ClientFactory<Channel>
{
    /// Makes a new server factory with a given protocol version.
    pub fn new(protocol_version: &'static str) -> Self
    {
        ClientFactory{ protocol_version, _phantom: PhantomData::default() }
    }

    /// Makes a new client.
    pub fn new_client(&self,
        runtime_handle : enfync::builtin::Handle,
        url            : url::Url,
        auth           : AuthRequest,
        config         : ClientConfig,
        connect_msg    : Channel::ConnectMsg,
    ) -> Client<Channel>
    {
        // prepare to make client connection
        // note: urls cannot contain raw bytes so we must serialize as json
        let client_config = ezsockets::ClientConfig::new(url)
            .reconnect_interval(config.reconnect_interval)
            .max_initial_connect_attempts(config.max_initial_connect_attempts)
            .max_reconnect_attempts(config.max_reconnect_attempts)
            .query_parameter(VERSION_MSG_KEY, self.protocol_version)
            .query_parameter(TYPE_MSG_KEY, env_type_as_str(env_type()));

        // prepare client's socket config
        let mut socket_config = ezsockets::SocketConfig::default();
        socket_config.heartbeat = config.heartbeat_interval;
        socket_config.timeout   = config.keepalive_timeout;

        #[cfg(target_family = "wasm")]
        {
            // on WASM we need custom Ping/Pong protocol
            socket_config.heartbeat_ping_msg_fn = Arc::new(text_ping_fn);
        }

        let client_config = client_config.socket_config(socket_config);

        // prepare message channel that points out of our client
        let (client_event_sender, client_event_receiver) = crossbeam::channel::unbounded::<ClientEventFrom<Channel>>();

        // prepare client connector
        let client_connector = {
                #[cfg(not(target_family = "wasm"))]
                { ezsockets::ClientConnectorTokio::from(runtime_handle.clone()) }

                #[cfg(target_family = "wasm")]
                { ezsockets::ClientConnectorWasm::default() }
            };

        // prep auth
        let client_id = auth.client_id();
        let auth = ClientAuthMsg{ auth, msg: connect_msg };

        // make client core with our handler
        let client_event_sender_clone = client_event_sender.clone();
        let pending_requests = Arc::new(Mutex::new(PendingRequestTracker::default()));
        let pending_requests_clone = pending_requests.clone();
        let client_disconnected_count = Arc::new(AtomicU16::new(1u16));  //start at 1 for 'starting disconnected'
        let client_closed_signal = Arc::new(AtomicBool::new(false));
        let client_disconnected_count_clone = client_disconnected_count.clone();
        let client_closed_signal_clone = client_closed_signal.clone();
        let (client, _client_task_handle) = ezsockets::connect_with(
                move |client|
                {
                    ClientHandler::<Channel>{
                            config,
                            auth,
                            client,
                            client_event_sender       : client_event_sender_clone,
                            pending_requests          : pending_requests_clone,
                            client_disconnected_count : client_disconnected_count_clone,
                            client_closed_signal      : client_closed_signal_clone,
                        }
                },
                client_config,
                client_connector,
            );

        // finish assembling our client
        tracing::info!("created new client");

        Client{
                client_id,
                client,
                client_event_sender,
                client_event_receiver,
                pending_requests,
                client_disconnected_count,
                client_closed_signal,
                closed_by_self: Arc::new(AtomicBool::new(false)),
            }
    }
}

//-------------------------------------------------------------------------------------------------------------------
