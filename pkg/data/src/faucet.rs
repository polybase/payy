#[cfg(feature = "diesel")]
use database::schema::faucets;
#[cfg(feature = "diesel")]
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "diesel",
    derive(Queryable, Selectable, Identifiable, AsChangeset)
)]
#[cfg_attr(feature = "diesel", diesel(table_name = faucets))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct Faucet {
    pub id: Uuid,
    pub url: String,
    pub claimed_by: Option<String>,
    pub added_at: chrono::NaiveDateTime,
    pub claimed_at: Option<chrono::NaiveDateTime>,
}

#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = faucets))]
pub struct NewFaucet<'a> {
    pub url: &'a str,
}
