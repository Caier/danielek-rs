#![allow(non_camel_case_types, unused)]

use derive_builder::Builder;
use serde::{Deserialize, Serialize};

use crate::dapi::routes::common_types::{IntOrStr, Snowflake};

pub type GiftRedeemSuccess = serde_json::Value; //why would I even deserialize this
pub type GiftInfo = serde_json::Value;

#[derive(Default, Clone, Serialize)]
pub struct EntitlementRedeemBody {
    pub channel_id: Option<bool>,
    pub payment_source_id: Option<bool>,
}

// pub struct SubscriptionPlan { //undocumented
//     pub id: Snowflake,
//     pub name: String,
//     pub interval: i32,
//     pub interval_count: i32,
//     pub tax_inclusive: bool,
//     pub sku_id: Snowflake,
//     pub currency: String,
//     pub price: i64,
//     //pub price_tier: null, no idea
// }

// pub struct Sku { //undocumented
//     pub id: Snowflake,
//     pub r#type: u32,
//     //pub dependent_sku_id: null no ide
//     pub application_id: String,
//     //pub manifest_labels: null, no idea
//     pub access_type: i32
//     pub name: String,
//     pub features: Vec<???>,
//     pub release_date: ????,
//     pub premium: bool,
//     pub slug: String,
//     pub flags: u64,
//     pub show_age_gate: bool
// }

// pub struct GiftRedeemSuccess { //undocumented type hhh
//     pub id: Snowflake,
//     pub sku_id: Snowflake,
//     pub application_id: Snowflake,
//     pub user_id: Snowflake,
//     pub promotion_id: Option<Snowflake>, //no idea
//     pub r#type: u32,
//     pub deleted: bool,
//     pub gift_code_flags: u32,
//     pub consumed: bool,
//     pub gifter_user_id: Snowflake,
//     pub subscription_plan: Option<SubscriptionPlan>,
//     pub sku: Option<Sku>
// }
