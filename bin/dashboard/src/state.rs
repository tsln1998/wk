use crate::args::Args;
use anyhow::Ok;
use anyhow::Result;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::EncodingKey;
use jsonwebtoken::Header;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub jwt: AppStateJwtSecret,
    pub database: Arc<DatabaseConnection>,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct AppStateJwtSecret {
    pub header: jsonwebtoken::Header,
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl AppState {
    pub fn new(args: Args, database: DatabaseConnection) -> Self {
        let jwt = {
            let secret: Vec<u8> = args
                .secret
                .map_or_else(|| vec![0u8], |v| v.as_bytes().to_vec());

            AppStateJwtSecret {
                header: Header::new(jsonwebtoken::Algorithm::HS512),
                encoding: EncodingKey::from_secret(&secret),
                decoding: DecodingKey::from_secret(&secret),
            }
        };

        Self {
            jwt: jwt,
            database: Arc::new(database),
        }
    }

    pub async fn close(&self) -> Result<()> {
        self.database.close_by_ref().await?;
        Ok(())
    }
}
