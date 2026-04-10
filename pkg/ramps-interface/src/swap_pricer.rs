use async_trait::async_trait;
use element::Element;
use unimock::unimock;

use crate::Result;

/// Converts one in-rollup note amount into another using live token pricing.
#[unimock(api = SwapPricerMock)]
#[async_trait]
pub trait SwapPricer: Send + Sync {
    /// Convert an amount of `from_note_kind` into the equivalent amount of
    /// `to_note_kind`, rounding down in favor of the system.
    async fn convert(
        &self,
        from_note_kind: &Element,
        to_note_kind: &Element,
        from_amount: Element,
    ) -> Result<Element>;
}
