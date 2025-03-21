use crate::prelude::axum::*;
use crate::state::AppState;
use anyhow::anyhow;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use axum::Json;
use proto::agent::Events;
use tokio::sync::mpsc;

/// Finds the host with the given `machine_id` in the database and returns its
/// configuration. If the host does not exist, creates a new host with the given
/// `machine_id` and returns its configuration.
///
/// # Errors
///
/// Returns an error if database operations fail.
pub async fn config(
    State(state): State<AppState>,
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
    State(state): State<AppState>,
    Path(machine_id): Path<String>,
    Json(values): Json<Vec<serde_json::Value>>,
) -> Result<(), AxumError> {
    // create event pipeline
    let tx = internal::eventbus_with_machine_id(&state, &machine_id).await?;

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
    State(state): State<AppState>,
    Path(machine_id): Path<String>,
    upgrade: WebSocketUpgrade,
) -> Result<impl IntoResponse, AxumError> {
    // create event pipeline
    let tx = internal::eventbus_with_machine_id(&state, &machine_id).await?;

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
    use sea_orm::IntoActiveValue;
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
        state: &AppState,
        machine_id: &str,
    ) -> Result<mpsc::Sender<proto::agent::Events>> {
        let state = state.clone();
        let machine_id = machine_id.to_owned();

        let target = upsert_host_with_machine_id(&state, &machine_id).await?;

        // create tokio channel
        let (tx, mut rx) = mpsc::channel::<proto::agent::Events>(16);
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                // received event from client
                tracing::debug!("received event from {}: {:?}", &target.machine_id, &event);

                // dispatch to handler
                if let Err(err) = eventbus_handler(&state, &target, event).await {
                    tracing::warn!("eventbus handler failed: {}", err);
                };
            }
        });

        Ok(tx)
    }

    /// Handles events sent to the eventbus.
    ///
    /// This function is responsible for processing events sent to the eventbus. It currently does
    /// nothing, but it may do so in the future.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be processed.
    pub async fn eventbus_handler(
        state: &AppState,
        target: &host::Model,
        event: Events,
    ) -> Result<()> {
        match event {
            Events::EvtMachineEmit(machine) => {
                Host::update(host::ActiveModel {
                    id: Set(target.id),
                    machine_ip: Set(machine.ip),
                    machine_country: if let Some(country) = machine.country {
                        Set(country)
                    } else {
                        NotSet
                    },
                    ..Default::default()
                })
                .exec(state.database.as_ref())
                .await?;
            }
            Events::EvtOsEmit(os) => {
                Host::update(host::ActiveModel {
                    id: Set(target.id),
                    os_family: os.family.into_active_value(),
                    os_name: if let Some(name) = os.name {
                        Set(name)
                    } else {
                        NotSet
                    },
                    os_version: if let Some(version) = os.version {
                        Set(version)
                    } else {
                        NotSet
                    },
                    os_arch: if let Some(arch) = os.arch {
                        Set(arch)
                    } else {
                        NotSet
                    },
                    os_build: if let Some(build) = os.build {
                        Set(build)
                    } else {
                        NotSet
                    },
                    os_virtualization: if let Some(virtualization) = os.virtualization {
                        Set(virtualization)
                    } else {
                        NotSet
                    },
                    ..Default::default()
                })
                .exec(state.database.as_ref())
                .await?;
            }
        }
        Ok(())
    }
}
