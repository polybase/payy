DROP INDEX IF EXISTS idx_token_price_history_network_address;
DROP INDEX IF EXISTS idx_token_price_history_symbol_currency;
DROP TABLE IF EXISTS token_price_history;

ALTER TABLE token_prices
    DROP CONSTRAINT IF EXISTS token_prices_symbol_network_currency_unique;
ALTER TABLE token_prices
    DROP CONSTRAINT IF EXISTS token_prices_network_contract_currency_unique;
DROP INDEX IF EXISTS idx_token_prices_updated;
DROP INDEX IF EXISTS idx_token_prices_address;
DROP INDEX IF EXISTS idx_token_prices_symbol;
DROP TABLE IF EXISTS token_prices;
