use fedimint_core::db::DatabaseTransaction;
use fedimint_core::encoding::{Decodable, Encodable};
use fedimint_core::epoch::SerdeSignatureShare;
use fedimint_core::{impl_db_lookup, impl_db_record, Amount, OutPoint, PeerId};
use fedimint_nostimint_common::Event;
use futures::StreamExt;
use nostr_sdk::EventId;
use secp256k1::XOnlyPublicKey;
use serde::Serialize;
use strum_macros::EnumIter;

use crate::NostimintOutputOutcome;

/// Namespaces DB keys for this module
#[repr(u8)]
#[derive(Clone, EnumIter, Debug)]
pub enum DbKeyPrefix {
    Funds = 0x01,
    Outcome = 0x02,
    SignatureShare = 0x03,
    Event = 0x04,
}

// TODO: Boilerplate-code
impl std::fmt::Display for DbKeyPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Example old version 0 of DB entries
// TODO: can we simplify this by just using macros?
#[derive(Debug, Clone, Encodable, Decodable, Eq, PartialEq, Hash, Serialize)]
pub struct NostimintFundsKeyV0(pub XOnlyPublicKey);

#[derive(Debug, Encodable, Decodable)]
pub struct NostimintFundsKeyPrefixV0;

impl_db_record!(
    key = NostimintFundsKeyV0,
    value = (),
    db_prefix = DbKeyPrefix::Funds,
);
impl_db_lookup!(
    key = NostimintFundsKeyV0,
    query_prefix = NostimintFundsKeyPrefixV0
);

/// Lookup funds for a user by key or prefix
#[derive(Debug, Clone, Encodable, Decodable, Eq, PartialEq, Hash, Serialize)]
pub struct NostimintFundsKeyV1(pub XOnlyPublicKey);

#[derive(Debug, Encodable, Decodable)]
pub struct NostimintFundsPrefixV1;

impl_db_record!(
    key = NostimintFundsKeyV1,
    value = Amount,
    db_prefix = DbKeyPrefix::Funds,
);
impl_db_lookup!(
    key = NostimintFundsKeyV1,
    query_prefix = NostimintFundsPrefixV1
);

/// Example DB migration from version 0 to version 1
pub async fn migrate_to_v1(dbtx: &mut DatabaseTransaction<'_>) -> Result<(), anyhow::Error> {
    // Select old entries
    let v0_entries = dbtx
        .find_by_prefix(&NostimintFundsKeyPrefixV0)
        .await
        .collect::<Vec<(NostimintFundsKeyV0, ())>>()
        .await;

    // Remove old entries
    dbtx.remove_by_prefix(&NostimintFundsKeyPrefixV0).await;

    // Migrate to new entries
    for (v0_key, _v0_val) in v0_entries {
        let v1_key = NostimintFundsKeyV1(v0_key.0);
        dbtx.insert_new_entry(&v1_key, &Amount::ZERO).await;
    }
    Ok(())
}

/// Lookup tx outputs by key or prefix
#[derive(Debug, Clone, Encodable, Decodable, Eq, PartialEq, Hash, Serialize)]
pub struct NostimintOutcomeKey(pub OutPoint);

#[derive(Debug, Encodable, Decodable)]
pub struct NostimintOutcomePrefix;

impl_db_record!(
    key = NostimintOutcomeKey,
    value = NostimintOutputOutcome,
    db_prefix = DbKeyPrefix::Outcome,
);
impl_db_lookup!(
    key = NostimintOutcomeKey,
    query_prefix = NostimintOutcomePrefix
);

/// Lookup signature requests by key or prefix
#[derive(Debug, Clone, Encodable, Decodable, Eq, PartialEq, Hash, Serialize)]
pub struct NostimintSignatureShareKey(pub Event, pub PeerId);

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize)]
pub struct NostimintSignatureShareStringPrefix(pub EventId);

impl Encodable for NostimintSignatureShareStringPrefix {
    fn consensus_encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        self.0.as_bytes().consensus_encode(writer)
    }
}

#[derive(Debug, Clone, Encodable, Decodable, Eq, PartialEq, Hash, Serialize)]
pub struct NostimintSignatureSharePrefix;

impl_db_record!(
    key = NostimintSignatureShareKey,
    value = SerdeSignatureShare,
    db_prefix = DbKeyPrefix::SignatureShare,
);
impl_db_lookup!(
    key = NostimintSignatureShareKey,
    query_prefix = NostimintSignatureShareStringPrefix,
    query_prefix = NostimintSignatureSharePrefix
);

/// Lookup signature requests by key or prefix
#[derive(Debug, Clone, Encodable, Decodable, Eq, PartialEq, Hash, Serialize)]
pub struct NostimintKind1Key(pub Event);

#[derive(Debug, Encodable, Decodable)]
pub struct NostimintKind1Prefix;

impl_db_record!(
    key = NostimintKind1Key,
    value = Option<Event>,
    db_prefix = DbKeyPrefix::Event,
    // Allows us to listen for notifications on this key
    notify_on_modify = true
);
impl_db_lookup!(key = NostimintKind1Key, query_prefix = NostimintKind1Prefix);
