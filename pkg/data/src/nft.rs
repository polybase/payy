#[cfg(feature = "diesel")]
use database::schema::nfts;
#[cfg(feature = "diesel")]
use diesel::{
    Selectable,
    prelude::{Insertable, Queryable},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "diesel", derive(Insertable, Queryable, Selectable))]
#[cfg_attr(feature = "diesel", diesel(primary_key(id)))]
#[cfg_attr(feature = "diesel", diesel(table_name = nfts))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct Nft {
    pub id: Uuid,
    pub url: String,
    pub price: i32,
}
