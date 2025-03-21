use crate::prelude::axum::*;
use crate::state::AppState;

pub async fn config(
    State(state): State<AppState>,
    Path(machine_id): Path<String>,
) -> Result<(), AxumError> {
    // find or create target host
    _ = internal::upsert_host_with_machine_id(&state.database, &machine_id).await?;

    Result::<(), AxumError>::Ok(())
}

mod internal {
    use crate::prelude::seaorm::*;

    /// Finds the host with the given `machine_id` in the database and returns it. If the host
    /// does not exist, creates a new host with the given `machine_id` and returns it.
    pub async fn upsert_host_with_machine_id(
        database: &DatabaseConnection,
        machine_id: &str,
    ) -> anyhow::Result<host::Model> {
        let exists = Host::find()
            .filter(host::Column::MachineId.eq(machine_id))
            .one(database)
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
            .exec_with_returning(database)
            .await?;

            tracing::debug!(
                "created host with machine id: {} -> {}",
                machine_id,
                target.id
            );

            Ok(target)
        }
    }
}
