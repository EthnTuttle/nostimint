use std::ffi;

use fedimint_client::derivable_secret::DerivableSecret;
use fedimint_client::module::init::ClientModuleInit;
use fedimint_client::module::{ClientModule, IClientModule};
use fedimint_client::sm::{Context, ModuleNotifier};

use fedimint_client::{Client, DynGlobalClientContext};
use fedimint_core::api::{DynGlobalApi, DynModuleApi};
use fedimint_core::config::FederationId;
use fedimint_core::core::{Decoder, KeyPair};
use fedimint_core::db::Database;
use fedimint_core::module::{
    ApiVersion, CommonModuleInit, ExtendsCommonModuleInit, ModuleCommon, MultiApiVersion,
    TransactionItemAmount,
};

use fedimint_core::{apply, async_trait_maybe_send};
pub use fedimint_nostimint_common as common;
use fedimint_nostimint_common::config::NostimintClientConfig;
use fedimint_nostimint_common::{NostimintCommonGen, NostimintModuleTypes, KIND};

use secp256k1::{Secp256k1, XOnlyPublicKey};
use states::NostimintStateMachine;
use threshold_crypto::{PublicKey, Signature};
use tracing::info;

use crate::api::NostimintFederationApi;

pub mod api;
mod states;

/// Exposed API calls for client apps
#[apply(async_trait_maybe_send!)]
pub trait NostimintClientExt {
    /// Request the federation signs a note for us
    async fn fed_sign_note(&self, message: &str) -> anyhow::Result<Signature>;

    /// Return our account
    fn account(&self) -> XOnlyPublicKey;

    /// Return the fed's public key
    fn fed_public_key(&self) -> PublicKey;
}

#[apply(async_trait_maybe_send!)]
impl NostimintClientExt for Client {
    async fn fed_sign_note(&self, message: &str) -> anyhow::Result<Signature> {
        let (_nostimint, instance) = self.get_first_module::<NostimintClientModule>(&KIND);
        instance.api.sign_note(message.to_string()).await?;
        info!("message sent to server to be signed: {}", message);
        let sig = instance.api.wait_signed_note(message.to_string()).await?;
        Ok(sig.0)
    }

    fn account(&self) -> XOnlyPublicKey {
        let (nostimint, _instance) = self.get_first_module::<NostimintClientModule>(&KIND);
        nostimint.key.x_only_public_key().0
    }

    fn fed_public_key(&self) -> PublicKey {
        let (nostimint, _instance) = self.get_first_module::<NostimintClientModule>(&KIND);
        nostimint.cfg.fed_public_key
    }
}

#[derive(Debug)]
pub struct NostimintClientModule {
    cfg: NostimintClientConfig,
    key: KeyPair,
    notifier: ModuleNotifier<DynGlobalClientContext, NostimintStateMachine>,
}

/// Data needed by the state machine
#[derive(Debug, Clone)]
pub struct NostimintClientContext {
    pub nostimint_decoder: Decoder,
}

// TODO: Boiler-plate
impl Context for NostimintClientContext {}

#[apply(async_trait_maybe_send!)]
impl ClientModule for NostimintClientModule {
    type Common = NostimintModuleTypes;
    type ModuleStateMachineContext = NostimintClientContext;
    type States = NostimintStateMachine;

    fn context(&self) -> Self::ModuleStateMachineContext {
        NostimintClientContext {
            nostimint_decoder: self.decoder(),
        }
    }

    fn input_amount(&self, input: &<Self::Common as ModuleCommon>::Input) -> TransactionItemAmount {
        TransactionItemAmount {
            amount: input.amount,
            fee: self.cfg.tx_fee,
        }
    }

    fn output_amount(
        &self,
        output: &<Self::Common as ModuleCommon>::Output,
    ) -> TransactionItemAmount {
        TransactionItemAmount {
            amount: output.amount,
            fee: self.cfg.tx_fee,
        }
    }

    fn supports_being_primary(&self) -> bool {
        false
    }

    async fn handle_cli_command(
        &self,
        client: &Client,
        args: &[ffi::OsString],
    ) -> anyhow::Result<serde_json::Value> {
        if args.is_empty() {
            return Err(anyhow::format_err!(
                "Expected to be called with at least 1 arguments: <command> …"
            ));
        }

        let command = args[0].to_string_lossy();

        match command.as_ref() {
            "sign-note" => {
                if args.len() != 2 {
                    return Err(anyhow::format_err!(
                        "`sign-note` command expects 1 argument: <message of kind1 note>"
                    ));
                }

                // TODO: craft other note types

                Ok(serde_json::to_value(
                    client.fed_sign_note(&args[1].to_string_lossy()).await?,
                )?)
            }
            command => Err(anyhow::format_err!(
                "Unknown command: {command}, supported commands: print-money"
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NostimintClientGen;

// TODO: Boilerplate-code
impl ExtendsCommonModuleInit for NostimintClientGen {
    type Common = NostimintCommonGen;
}

/// Generates the client module
#[apply(async_trait_maybe_send!)]
impl ClientModuleInit for NostimintClientGen {
    type Module = NostimintClientModule;

    fn supported_api_versions(&self) -> MultiApiVersion {
        MultiApiVersion::try_from_iter([ApiVersion { major: 0, minor: 0 }])
            .expect("no version conflicts")
    }

    async fn init(
        &self,
        _federation_id: FederationId,
        cfg: NostimintClientConfig,
        _db: Database,
        _api_version: ApiVersion,
        module_root_secret: DerivableSecret,
        notifier: ModuleNotifier<DynGlobalClientContext, <Self::Module as ClientModule>::States>,
        _api: DynGlobalApi,
        _module_api: DynModuleApi,
    ) -> anyhow::Result<Self::Module> {
        Ok(NostimintClientModule {
            cfg,
            key: module_root_secret.to_secp_key(&Secp256k1::new()),
            notifier,
        })
    }
}
