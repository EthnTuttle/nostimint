use fedimint_core::api::{FederationApiExt, FederationResult, IModuleFederationApi};
use fedimint_core::epoch::SerdeSignature;
use fedimint_core::module::ApiRequestErased;
use fedimint_core::task::{MaybeSend, MaybeSync};
use fedimint_core::{apply, async_trait_maybe_send};

#[apply(async_trait_maybe_send!)]
pub trait NostimintFederationApi {
    async fn sign_note(&self, message: String) -> FederationResult<()>;
    // TODO: Update returned type to be signed nostr event
    async fn wait_signed_note(&self, message: String) -> FederationResult<SerdeSignature>;
}

#[apply(async_trait_maybe_send!)]
impl<T: ?Sized> NostimintFederationApi for T
where
    T: IModuleFederationApi + MaybeSend + MaybeSync + 'static,
{
    async fn sign_note(&self, message: String) -> FederationResult<()> {
        self.request_current_consensus("sign_note".to_string(), ApiRequestErased::new(message))
            .await
    }

    // TODO: Update returned type to be signed nostr event
    async fn wait_signed_note(&self, message: String) -> FederationResult<SerdeSignature> {
        self.request_current_consensus(
            "wait_signed_note".to_string(),
            ApiRequestErased::new(message),
        )
        .await
    }
}
