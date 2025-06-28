use serde::Serialize;
use sqlx::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize)]
#[sqlx(type_name = "period_type", rename_all = "snake_case")]
pub enum PeriodType {
    Regulation,
    Overtime,
    Shootout,
}
impl PeriodType {
    pub fn from_str(s: &str) -> Option<PeriodType> {
        match s {
            "REG" => Some(crate::models::period_type::PeriodType::Regulation),
            "OT" => Some(crate::models::period_type::PeriodType::Overtime),
            "SO" => Some(crate::models::period_type::PeriodType::Shootout),
            s => {
                tracing::warn!("Encountered unexpected value {s} when accessing `periodType`");
                None
            }
        }
    }
}
