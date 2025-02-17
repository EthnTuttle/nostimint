use std::fmt;
use std::str;

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

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, Encodable, Decodable)]
pub struct Nonce {
    value: u64,
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub event: nostr_sdk::Event,
}

impl AsRef<[u8]> for Event {
    fn as_ref(&self) -> &[u8] {
        self.event.id.as_bytes()
    }
}

impl Decodable for Event {
    fn consensus_decode<R: std::io::Read>(
        r: &mut R,
        modules: &fedimint_core::module::registry::ModuleDecoderRegistry,
    ) -> Result<Self, fedimint_core::encoding::DecodeError> {
        let bytes = Vec::<u8>::consensus_decode(r, modules)?;
        let json = String::from_utf8(bytes).unwrap();
        let event = nostr_sdk::Event::from_json(json).unwrap();
        Ok(Event { event })
    }
}

impl Encodable for Event {
    fn consensus_encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        self.event.as_json().as_bytes().consensus_encode(writer)
    }
}

/// Non-transaction items that will be submitted to consensus
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Encodable, Decodable)]
pub enum NostimintConsensusItem {
    /// User's message sign request signed by a single peer
    Note(Event, SerdeSignatureShare), // Nonce here eventually
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
