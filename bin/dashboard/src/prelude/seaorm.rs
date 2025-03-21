pub use database::models::prelude::*;
pub use database::models::*;
pub use sea_orm::prelude::*;
pub use sea_orm::ActiveValue::*;
pub use sea_orm::EntityTrait;
pub use sea_orm::QueryFilter;

pub trait IntoActiveValueExt<V>
where
    V: Into<Value>,
{
    fn into_active_value_(self) -> sea_orm::ActiveValue<V>;
}

impl<V> IntoActiveValueExt<V> for Option<V>
where
    V: Into<Value>,
{
    fn into_active_value_(self) -> sea_orm::ActiveValue<V> {
        match self {
            Some(value) => Set(value),
            None => NotSet,
        }
    }
}
