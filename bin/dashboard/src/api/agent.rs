use crate::prelude::axum::*;
use crate::state::AppState;
use anyhow::anyhow;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use axum::Json;
use proto::agent::Events;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Finds the host with the given `machine_id` in the database and returns its
/// configuration. If the host does not exist, creates a new host with the given
/// `machine_id` and returns its configuration.
///
/// # Errors
///
/// Returns an error if database operations fail.
pub async fn config(
    State(state): State<Arc<AppState>>,
    Path(machine_id): Path<String>,
) -> Result<Json<proto::agent::Config>, AxumError> {
    // find or create target host
    _ = internal::upsert_host_with_machine_id(&state, &machine_id).await?;

    Ok(Json(proto::agent::Config {}))
}

/// Handles a report request for the given `machine_id`.
///
/// This function processes incoming JSON data representing a list of events
/// and sends each event to the eventbus associated with the specified
/// `machine_id`. The eventbus is created or retrieved via an internal
/// function. If an event cannot be deserialized, it is skipped.
///
/// # Errors
///
/// Returns an error if the eventbus cannot be created or if sending an event
/// to the eventbus fails.

pub async fn report(
    State(state): State<Arc<AppState>>,
    Path(machine_id): Path<String>,
    Json(values): Json<Vec<serde_json::Value>>,
) -> Result<(), AxumError> {
    // create event pipeline
    let tx = internal::eventbus_with_machine_id(state, &machine_id).await?;

    // dispatch all events
    for value in values {
        match serde_json::from_value(value) {
            Ok(event) => {
                tx.send(event).await?;
            }
            Err(err) => {
                tracing::warn!("deserialize event failed: {}", err);
            }
        }
    }

    Ok(())
}

/// Handles a WebSocket connection for the given `machine_id`.
///
/// This function upgrades an HTTP request to a WebSocket connection,
/// establishes an event pipeline, and continuously listens for incoming
/// WebSocket messages. Each message received is processed by the `handler`
/// function. If the handler encounters an error, the connection is
/// terminated.
pub async fn websocket(
    State(state): State<Arc<AppState>>,
    Path(machine_id): Path<String>,
    upgrade: WebSocketUpgrade,
) -> Result<impl IntoResponse, AxumError> {
    // create event pipeline
    let tx = internal::eventbus_with_machine_id(state, &machine_id).await?;

    Ok(upgrade.on_upgrade(move |mut ws| async move {
        // translate websocket message
        while let Some(Ok(message)) = ws.recv().await {
            if let Err(_) = handler(message, &mut ws, &tx).await {
                // something went wrong, disconnect connection
                break;
            }
        }
    }))
}

/// Handle an incoming websocket message.
///
/// This function translates the message into an `Events` and sends it to the
/// eventbus. If the message is a close message, it returns an error.
///
/// # Errors
///
/// Returns an error if the message is a close message or something went wrong.
async fn handler(
    message: Message,
    ws: &mut WebSocket,
    tx: &mpsc::Sender<Events>,
) -> Result<(), anyhow::Error> {
    match message {
        Message::Text(text) => {
            tracing::trace!("received text");

            match serde_json::from_slice(text.as_bytes()) {
                Ok(event) => {
                    tx.send(event).await?;
                }
                Err(err) => {
                    tracing::warn!("deserialize event failed: {}", err);
                }
            }
        }
        Message::Binary(data) => {
            tracing::trace!("received binary");

            match serde_json::from_slice(&data) {
                Ok(event) => {
                    tx.send(event).await?;
                }
                Err(err) => {
                    tracing::warn!("deserialize event failed: {}", err);
                }
            }
        }
        Message::Ping(data) => {
            tracing::trace!("received ping");

            ws.send(Message::Pong(data)).await?;
        }
        Message::Close(_) => {
            tracing::trace!("received close");

            return Err(anyhow!("close received"));
        }
        _ => {}
    }
    Ok(())
}

mod internal {
    use crate::prelude::seaorm::*;
    use crate::state::AppState;
    use anyhow::Result;
    use proto::agent::Events;
    use proto::agent::EvtMachineEmit;
    use proto::agent::EvtOsEmit;
    use sea_orm::IntoActiveValue;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    /// Finds the host with the given `machine_id` in the database and returns it. If the host
    /// does not exist, creates a new host with the given `machine_id` and returns it.
    pub async fn upsert_host_with_machine_id(
        state: &AppState,
        machine_id: &str,
    ) -> anyhow::Result<host::Model> {
        let exists = Host::find()
            .filter(host::Column::MachineId.eq(machine_id))
            .one(state.database.as_ref())
            .await?;

        if let Some(target) = exists {
            tracing::debug!(
                "found host with machine id: {} -> {}",
                machine_id,
                target.id
            );

            Ok(target)
        } else {
            let target = Host::insert(host::ActiveModel {
                id: Set(Uuid::from_bytes(uuidv7::create_raw())),
                machine_id: Set(machine_id.to_owned()),
                machine_ip: Set("".to_owned()),
                machine_country: Set("".to_owned()),
                machine_geo: Set("".to_owned()),
                os_family: Set("".to_owned()),
                os_name: Set("".to_owned()),
                os_version: Set("".to_owned()),
                os_arch: Set("".to_owned()),
                os_build: Set("".to_owned()),
                os_virtualization: Set(false),
                hashed_cpu: Set(0),
                hashed_gpu: Set(0),
                hashed_memory: Set(0),
                hashed_disk: Set(0),
                hashed_network: Set(0),
            })
            .exec_with_returning(state.database.as_ref())
            .await?;

            tracing::debug!(
                "created host with machine id: {} -> {}",
                machine_id,
                target.id
            );

            Ok(target)
        }
    }

    /// Finds the host with the given `machine_id` in the database and returns a mpsc eventbus
    /// sender which will send events to the host. If the host does not exist, creates a new host
    /// with the given `machine_id` and returns its eventbus sender.
    ///
    /// The eventbus sender returned by this function is connected to an eventbus receiver running
    /// in a separate task. Any events sent to the sender will be received by the receiver and
    /// processed.
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub async fn eventbus_with_machine_id(
        state: Arc<AppState>,
        machine_id: &str,
    ) -> Result<mpsc::Sender<proto::agent::Events>> {
        let target = upsert_host_with_machine_id(&state, machine_id).await?;

        // create tokio channel
        let (tx, mut rx) = mpsc::channel::<proto::agent::Events>(16);
        tokio::spawn({
            let state = state.clone();

            async move {
                while let Some(event) = rx.recv().await {
                    // received event from client
                    tracing::debug!("received event from {}: {:?}", &target.machine_id, &event);

                    // dispatch to handler
                    if let Err(err) = eventbus_handler(&state, &target, event).await {
                        tracing::warn!("eventbus handler failed: {}", err);
                    };
                }
            }
        });

        Ok(tx)
    }

    /// Handles an `Events` enum by dispatching it to the appropriate handler.
    ///
    /// This function takes an `event` of type `Events` and matches it to call
    /// the corresponding event handler function.
    ///
    /// # Errors
    ///
    /// Returns an error if the event handling fails, which could be due to
    /// database operation errors.
    async fn eventbus_handler(state: &AppState, target: &host::Model, event: Events) -> Result<()> {
        match event {
            Events::EvtMachineEmit(machine) => {
                eventbus_handle_machine_emit(state, target, machine).await?;
            }
            Events::EvtOsEmit(os) => {
                eventbus_handle_os_emit(state, target, os).await?;
            }
        }
        Ok(())
    }

    /// Handles a `EvtMachineEmit` event sent to the eventbus.
    ///
    /// This function updates the `machine_*` fields of the host.
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    async fn eventbus_handle_machine_emit(
        state: &AppState,
        target: &host::Model,
        machine: EvtMachineEmit,
    ) -> Result<()> {
        Host::update(host::ActiveModel {
            id: target.id.into_active_value(),
            machine_ip: machine.ip.into_active_value(),
            machine_country: machine.country.into_active_value_(),
            ..Default::default()
        })
        .exec(state.database.as_ref())
        .await?;

        Ok(())
    }

    /// Handles an `EvtOsEmit` event sent to the eventbus.
    ///
    /// This function updates the `os_*` fields of the host.
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    async fn eventbus_handle_os_emit(
        state: &AppState,
        target: &host::Model,
        os: EvtOsEmit,
    ) -> Result<()> {
        Host::update(host::ActiveModel {
            id: target.id.into_active_value(),
            os_family: os.family.into_active_value(),
            os_name: os.name.into_active_value_(),
            os_version: os.version.into_active_value_(),
            os_arch: os.arch.into_active_value_(),
            os_build: os.build.into_active_value_(),
            os_virtualization: os.virtualization.into_active_value_(),
            ..Default::default()
        })
        .exec(state.database.as_ref())
        .await?;

        Ok(())
    }
}
