use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deal {
    pub deal_id: String,
    pub country_code: String,
    pub partner_type: String,
    pub partner_name: String,
    pub deal_date: String,
    pub deal_type: String,
    pub description: String,
    pub stack_layer: String,
    pub confidence: String,
    pub source_notes: String,
}
