use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdoptionSignal {
    pub signal_id: String,
    pub country_code: String,
    pub provider: String,
    pub model_or_service: String,
    pub user_segment: String,
    pub signal_type: String,
    pub value_text: String,
    pub signal_date: String,
    pub confidence: String,
    pub source_notes: String,
}
