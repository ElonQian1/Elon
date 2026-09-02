mod esk_asset {
    pub(crate) mod model {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../server/src/esk_asset/model.rs"
        ));
    }

    mod amount {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../server/src/esk_asset/amount.rs"
        ));
    }

    pub(crate) use amount::format_esk_amount;
    pub(crate) use model::{
        EskAccountLedger, EskAssetMode, ESK_ASSET_ID, ESK_DECIMALS, ESK_NAME, ESK_SYMBOL,
    };
}

#[path = "../../../server/src/quant_esk_asset_projection.rs"]
mod quant_esk_asset_projection;
#[path = "../../../server/src/quant_paper_signer.rs"]
mod quant_paper_signer;
