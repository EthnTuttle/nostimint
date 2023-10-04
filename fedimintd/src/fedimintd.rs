use fedimintd::fedimintd::Fedimintd;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Fedimintd::new()?
        .with_default_modules()
        .with_module(fedimint_nostimint_server::NostimintGen)
        .with_extra_module_inits_params(
            3,
            fedimint_nostimint_server::KIND,
            fedimint_nostimint_server::NostimintGenParams::default(),
        )
        .run()
        .await
}
