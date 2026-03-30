use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, Clone, Type, PartialEq)]
#[sqlx(type_name = "job_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReminderJobStatus {
    Pending,
    Sent,
}

impl Display for ReminderJobStatus {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ReminderJobStatus::Pending => write!(f, "PENDING"),
            ReminderJobStatus::Sent => write!(f, "SENT"),
        }
    }
}

impl FromStr for ReminderJobStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PENDING" => Ok(ReminderJobStatus::Pending),
            "SENT" => Ok(ReminderJobStatus::Sent),
            _ => Err(format!("Unknown status: {}", s)),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct ReminderJob {
    pub id: Uuid,
    pub task_id: Uuid,
    pub remind_at: DateTime<Utc>,
    pub status: ReminderJobStatus,
    pub created_at: DateTime<Utc>,
}
