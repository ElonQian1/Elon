use crate::types::AppState;
use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    let js = [
        ("content-type", "application/javascript; charset=utf-8"),
        ("cache-control", "no-cache"),
    ];
    Router::new()
        .route(
            "/assets/grid_share.js",
            get(move || async move { (js, include_str!("../../assets/grid_share.js")) }),
        )
        .route(
            "/assets/grid_share_view.js",
            get(move || async move { (js, include_str!("../../assets/grid_share_view.js")) }),
        )
        .route(
            "/assets/grid_share.css",
            get(|| async {
                (
                    [
                        ("content-type", "text/css; charset=utf-8"),
                        ("cache-control", "no-cache"),
                    ],
                    include_str!("../../assets/grid_share.css"),
                )
            }),
        )
        .route("/assets/grid-token-icons/:name", get(token_icon))
}

async fn token_icon(Path(name): Path<String>) -> Response {
    match icon_bytes(&name) {
        Some(bytes) => (
            [
                ("content-type", "image/png"),
                ("cache-control", "public, max-age=86400"),
                ("x-content-type-options", "nosniff"),
            ],
            bytes,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// Embedded allowlist, shared with Android. Never turn request paths into filesystem reads.
macro_rules! icons {
    ($name:expr; $($coin:literal),+ $(,)?) => {
        match $name {
            $(concat!($coin, ".png") => Some(include_bytes!(concat!("../../../../android/app/src/main/assets/grid-token-icons/", $coin, ".png")).as_slice()),)+
            _ => None,
        }
    };
}
fn icon_bytes(name: &str) -> Option<&'static [u8]> {
    icons!(name;
        "0xbtc", "1inch", "2give", "aave", "abt", "act", "actn", "ada", "add", "adx",
        "ae", "aeon", "aeur", "agi", "agrs", "aion", "algo", "amb", "amp", "ampl",
        "ankr", "ant", "ape", "apex", "appc", "ardr", "arg", "ark", "arn", "arnx",
        "ary", "ast", "atlas", "atm", "atom", "audr", "aury", "auto", "avax", "aywa",
        "bab", "bal", "band", "bat", "bay", "bcbc", "bcc", "bcd", "bch", "bcio",
        "bcn", "bco", "bcpt", "bdl", "beam", "bela", "bix", "blcn", "blk", "block",
        "blz", "bnb", "bnt", "bnty", "booty", "bos", "bpt", "bq", "brd", "bsd",
        "bsv", "btc", "btcd", "btch", "btcp", "btcz", "btdx", "btg", "btm", "bts",
        "btt", "btx", "burst", "bze", "call", "cc", "cdn", "cdt", "cenz", "chain",
        "chat", "chips", "chsb", "chz", "cix", "clam", "cloak", "cmm", "cmt", "cnd",
        "cnx", "cny", "cob", "colx", "comp", "coqui", "cred", "crpt", "crv", "crw",
        "cs", "ctr", "ctxc", "cvc", "d", "dai", "dash", "dat", "data", "dbc",
        "dcn", "dcr", "deez", "dent", "dew", "dgb", "dgd", "dlt", "dnt", "dock",
        "doge", "dot", "drgn", "drop", "dta", "dth", "dtr", "ebst", "eca", "edg",
        "edo", "edoge", "ela", "elec", "elf", "elix", "ella", "emb", "emc", "emc2",
        "eng", "enj", "entrp", "eon", "eop", "eos", "eqli", "equa", "etc", "eth",
        "ethos", "etn", "etp", "eur", "evx", "exmo", "exp", "fair", "fct", "fida",
        "fil", "fjc", "fldc", "flo", "flux", "fsn", "ftc", "fuel", "fun", "game",
        "gas", "gbp", "gbx", "gbyte", "generic", "gin", "glxt", "gmr", "gmt", "gno",
        "gnt", "gold", "grc", "grin", "grs", "grt", "gsc", "gto", "gup", "gusd",
        "gvt", "gxs", "gzr", "hight", "hns", "hodl", "hot", "hpb", "hsr", "ht",
        "html", "huc", "husd", "hush", "icn", "icp", "icx", "ignis", "ilk", "ink",
        "ins", "ion", "iop", "iost", "iotx", "iq", "itc", "jnt", "jpy", "kcs",
        "kin", "klown", "kmd", "knc", "krb", "ksm", "lbc", "lend", "leo", "link",
        "lkk", "loom", "lpt", "lrc", "lsk", "ltc", "lun", "maid", "mana", "matic",
        "max", "mcap", "mco", "mda", "mds", "med", "meetone", "mft", "miota", "mith",
        "mkr", "mln", "mnx", "mnz", "moac", "mod", "mona", "msr", "mth", "mtl",
        "music", "mzc", "nano", "nas", "nav", "ncash", "ndz", "nebl", "neo", "neos",
        "neu", "nexo", "ngc", "nio", "nkn", "nlc2", "nlg", "nmc", "nmr", "npxs",
        "ntbc", "nuls", "nxs", "nxt", "oax", "ok", "omg", "omni", "one", "ong",
        "ont", "oot", "ost", "ox", "oxt", "oxy", "part", "pasc", "pasl", "pax",
        "paxg", "pay", "payx", "pink", "pirl", "pivx", "plr", "poa", "poe", "polis",
        "poly", "pot", "powr", "ppc", "ppp", "ppt", "pre", "prl", "pungo", "pura",
        "qash", "qiwi", "qlc", "qnt", "qrl", "qsp", "qtum", "r", "rads", "rap",
        "ray", "rcn", "rdd", "rdn", "ren", "rep", "repv2", "req", "rhoc", "ric",
        "rise", "rlc", "rpx", "rub", "rvn", "ryo", "safe", "safemoon", "sai", "salt",
        "san", "sand", "sbd", "sberbank", "sc", "ser", "shift", "sib", "sin", "skl",
        "sky", "slr", "sls", "smart", "sngls", "snm", "snt", "snx", "soc", "sol",
        "spacehbit", "spank", "sphtx", "srn", "stak", "start", "steem", "storj", "storm", "stox",
        "stq", "strat", "stx", "sub", "sumo", "sushi", "sys", "taas", "tau", "tbx",
        "tel", "ten", "tern", "tgch", "theta", "tix", "tkn", "tks", "tnb", "tnc",
        "tnt", "tomo", "tpay", "trig", "trtl", "trx", "tusd", "tzc", "ubq", "uma",
        "uni", "unity", "usd", "usdc", "usdt", "utk", "veri", "vet", "via", "vib",
        "vibe", "vivo", "vrc", "vrsc", "vtc", "vtho", "wabi", "wan", "waves", "wax",
        "wbtc", "wgr", "wicc", "wings", "wpr", "wtc", "x", "xas", "xbc", "xbp",
        "xby", "xcp", "xdn", "xem", "xin", "xlm", "xmcc", "xmg", "xmo", "xmr",
        "xmy", "xp", "xpa", "xpm", "xpr", "xrp", "xsg", "xtz", "xuc", "xvc",
        "xvg", "xzc", "yfi", "yoyow", "zcl", "zec", "zel", "zen", "zest", "zil",
        "zilla", "zrx",
    )
}

#[cfg(test)]
mod tests {
    use super::icon_bytes;
    #[test]
    fn grid_icons_are_embedded_and_allowlisted() {
        assert_eq!(&icon_bytes("qnt.png").unwrap()[..8], b"\x89PNG\r\n\x1a\n");
        assert!(icon_bytes("unknown.png").is_none());
        assert!(icon_bytes("../qnt.png").is_none());
        assert!(icon_bytes("qnt.svg").is_none());
    }
}
