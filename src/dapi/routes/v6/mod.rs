use crate::dapi::{
    types::{dapi_endpoint, DApiGET, DApiPOST},
    versions::v6,
};

use self::types::{EntitlementRedeemBody, GiftInfo, GiftRedeemSuccess};

pub mod types;

dapi_endpoint! {
    version = v6,
    DApiPOST = (GiftRedeemSuccess, EntitlementRedeemBody);

    pub fn entitlements_giftcode_redeem(gift_code: impl AsRef<str>) {
        format!("/entitlements/gift-codes/{}/redeem", gift_code.as_ref())
    }
}

dapi_endpoint! {
    version = v6,
    DApiGET = (GiftInfo);

    pub fn entitlements_giftcode(gift_code: impl AsRef<str>) {
        format!("/entitlements/gift-codes/{}", gift_code.as_ref())
    }
}
