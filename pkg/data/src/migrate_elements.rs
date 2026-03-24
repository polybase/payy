#[cfg(feature = "diesel")]
use database::schema::migrate_elements;
#[cfg(feature = "diesel")]
use diesel::{
    Selectable,
    prelude::{Insertable, Queryable},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "diesel", derive(Insertable, Queryable, Selectable))]
#[cfg_attr(feature = "diesel", diesel(primary_key(element)))]
#[cfg_attr(feature = "diesel", diesel(table_name = migrate_elements))]
#[cfg_attr(feature = "diesel", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct MigrateElement {
    pub element: String,
}
