# Stable Gas

### Gas in PUSD stablecoin

[PUSD](pusd.md) is the native gas token for Payy, enabling simpler gas interactions:

* PUSD is non-volatile, so the price you pay for gas remains more consistent
* Less risk related to holding a volatile asset for future gas payments
* Simpler gas calculations, no conversion into USD required
* PUSD natively earns yield, helping to accrue value to PAYY token

### Fee stabilisation

Given that Payy is a high performance blockchain with multiple blocks per second, fees are dynamically adjusted every 30 blocks (\~10 seconds) to reduce excessive changes in gas prices.

In addition, Payy builds on Ethereum’s EIP-1559 but smooths how the base fee moves so fees change less erratically. Instead of reacting only to the last block’s gas usage, Payy uses a simple weighted average (EWMA) of recent usage to decide how much to adjust the base fee. This keeps confirmations reliable without surprise fee spikes.

#### Raw utilisation calculation

Raw utilisation per block:

&#x20;$$U_t = \frac{\text{gas used in block } t}{\text{target gas per block}}$$)

Smoothed utilisation:

$$\hat{U}_t = \alpha \cdot U_{t-1} + (1 - \alpha) \cdot \hat{U}_{t-1}$$

where:

$$\begin{aligned} U_t &\;\text{gas utilisation ratio of block } t \\ U^{\ast} &\;\text{target utilisation (e.g.\ } 50\%\text{)} \\ \hat{U}_t &\;\text{smoothed utilisation estimate} \\ \alpha &\in (0,1] \;\text{smoothing factor} \\ B_t &\;\text{base fee for block } t \end{aligned}$$

#### Base fee calculation

The base fee update then mirrors EIP-1559, but uses the smoothed utilisation. Using the utilisation calculation we can update the base fee as follows:

$$B_t = B_{t-1} \cdot \left(1 + \gamma \cdot \frac{\hat{U}_t - U^{\ast}}{U^{\ast}}\right)$$

where:

$$\gamma \;\text{is the maximum per-block adjustment factor}$$

### Gas swaps

PUSD is the native token on Payy, and is used for paying gas, but network participants receive payment in PAYY.

To do this, a native swap is performed at the start of each block to swap all gas paid in PUSD to PAYY token.

### Why it matters

* Stablecoins payments for gas provide a more stable gas fee and therefore more predictable operations for payments
* Institutions prefer to hold stablecoins over volatile assets&#x20;
* Additional compliance and controls are often needed with non-stable tokens

