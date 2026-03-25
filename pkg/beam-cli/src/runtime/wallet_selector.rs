use super::{BeamApp, parse_address};
use crate::{
    ens::{ens_name, resolve_ens_address},
    error::{Error, Result},
    keystore::{find_wallet, is_address_selector},
};

impl BeamApp {
    pub(crate) async fn canonical_wallet_selector(
        &self,
        selection: Option<&str>,
    ) -> Result<Option<String>> {
        let Some(selection) = selection else {
            return Ok(None);
        };
        let wallets = self.keystore_store.get().await;

        if let Ok(wallet) = find_wallet(&wallets.wallets, selection) {
            return Ok(Some(wallet.name.clone()));
        }
        if is_address_selector(selection) {
            return Ok(Some(format!("{:#x}", parse_address(selection)?)));
        }
        let Some(name) = ens_name(selection) else {
            return Err(Error::WalletNotFound {
                selector: selection.to_string(),
            });
        };
        let address = resolve_ens_address(self, &name)
            .await?
            .ok_or_else(|| Error::EnsNameNotFound { name })?;
        let address = format!("{address:#x}");

        Ok(Some(match find_wallet(&wallets.wallets, &address) {
            Ok(wallet) => wallet.name.clone(),
            Err(Error::WalletNotFound { .. }) => address,
            Err(err) => return Err(err),
        }))
    }
}
