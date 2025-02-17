use fedimint_core::core::ModuleKind;
use fedimint_core::encoding::{Decodable, Encodable};
use fedimint_core::{plugin_types_trait_impl_config, Amount};
use serde::{Deserialize, Serialize};
use threshold_crypto::serde_impl::SerdeSecret;
use threshold_crypto::{PublicKey, PublicKeySet, SecretKeyShare};

use crate::NostimintCommonGen;

/// Parameters necessary to generate this module's configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NostimintGenParams {
    pub local: NostimintGenParamsLocal,
    pub consensus: NostimintGenParamsConsensus,
}

/// Local parameters for config generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NostimintGenParamsLocal(pub String);

/// Consensus parameters for config generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NostimintGenParamsConsensus {
    pub tx_fee: Amount,
}

impl Default for NostimintGenParams {
    fn default() -> Self {
        Self {
            local: NostimintGenParamsLocal("example".to_string()),
            consensus: NostimintGenParamsConsensus {
                tx_fee: Amount::ZERO,
            },
        }
    }
}

/// Contains all the configuration for the server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NostimintConfig {
    pub local: NostimintConfigLocal,
    pub private: NostimintConfigPrivate,
    pub consensus: NostimintConfigConsensus,
}

/// Contains all the configuration for the client
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Encodable, Decodable, Hash)]
pub struct NostimintClientConfig {
    /// Accessible to clients
    pub tx_fee: Amount,
    pub fed_public_key: PublicKey,
}

/// Locally unencrypted config unique to each member
#[derive(Clone, Debug, Serialize, Deserialize, Decodable, Encodable)]
pub struct NostimintConfigLocal {
    pub example: String,
}

/// Will be the same for every federation member
#[derive(Clone, Debug, Serialize, Deserialize, Decodable, Encodable)]
pub struct NostimintConfigConsensus {
    /// Example federation threshold signing key
    pub public_key_set: PublicKeySet,
    /// Will be the same for all peers
    pub tx_fee: Amount,
}

/// Will be encrypted and not shared such as private key material
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NostimintConfigPrivate {
    /// Example private key share for a single member
    pub private_key_share: SerdeSecret<SecretKeyShare>,
}

// Wire together the configs for this module
plugin_types_trait_impl_config!(
    NostimintCommonGen,
    NostimintGenParams,
    NostimintGenParamsLocal,
    NostimintGenParamsConsensus,
    NostimintConfig,
    NostimintConfigLocal,
    NostimintConfigPrivate,
    NostimintConfigConsensus,
    NostimintClientConfig
);
