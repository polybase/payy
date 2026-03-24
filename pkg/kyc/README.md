# KYC (Know Your Customer) Crate

A comprehensive Rust library for managing KYC (Know Your Customer) verification processes across multiple financial service providers. This crate provides a unified interface for handling identity verification, document management, and compliance requirements.

## Overview

The `kyc` crate abstracts the complexity of integrating with various KYC providers (Alfred, Manteca, Rain, Sumsub) by providing:

- **Standardized data structures** for personal information and documents
- **Comprehensive error handling** for KYC-specific scenarios
- **Type-safe status management** for verification workflows
- **Document validation and storage** capabilities

## Features

- `diesel` - Enable PostgreSQL database integration via Diesel ORM
- `ts-rs` - Generate TypeScript type definitions for frontend integration

## Architecture

### Core Components

#### 1. **Kyc Struct** (`kyc.rs`)
The central data structure that holds all personal information required for KYC verification:

```rust
pub struct Kyc {
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub dob: Option<chrono::NaiveDate>,
    pub addressstreet: Option<String>,
    pub addresscity: Option<String>,
    pub addresscountry: Option<Country>,
    pub nationalities: Option<CountryList>,
    pub documents: Option<HashMap<Country, HashMap<IDKind, IDDocument>>>,
    pub phone: Option<String>,
    pub email: Option<String>,
    // ... additional fields
}
```

#### 2. **Type System** (`types.rs`)

##### KycStatus Enum
Represents the current state of KYC verification:

```rust
pub enum KycStatus {
    NotStarted,    // Initial state
    Pending,       // Verification in progress
    Approved,      // Successfully verified
    UpdateRequired, // Additional information needed
    Rejected,      // Verification failed
}
```

##### KycField Enum
Identifies specific fields in the KYC data structure:

```rust
pub enum KycField {
    Firstname,
    Lastname,
    Dob,
    AddressStreet,
    AddressCity,
    Phone,
    Documents,
    // ... etc
}
```

#### 3. **Document Management** (`documents.rs`)

Handles identity document storage and validation:

- **IDDocument**: Structure for storing document images (front/back)
- **IDKind**: Enumeration of supported document types (Passport, DriverLicense, NationalId, etc.)
- **IDDocumentField**: Wrapper for document data (supports both base64 strings and raw bytes)

#### 4. **Update Requirements** (`kyc_update_required.rs`)

Represents validation errors and update requirements:

- **KycUpdateRequired**: Detailed information about why updates are needed
- **KycUpdateRequiredInvalidFields**: Field-specific failure reasons

#### 5. **Error Handling** (`error.rs`)

Comprehensive error types for KYC operations:

```rust
pub enum Error {
    MissingKYCField(String),
    InvalidPhoneFormat,
    InvalidState { state: String },
    Json(String),
}
```

## Usage Examples

### Working with KYC Data

```rust
use kyc::{Kyc, KycStatus};
use currency::Country;

// Create a new KYC instance
let mut kyc = Kyc::default();
kyc.firstname = Some("John".to_string());
kyc.lastname = Some("Doe".to_string());
kyc.addresscountry = Some(Country::US);

// Use getter methods with error handling
let full_name = format!("{} {}", 
    kyc.get_firstname()?, 
    kyc.get_lastname()?
);

// Merge KYC data from different sources
let updated_kyc = Kyc {
    phone: Some("+1234567890".to_string()),
    email: Some("john@example.com".to_string()),
    ..Default::default()
};

let merged = kyc.merge(&updated_kyc);
```

## Integration with Ramps Providers

This crate provides shared KYC domain types used by the ramps stack. Concrete
provider integrations live in the provider crates:

- **Alfred**: `alfred-provider` (European-focused KYC provider)
- **Manteca**: `manteca-provider` (Latin American markets)
- **Rain**: `rain-provider` (Middle Eastern financial services)
- **Sumsub**: `sumsub-provider` (Global KYC/AML compliance)

Shared contracts and validation live in `ramps-interface`, while this crate
defines the common KYC data types and helpers used across providers.

## Database Integration

When the `diesel` feature is enabled, all core types can be stored directly in PostgreSQL:

```rust
// Kyc struct can be stored as JSONB
#[derive(AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct Kyc { /* ... */ }

// KycStatus can be stored as TEXT
#[derive(AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub enum KycStatus { /* ... */ }
```

## Testing

The crate includes comprehensive test coverage:

```bash
# Run all tests
cargo test -p kyc

# Run with specific features
cargo test -p kyc --features diesel,ts-rs
```

Test categories include:
- Getter method validation
- Serialization/deserialization
- Document merging logic
- Phone number validation
- Status transitions
- Error handling

## Security Considerations

1. **Sensitive Data Handling**: Personal information should be encrypted at rest and in transit
2. **Document Storage**: Raw document bytes are sanitized during merge operations to prevent memory bloat
3. **Phone Validation**: Phone numbers are validated using the `phonenumber` crate for international formats
4. **Access Control**: Implement appropriate access controls when exposing KYC data through APIs

## Contributing

When extending this crate:

1. Ensure all new fields in `Kyc` struct have corresponding getter methods with proper error handling
2. Update the `KycField` enum when adding new fields
3. Maintain backward compatibility for serialization
4. Add comprehensive tests for new functionality
5. Update provider implementations in the relevant provider crate as needed

## License

This crate follows the same license as the parent workspace.
