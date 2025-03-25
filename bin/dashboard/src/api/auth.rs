use crate::prelude::axum::*;
use crate::state::AppState;
use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use proto::auth::captcha::CaptchaGenerateReq;
use proto::auth::captcha::CaptchaGenerateResp;
use std::sync::Arc;

/// Generates a new captcha image.
///
/// This endpoint generates a new captcha image and returns
/// its base64 encoding and the captcha's ID.
///
/// The response is a JSON object with the following fields:
///
/// - `id`: The ID of the captcha.
/// - `base64`: The base64 encoding of the captcha image.
///
/// The image is a PNG image with a width and height of 220x120 pixels.
/// The image contains 4 random characters.
pub async fn captcha(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CaptchaGenerateReq>,
) -> Result<Json<CaptchaGenerateResp>, AxumError> {
    // polyfill width and height
    let width = query.w.unwrap_or(220);
    let height = query.h.unwrap_or(120);

    // generate captcha
    let (id, base64) = internal::captcha_generate(&state, width, height).await?;

    Ok(Json(CaptchaGenerateResp { id, base64 }))
}

mod internal {
    use crate::state::AppState;
    use anyhow::anyhow;
    use anyhow::Result;
    use captcha::filters::Noise;
    use captcha::Captcha;
    use database::models::captcha as captcha_;
    use database::models::captcha::Entity as Captcha_;
    use sea_orm::prelude::*;
    use sea_orm::IntoActiveModel;
    use std::str::FromStr;

    /// Generates a new captcha image and persists it in the database.
    ///
    /// This function generates a new captcha image and persists it in the database.
    /// The image is a PNG image with a width and height of the given parameters.
    /// The image contains 4 random characters.
    ///
    /// The response is a tuple of two strings. The first element is the ID of the captcha.
    /// The second element is the base64 encoding of the captcha image.
    pub async fn captcha_generate(
        state: &AppState,
        width: u32,
        height: u32,
    ) -> Result<(String, String)> {
        // generate captcha (Captcha is not Send + Sync, so we need generate it in closure)
        let (answer, base64) = {
            let mut captcha = Captcha::new();
            captcha.add_chars(4);
            captcha.view(width, height);
            captcha.apply_filter(Noise::new(0.1));

            let answer = captcha.chars_as_string();
            let base64 = captcha.as_base64();

            (answer, base64.ok_or(anyhow!("captcha generate failed"))?)
        };

        // storage captcha in database
        let persisted = Captcha_::insert(
            captcha_::Model {
                id: Uuid::from_bytes(uuidv7::create_raw()),
                answer: answer,
                expired_at: chrono::Utc::now(),
            }
            .into_active_model(),
        )
        .exec_with_returning(state.database.as_ref())
        .await?;

        Ok((
            format!("{}", persisted.id),
            format!("data:image/png;base64,{}", base64),
        ))
    }

    /// Verifies the given captcha `id` and `answer`.
    ///
    /// This function loads the captcha from the database, checks if it is expired,
    /// deletes it from the database, and compares the answer. If the answer is
    /// invalid or the captcha does not exist, an error is returned.
    ///
    /// # Errors
    ///
    /// Returns an error if the captcha is invalid.
    #[allow(dead_code)]
    pub async fn captcha_verify(state: &AppState, id: &str, answer: &str) -> Result<()> {
        // load captcha from database
        let found = Captcha_::find()
            .filter(captcha_::Column::Id.eq(Uuid::from_str(id)?))
            .filter(captcha_::Column::ExpiredAt.lt(chrono::Utc::now()))
            .one(state.database.as_ref())
            .await?;

        // delete captcha from database
        if let Some(found) = found.clone() {
            Captcha_::delete_by_id(found.id)
                .exec(state.database.as_ref())
                .await?;
        }

        // compare answer
        if found.is_none() || found.unwrap().answer != answer {
            return Err(anyhow!("invalid captcha"));
        }

        Ok(())
    }
}
