CREATE TABLE token_prices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol VARCHAR(20),
    network VARCHAR(50) NOT NULL DEFAULT 'global',
    contract_address CHAR(42),
    price NUMERIC NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USD',
    last_updated_at TIMESTAMPTZ NOT NULL,
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_token_prices_symbol ON token_prices(symbol);
CREATE INDEX idx_token_prices_address ON token_prices(network, contract_address);
CREATE INDEX idx_token_prices_updated ON token_prices(last_updated_at);

ALTER TABLE token_prices
    ADD CONSTRAINT token_prices_network_contract_currency_unique
    UNIQUE (network, contract_address, currency);

ALTER TABLE token_prices
    ADD CONSTRAINT token_prices_symbol_network_currency_unique
    UNIQUE (symbol, network, currency);

CREATE TABLE token_price_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol VARCHAR(20),
    network VARCHAR(50) NOT NULL DEFAULT 'global',
    contract_address CHAR(42),
    price NUMERIC NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USD',
    last_updated_at TIMESTAMPTZ NOT NULL,
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_token_price_history_symbol_currency
    ON token_price_history(symbol, currency, fetched_at DESC);
CREATE INDEX idx_token_price_history_network_address
    ON token_price_history(network, contract_address, fetched_at DESC);
