use std::collections::BTreeMap;
use std::env;
use std::string::ToString;

use nostr_sdk::prelude::FromSkStr;
use nostr_sdk::{EventId, Keys, ToBech32};

use anyhow::bail;
use async_trait::async_trait;
use fedimint_core::config::{
    ConfigGenModuleParams, DkgResult, ServerModuleConfig, ServerModuleConsensusConfig,
    TypedServerModuleConfig, TypedServerModuleConsensusConfig,
};
use fedimint_core::db::{Database, DatabaseVersion, MigrationMap, ModuleDatabaseTransaction};
use fedimint_core::epoch::SerdeSignatureShare;
use fedimint_core::module::audit::Audit;
use fedimint_core::module::{
    api_endpoint, ApiEndpoint, ConsensusProposal, CoreConsensusVersion, ExtendsCommonModuleInit,
    InputMeta, IntoModuleError, ModuleConsensusVersion, ModuleError, PeerHandle, ServerModuleInit,
    SupportedModuleApiVersions, TransactionItemAmount,
};
use fedimint_core::server::DynServerModule;
use fedimint_core::task::TaskGroup;
use fedimint_core::{push_db_pair_items, Amount, OutPoint, PeerId, ServerModule};
pub use fedimint_nostimint_common::config::{
    NostimintClientConfig, NostimintConfig, NostimintConfigConsensus, NostimintConfigLocal,
    NostimintConfigPrivate, NostimintGenParams,
};
use fedimint_nostimint_common::Event;
pub use fedimint_nostimint_common::{
    fed_public_key, NostimintCommonGen, NostimintConsensusItem, NostimintError, NostimintInput,
    NostimintModuleTypes, NostimintOutput, NostimintOutputOutcome, CONSENSUS_VERSION, KIND,
};
use fedimint_server::config::distributedgen::PeerHandleOps;
use futures::{FutureExt, StreamExt};
use strum::IntoEnumIterator;
use tokio::sync::Notify;

use crate::db::{
    migrate_to_v1, DbKeyPrefix, NostimintFundsKeyV1, NostimintFundsPrefixV1, NostimintKind1Key,
    NostimintKind1Prefix, NostimintOutcomeKey, NostimintOutcomePrefix, NostimintSignatureShareKey,
    NostimintSignatureSharePrefix, NostimintSignatureShareStringPrefix,
};

mod db;

/// Generates the module
#[derive(Debug, Clone)]
pub struct NostimintGen;

// TODO: Boilerplate-code
impl ExtendsCommonModuleInit for NostimintGen {
    type Common = NostimintCommonGen;
}

/// Implementation of server module non-consensus functions
#[async_trait]
impl ServerModuleInit for NostimintGen {
    type Params = NostimintGenParams;
    const DATABASE_VERSION: DatabaseVersion = DatabaseVersion(1);

    /// Returns the version of this module
    fn versions(&self, _core: CoreConsensusVersion) -> &[ModuleConsensusVersion] {
        &[CONSENSUS_VERSION]
    }

    fn supported_api_versions(&self) -> SupportedModuleApiVersions {
        SupportedModuleApiVersions::from_raw(1, 0, &[(0, 0)])
    }

    /// Initialize the module
    async fn init(
        &self,
        cfg: ServerModuleConfig,
        _db: Database,
        _task_group: &mut TaskGroup,
    ) -> anyhow::Result<DynServerModule> {
        Ok(Nostimint::new(cfg.to_typed()?).into())
    }

    /// DB migrations to move from old to newer versions
    fn get_database_migrations(&self) -> MigrationMap {
        let mut migrations = MigrationMap::new();
        migrations.insert(DatabaseVersion(0), move |dbtx| migrate_to_v1(dbtx).boxed());
        migrations
    }

    /// Generates configs for all peers in a trusted manner for testing
    fn trusted_dealer_gen(
        &self,
        _peers: &[PeerId],
        _params: &ConfigGenModuleParams,
    ) -> BTreeMap<PeerId, ServerModuleConfig> {
        // let params = self.parse_params(params).unwrap();
        // // Create trusted set of threshold keys
        // let sks = SecretKeySet::random(peers.degree(), &mut OsRng);
        // let pks: PublicKeySet = sks.public_keys();
        // // Generate a config for each peer
        // peers
        //     .iter()
        //     .map(|&peer| {
        //         let private_key_share = SerdeSecret(sks.secret_key_share(peer.to_usize()));
        //         let config = NostimintConfig {
        //             local: NostimintConfigLocal {
        //                 example: params.local.0.clone(),
        //             },
        //             private: NostimintConfigPrivate { private_key_share },
        //             consensus: NostimintConfigConsensus {
        //                 public_key_set: pks.clone(),
        //                 tx_fee: params.consensus.tx_fee,
        //             },
        //         };
        //         (peer, config.to_erased())
        //     })
        //     .collect()
        BTreeMap::new()
    }

    /// Generates configs for all peers in an untrusted manner
    async fn distributed_gen(
        &self,
        peers: &PeerHandle,
        params: &ConfigGenModuleParams,
    ) -> DkgResult<ServerModuleConfig> {
        let params = self.parse_params(params).unwrap();
        // Runs distributed key generation
        // Could create multiple keys, here we use '()' to create one
        let g1 = peers.run_dkg_g1(()).await?;
        let keys = g1[&()].threshold_crypto();

        Ok(NostimintConfig {
            local: NostimintConfigLocal {
                example: params.local.0.clone(),
            },
            private: NostimintConfigPrivate {
                private_key_share: keys.secret_key_share,
            },
            consensus: NostimintConfigConsensus {
                public_key_set: keys.public_key_set,
                tx_fee: params.consensus.tx_fee,
            },
        }
        .to_erased())
    }

    /// Converts the consensus config into the client config
    fn get_client_config(
        &self,
        config: &ServerModuleConsensusConfig,
    ) -> anyhow::Result<NostimintClientConfig> {
        let config = NostimintConfigConsensus::from_erased(config)?;
        Ok(NostimintClientConfig {
            tx_fee: config.tx_fee,
            fed_public_key: config.public_key_set.public_key(),
        })
    }

    /// Validates the private/public key of configs
    fn validate_config(&self, identity: &PeerId, config: ServerModuleConfig) -> anyhow::Result<()> {
        let config = config.to_typed::<NostimintConfig>()?;
        let our_id = identity.to_usize();
        let our_share = config.consensus.public_key_set.public_key_share(our_id);

        // Check our private key matches our public key share
        if config.private.private_key_share.public_key_share() != our_share {
            bail!("Private key doesn't match public key share");
        }
        Ok(())
    }

    /// Dumps all database items for debugging
    async fn dump_database(
        &self,
        dbtx: &mut ModuleDatabaseTransaction<'_>,
        prefix_names: Vec<String>,
    ) -> Box<dyn Iterator<Item = (String, Box<dyn erased_serde::Serialize + Send>)> + '_> {
        // TODO: Boilerplate-code
        let mut items: BTreeMap<String, Box<dyn erased_serde::Serialize + Send>> = BTreeMap::new();
        let filtered_prefixes = DbKeyPrefix::iter().filter(|f| {
            prefix_names.is_empty() || prefix_names.contains(&f.to_string().to_lowercase())
        });

        for table in filtered_prefixes {
            match table {
                DbKeyPrefix::Funds => {
                    push_db_pair_items!(
                        dbtx,
                        NostimintFundsPrefixV1,
                        NostimintFundsKeyV1,
                        Amount,
                        items,
                        "Nostimint Funds"
                    );
                }
                DbKeyPrefix::Outcome => {
                    push_db_pair_items!(
                        dbtx,
                        NostimintOutcomePrefix,
                        NostimintOutcomeKey,
                        NostimintOutputOutcome,
                        items,
                        "Nostimint Outputs"
                    );
                }
                DbKeyPrefix::SignatureShare => {
                    push_db_pair_items!(
                        dbtx,
                        NostimintSignatureSharePrefix,
                        NostimintSignatureShareKey,
                        SerdeSignatureShare,
                        items,
                        "Nostimint Signature Shares"
                    );
                }
                DbKeyPrefix::Event => {
                    push_db_pair_items!(
                        dbtx,
                        NostimintKind1Prefix,
                        NostimintSignatureKey,
                        Option<Event>,
                        items,
                        "Nostimint Events"
                    );
                }
            }
        }

        Box::new(items.into_iter())
    }
}

/// Nostimint module
#[derive(Debug)]
pub struct Nostimint {
    pub cfg: NostimintConfig,
    /// Notifies us to propose an epoch
    pub sign_notify: Notify,
}

/// Implementation of consensus for the server module
#[async_trait]
impl ServerModule for Nostimint {
    /// Define the consensus types
    type Common = NostimintModuleTypes;
    type Gen = NostimintGen;
    type VerificationCache = NostimintVerificationCache;

    async fn await_consensus_proposal(&self, dbtx: &mut ModuleDatabaseTransaction<'_>) {
        // Wait until we have a proposal
        if !self.consensus_proposal(dbtx).await.forces_new_epoch() {
            self.sign_notify.notified().await;
        }
    }

    async fn consensus_proposal(
        &self,
        dbtx: &mut ModuleDatabaseTransaction<'_>,
    ) -> ConsensusProposal<NostimintConsensusItem> {
        // Check for Kind1's to be signed
        let sign_requests: Vec<_> = dbtx
            .find_by_prefix(&NostimintKind1Prefix)
            .await
            .collect()
            .await;

        // Create a Consensus Item
        let consensus_items = sign_requests
            .into_iter()
            .filter(|(_, sig)| sig.is_none())
            .map(|(NostimintKind1Key(message), _)| {
                // TODO: craft nostr note of kind1 and sign wiht FROST key (shoudl the nonce used be part of this?)
                let my_keys = Keys::from_sk_str(&env::var("NOSTR_PRIVKEY").unwrap()).unwrap();
                let bech32_pubkey: String = my_keys.public_key().to_bech32().unwrap();
                println!("Bech32 PubKey: {}", bech32_pubkey);
                let sig = self.cfg.private.private_key_share.sign(&message);
                NostimintConsensusItem::Note(message, SerdeSignatureShare(sig))
            });
        ConsensusProposal::new_auto_trigger(consensus_items.collect())
    }

    async fn process_consensus_item<'a, 'b>(
        &'a self,
        dbtx: &mut ModuleDatabaseTransaction<'b>,
        consensus_item: NostimintConsensusItem,
        peer_id: PeerId,
    ) -> anyhow::Result<()> {
        let NostimintConsensusItem::Note(event, share) = consensus_item;

        if dbtx
            .get_value(&NostimintSignatureShareKey(event.clone(), peer_id))
            .await
            .is_some()
        {
            bail!("Already received a valid signature share")
        }

        if !self
            .cfg
            .consensus
            .public_key_set
            .public_key_share(peer_id.to_usize())
            .verify(&share.0, event.clone())
        {
            bail!("Signature share is invalid");
        }

        dbtx.insert_new_entry(&NostimintSignatureShareKey(event.clone(), peer_id), &share)
            .await;

        // Collect all valid signature shares previously received
        let signature_shares = dbtx
            .find_by_prefix(&NostimintSignatureShareStringPrefix(event.event.id))
            .await
            .collect::<Vec<_>>()
            .await;

        if signature_shares.len() <= self.cfg.consensus.public_key_set.threshold() {
            return Ok(());
        }

        // let _threshold_signature = self
        //     .cfg
        //     .consensus
        //     .public_key_set
        //     .combine_signatures(
        //         signature_shares
        //             .iter()
        //             .map(|(peer_id, share)| (peer_id.1.to_usize(), &share.0)),
        //     )
        //     .expect("We have verified all signature shares before");

        // pretty sure this is incorrect....
        dbtx.remove_by_prefix(&NostimintSignatureShareStringPrefix(event.event.id))
            .await;

        dbtx.insert_entry(&NostimintKind1Key(event.clone()), &Some(event))
            .await;

        Ok(())
    }

    fn build_verification_cache<'a>(
        &'a self,
        _inputs: impl Iterator<Item = &'a NostimintInput> + Send,
    ) -> Self::VerificationCache {
        NostimintVerificationCache
    }

    async fn process_input<'a, 'b, 'c>(
        &'a self,
        dbtx: &mut ModuleDatabaseTransaction<'c>,
        input: &'b NostimintInput,
        _cache: &Self::VerificationCache,
    ) -> Result<InputMeta, ModuleError> {
        let current_funds = dbtx
            .get_value(&NostimintFundsKeyV1(input.account))
            .await
            .unwrap_or(Amount::ZERO);

        // verify user has enough funds or is using the fed account
        if input.amount > current_funds && fed_public_key() != input.account {
            return Err(NostimintError::NotEnoughFunds).into_module_error_other();
        }

        // Subtract funds from normal user, or print funds for the fed
        let updated_funds = if fed_public_key() == input.account {
            current_funds + input.amount
        } else {
            current_funds - input.amount
        };

        dbtx.insert_entry(&NostimintFundsKeyV1(input.account), &updated_funds)
            .await;

        Ok(InputMeta {
            amount: TransactionItemAmount {
                amount: input.amount,
                fee: self.cfg.consensus.tx_fee,
            },
            // IMPORTANT: include the pubkey to validate the user signed this tx
            pub_keys: vec![input.account],
        })
    }

    async fn process_output<'a, 'b>(
        &'a self,
        dbtx: &mut ModuleDatabaseTransaction<'b>,
        output: &'a NostimintOutput,
        out_point: OutPoint,
    ) -> Result<TransactionItemAmount, ModuleError> {
        // Add output funds to the user's account
        let current_funds = dbtx.get_value(&NostimintFundsKeyV1(output.account)).await;
        let updated_funds = current_funds.unwrap_or(Amount::ZERO) + output.amount;
        dbtx.insert_entry(&NostimintFundsKeyV1(output.account), &updated_funds)
            .await;

        // Update the output outcome the user can query
        let outcome = NostimintOutputOutcome(updated_funds, output.account);
        dbtx.insert_entry(&NostimintOutcomeKey(out_point), &outcome)
            .await;

        Ok(TransactionItemAmount {
            amount: output.amount,
            fee: self.cfg.consensus.tx_fee,
        })
    }

    async fn output_status(
        &self,
        dbtx: &mut ModuleDatabaseTransaction<'_>,
        out_point: OutPoint,
    ) -> Option<NostimintOutputOutcome> {
        // check whether or not the output has been processed
        dbtx.get_value(&NostimintOutcomeKey(out_point)).await
    }

    async fn audit(&self, dbtx: &mut ModuleDatabaseTransaction<'_>, audit: &mut Audit) {
        audit
            .add_items(
                dbtx,
                KIND.as_str(),
                &NostimintFundsPrefixV1,
                |k, v| match k {
                    // the fed's test account is considered an asset (positive)
                    // should be the bitcoin we own in a real module
                    NostimintFundsKeyV1(key) if key == fed_public_key() => v.msats as i64,
                    // a user's funds are a federation's liability (negative)
                    NostimintFundsKeyV1(_) => -(v.msats as i64),
                },
            )
            .await;
    }

    fn api_endpoints(&self) -> Vec<ApiEndpoint<Self>> {
        vec![
            api_endpoint! {
                // API allows users ask the fed to threshold-sign a message into a kind1 nostr note
                "sign_note",
                async |module: &Nostimint, context, message: Event| -> EventId {
                    // TODO: Should not write to DB in module APIs
                    let mut dbtx = context.dbtx();
                    // TODO: create event here now
                    dbtx.insert_entry(&NostimintKind1Key(message.clone()), &None).await;
                    module.sign_notify.notify_one();
                    let event_id = message.event.id;
                    Ok(event_id)
                }
            },
            api_endpoint! {
                // API waits for the signature to exist
                "wait_signed_note",
                async |_module: &Nostimint, context, message: Event| -> Event {
                    let future = context.wait_value_matches(NostimintKind1Key(message), |sig| sig.is_some());
                    let sig = future.await;
                    Ok(sig.expect("checked is some"))
                }
            },
        ]
    }
}

/// An in-memory cache we could use for faster validation
#[derive(Debug, Clone)]
pub struct NostimintVerificationCache;

impl fedimint_core::server::VerificationCache for NostimintVerificationCache {}

impl Nostimint {
    /// Create new module instance
    pub fn new(cfg: NostimintConfig) -> Nostimint {
        Nostimint {
            cfg,
            sign_notify: Notify::new(),
        }
    }
}

// fn publish_to_relay(relay: &str, message: &websocket::Message) -> Result<(), String> {
//     let mut client = ClientBuilder::new(relay)
//         .map_err(|err| format!("Could not create client: {}", err.to_string()))?
//         .connect(None)
//         .map_err(|err| format!("Could not connect to relay {}: {}", relay, err.to_string()))?;
//     client
//         .send_message(message)
//         .map_err(|err| format!("could not send message to relay: {}", err.to_string()))?;
//     Ok(())
// }
