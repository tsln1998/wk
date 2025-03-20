use crate::args::Args;
use anyhow::Ok;
use anyhow::Result;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct AppState {
    pub args: Args,
    pub database: Arc<DatabaseConnection>,
}

impl AppState {
    pub fn new(args: Args, database: DatabaseConnection) -> Self {
        Self {
            args: args,
            database: Arc::new(database),
        }
    }

    pub async fn close(&self) -> Result<()> {
        self.database.close_by_ref().await?;
        Ok(())
    }
}
