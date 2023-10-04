use std::fmt;

use config::NostimintClientConfig;
use fedimint_core::core::{Decoder, ModuleInstanceId, ModuleKind};
use fedimint_core::encoding::{Decodable, Encodable};
use fedimint_core::epoch::SerdeSignatureShare;
use fedimint_core::module::{CommonModuleInit, ModuleCommon, ModuleConsensusVersion};
use fedimint_core::{plugin_types_trait_impl_common, Amount};
use secp256k1::{KeyPair, Secp256k1, XOnlyPublicKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Common contains types shared by both the client and server

// The client and server configuration
pub mod config;

/// Unique name for this module
pub const KIND: ModuleKind = ModuleKind::from_static_str("nostimint");

/// Modules are non-compatible with older versions
pub const CONSENSUS_VERSION: ModuleConsensusVersion = ModuleConsensusVersion(0);

/// Non-transaction items that will be submitted to consensus
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Encodable, Decodable)]
pub enum NostimintConsensusItem {
    /// User's message sign request signed by a single peer
    Sign(String, SerdeSignatureShare),
}

/// Input for a fedimint transaction
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize, Encodable, Decodable)]
pub struct NostimintInput {
    pub amount: Amount,
    /// Associate the input with a user's pubkey
    pub account: XOnlyPublicKey,
}

/// Output for a fedimint transaction
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize, Encodable, Decodable)]
pub struct NostimintOutput {
    pub amount: Amount,
    /// Associate the output with a user's pubkey
    pub account: XOnlyPublicKey,
}

/// Information needed by a client to update output funds
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize, Encodable, Decodable)]
pub struct NostimintOutputOutcome(pub Amount, pub XOnlyPublicKey);

/// Errors that might be returned by the server
// TODO: Move to server lib?
#[derive(Debug, Clone, Eq, PartialEq, Hash, Error)]
pub enum NostimintError {
    #[error("Not enough funds")]
    NotEnoughFunds,
}

/// Contains the types defined above
pub struct NostimintModuleTypes;

// Wire together the types for this module
plugin_types_trait_impl_common!(
    NostimintModuleTypes,
    NostimintClientConfig,
    NostimintInput,
    NostimintOutput,
    NostimintOutputOutcome,
    NostimintConsensusItem
);

#[derive(Debug)]
pub struct NostimintCommonGen;

impl CommonModuleInit for NostimintCommonGen {
    const CONSENSUS_VERSION: ModuleConsensusVersion = CONSENSUS_VERSION;
    const KIND: ModuleKind = KIND;

    type ClientConfig = NostimintClientConfig;

    fn decoder() -> Decoder {
        NostimintModuleTypes::decoder_builder().build()
    }
}

impl fmt::Display for NostimintClientConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NostimintClientConfig")
    }
}
impl fmt::Display for NostimintInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NostimintInput {}", self.amount)
    }
}

impl fmt::Display for NostimintOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NostimintOutput {}", self.amount)
    }
}

impl fmt::Display for NostimintOutputOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NostimintOutputOutcome")
    }
}

impl fmt::Display for NostimintConsensusItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NostimintConsensusItem")
    }
}

/// A special key that creates assets for a test/example
const FED_SECRET_PHRASE: &str = "Money printer go brrr...........";

pub fn fed_public_key() -> XOnlyPublicKey {
    fed_key_pair().x_only_public_key().0
}

pub fn fed_key_pair() -> KeyPair {
    KeyPair::from_seckey_slice(&Secp256k1::new(), FED_SECRET_PHRASE.as_bytes()).expect("32 bytes")
}
