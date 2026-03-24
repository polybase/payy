// lint-long-file-override allow-max-lines=350
#[cfg(feature = "diesel")]
use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    pg::Pg,
    serialize::{self, IsNull, Output, ToSql},
};
use element::Element;
use serde::{Deserialize, Serialize};
#[cfg(feature = "diesel")]
use std::io::Write;
use strum::{Display, EnumString};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;
#[derive(
    Debug,
    Serialize,
    Deserialize,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Display,
    EnumString,
)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub enum Category {
    Bills,
    Charity,
    EatingOut,
    Entertainment,
    Expenses,
    Family,
    Finances,
    General,
    Gifts,
    Groceries,
    Holidays,
    Income,
    PersonalCare,
    Savings,
    Shopping,
    Transfer,
    Transport,
    PetCare,
}

impl Category {
    #[must_use]
    pub fn is_safe_6012_mcc_transaction(merchant_name: &str, amount: &Element) -> bool {
        amount.is_zero() && merchant_name == "Visa Provisioning Service"
    }

    #[must_use]
    #[allow(clippy::match_same_arms)]
    #[allow(clippy::too_many_lines)]
    pub fn from_mcc(mcc: u16) -> Category {
        match mcc {
            // Pet Care
            742 => Category::PetCare, // Veterinary Services
            // Expenses
            763 => Category::Expenses,  // Agricultural Co-operatives
            780 => Category::Expenses,  // Horticultural Services, Landscaping Services
            1520 => Category::Expenses, // General Contractors-Residential and Commercial
            1711 => Category::Expenses, // Air Conditioning Contractors – Sales and Installation, Heating Contractors – Sales, Service, Installation
            1731 => Category::Expenses, // Electrical Contractors
            1740 => Category::Expenses, // Insulation – Contractors, Masonry, Stonework Contractors, Plastering Contractors, Stonework and Masonry Contractors, Tile Settings Contractors
            1750 => Category::Expenses, // Carpentry Contractors
            1761 => Category::Expenses, // Roofing – Contractors, Sheet Metal Work – Contractors, Siding – Contractors
            1771 => Category::Expenses, // Contractors – Concrete Work
            1799 => Category::Expenses, // Contractors – Special Trade, Not Elsewhere Classified
            2741 => Category::Expenses, // Miscellaneous Publishing and Printing
            2791 => Category::Expenses, // Typesetting, Plate Making, & Related Services
            2842 => Category::Expenses, // Specialty Cleaning, Polishing, and Sanitation Preparations
            4119 => Category::Expenses, // Ambulance Services
            4225 => Category::Expenses, // Public Warehousing
            7332 => Category::Expenses, // Blueprinting and Photocopying
            7692 => Category::Expenses, // Welding Repair
            8111 => Category::Expenses, // Legal Services and Attorneys
            8211 => Category::Expenses, // Elementary and Secondary Schools
            8220 => Category::Expenses, // Colleges, Universities, Professional Schools, and Junior Colleges
            8241 => Category::Expenses, // Correspondence Schools
            8244 => Category::Expenses, // Business and Secretarial Schools
            8249 => Category::Expenses, // Vocational and Trade Schools
            8299 => Category::Expenses, // Schools and Educational Services
            8734 => Category::Expenses, // Testing Laboratories
            8911 => Category::Expenses, // Architectural, Engineering, and Surveying Services
            8999 => Category::Expenses, // Professional Services
            9211 => Category::Expenses, // Court Costs, Including Alimony and Child Support
            9222 => Category::Expenses, // Fines
            9223 => Category::Expenses, // Bail and Bond Payments
            9311 => Category::Expenses, // Tax Payments
            9399 => Category::Expenses, // Government Services
            9402 => Category::Expenses, // Postal Services
            9405 => Category::Expenses, // Intra-Government Purchases
            9700 => Category::Expenses, // Automated Referral Service
            9701 => Category::Expenses, // Visa Credential Service
            9702 => Category::Expenses, // GCAS Emergency Services
            9950 => Category::Expenses, // Intra-Company Purchases
            // Transport
            3000..=3299 => Category::Holidays,  // Airlines
            3351..=3441 => Category::Transport, // Car Rentals
            3501..=3999 => Category::Transport, // Car Rentals and Railroads
            4011 => Category::Transport,        // Railroads
            4111 => Category::Transport, // Local and Suburban Commuter Passenger Transportation
            4112 => Category::Transport, // Passenger Railways
            4121 => Category::Transport, // Taxicabs and Limousines
            4131 => Category::Transport, // Bus Lines
            4214 | 4215 => Category::Transport, // Motor Freight Carriers
            4457 => Category::Transport, // Boat Rentals
            4468 => Category::Transport, // Marinas
            4511 => Category::Transport, // Air Carriers, Airlines
            4582 => Category::Transport, // Airports
            4784 => Category::Transport, // Tolls and Bridge Fees
            4789 => Category::Transport, // Other Transport Services
            5172 => Category::Transport, // Petroleum Products
            5541 => Category::Transport, // Service Stations
            5542 => Category::Transport, // Automated Fuel Dispensers
            7511 => Category::Transport, // Truck Stop
            7512 => Category::Transport, // Automobile Rental Agency
            7513 => Category::Transport, // Truck and Utility Trailer Rentals
            7519 => Category::Transport, // Motor Home and Recreational Vehicle Rentals
            7523 => Category::Transport, // Parking Lots, Garages
            7531 => Category::Transport, // Automotive Body Repair Shops
            7534 => Category::Transport, // Tire Retreading and Repair Shops
            7535 => Category::Transport, // Automotive Paint Shops
            7538 => Category::Transport, // Automotive Service Shops
            7542 => Category::Transport, // Car Washes
            7549 => Category::Transport, // Towing Services
            // Bills
            4812 | 4815 | 4816 | 4821 | 4899 => Category::Bills, // Telecommunications
            4814 => Category::Bills,                             // Telecommunication Services
            4900 => Category::Bills, // Electric, Gas, Sanitary and Water Utilities
            // Transfer
            4829 => Category::Transfer, // Money Transfer
            6536 => Category::Transfer, // MoneySend Intracountry
            // Savings
            6760 => Category::Savings, // Savings Bonds
            // Holidays
            4411 => Category::Holidays, // Cruise Lines
            4722 => Category::Holidays, // Travel Agencies and Tour Operators
            4723 => Category::Holidays, // Package Tour Operators
            7011 => Category::Holidays, // Lodging - Hotels, Motels, Resorts
            7012 => Category::Holidays, // Timeshares
            7032 => Category::Holidays, // Sporting and Recreational Camps
            7033 => Category::Holidays, // Trailer Parks and Campgrounds
            // Shopping
            5013 => Category::Shopping,        // Motor Vehicle Supplies
            5021 => Category::Shopping,        // Office and Commercial Furniture
            5039 => Category::Shopping,        // Construction Materials
            5044 => Category::Shopping,        // Office Equipment
            5045 => Category::Shopping,        // Computers, Peripherals, and Software
            5047 => Category::Shopping,        // Medical Equipment
            5051 => Category::Shopping,        // Metal Service Centers
            5065 => Category::Shopping,        // Electrical Parts
            5072 => Category::Shopping,        // Hardware Supplies
            5074 => Category::Shopping,        // Plumbing and Heating
            5085 => Category::Shopping,        // Industrial Supplies
            5099 => Category::Shopping,        // Durable Goods
            5111 => Category::Shopping, // Stationery, Office Supplies, Printing, and Writing Paper
            5131 => Category::Shopping, // Piece Goods and Notions
            5137 => Category::Shopping, // Uniforms and Commercial Clothing
            5139 => Category::Shopping, // Commercial Footwear
            5169 => Category::Shopping, // Chemicals and Allied Products
            5192 => Category::Shopping, // Books and Newspapers
            5198 => Category::Shopping, // Paints and Supplies
            5199 => Category::Shopping, // Non-durable Goods
            5200 => Category::Shopping, // Home Supply Warehouse Stores
            5211 => Category::Shopping, // Building Materials Stores
            5231 => Category::Shopping, // Glass, Paint, and Wallpaper
            5251 => Category::Shopping, // Hardware Stores
            5261 => Category::Shopping, // Lawn and Garden Supply
            5271 => Category::Shopping, // Mobile Home Dealers
            5300 => Category::Shopping, // Wholesale Clubs
            5309 => Category::Shopping, // Duty-Free Store
            5310 => Category::Shopping, // Discount Stores
            5311 => Category::Shopping, // Department Stores
            5331 => Category::Shopping, // Variety Stores
            5399 => Category::Shopping, // Miscellaneous General Merchandise Stores
            5411 => Category::Groceries, // Grocery Stores, Supermarkets
            5422 => Category::Groceries, // Meat Provisioners
            5441 => Category::Groceries, // Candy and Confectionery Stores
            5451 => Category::Groceries, // Dairy Products Stores
            5462 => Category::Groceries, // Bakeries
            5499 => Category::Groceries, // Miscellaneous Food Stores
            5511 | 5521 => Category::Shopping, // Car and Truck Dealers
            5531 => Category::Shopping, // Automobile Supply Stores
            5532 => Category::Shopping, // Automotive Tire Stores
            5533 => Category::Shopping, // Automotive Parts and Accessories Stores
            5551 => Category::Shopping, // Boat Dealers
            5561 => Category::Shopping, // Recreational Dealers
            5571 => Category::Shopping, // Motorcycle Dealers
            5592 => Category::Shopping, // Motor Home Dealers
            5598 => Category::Shopping, // Snowmobile Dealers
            5599 => Category::Shopping, // Miscellaneous Auto Dealers
            5611 => Category::Shopping, // Men's and Boy's Clothing Stores
            5621 => Category::Shopping, // Women's Ready-to-Wear Stores
            5631 => Category::Shopping, // Women's Accessory Shops
            5641 => Category::Family,   // Children's and Infants' Wear Stores
            5651 => Category::Shopping, // Family Clothing Stores
            5655 => Category::Shopping, // Sports Apparel Stores
            5661 => Category::Shopping, // Shoe Stores
            5681 => Category::Shopping, // Furriers and Fur Shops
            5691 => Category::Shopping, // Men's and Women's Clothing Stores
            5699 => Category::Shopping, // Miscellaneous Apparel and Accessory Shops
            5712 => Category::Shopping, // Furniture and Home Furnishings
            5713 => Category::Shopping, // Floor Covering Stores
            5714 => Category::Shopping, // Drapery and Upholstery Stores
            5718 => Category::Shopping, // Fireplace and Accessories Stores
            5719 => Category::Shopping, // Miscellaneous Home Furnishing Stores
            5722 => Category::Shopping, // Household Appliance Stores
            5732 => Category::Shopping, // Electronics Stores
            5733 => Category::Shopping, // Music Stores
            5734 => Category::Shopping, // Computer Software Stores
            5735 => Category::Shopping, // Record Stores
            5811 => Category::EatingOut, // Caterers
            5812 => Category::EatingOut, // Eating Places and Restaurants
            5813 => Category::Entertainment, // Drinking Places (Alcoholic Beverages)
            5814 => Category::EatingOut, // Fast Food Restaurants
            5815..=5818 => Category::Entertainment, // Digital Goods
            5832 | 5932 | 5937 => Category::Shopping, // Antique Shops
            5912 => Category::Shopping, // Drug Stores and Pharmacies
            5921 => Category::Groceries, // Package Stores – Beer, Wine, and Liquor
            5931 => Category::Shopping, // Used Merchandise and Secondhand Stores
            5933 => Category::Shopping, // Pawn Shops
            5935 => Category::Shopping, // Wrecking and Salvage Yards
            5940 => Category::Shopping, // Bicycle Shops
            5941 => Category::Shopping, // Sporting Goods Stores
            5942 => Category::Shopping, // Book Stores
            5943 => Category::Shopping, // Stationery Stores
            5945 => Category::Shopping, // Hobby, Toy, and Game Shops
            5946 => Category::Shopping, // Camera and Photographic Supply Stores
            5947 => Category::Gifts,    // Gift, Card, Novelty, and Souvenir Shops
            5948 => Category::Shopping, // Leather Goods Stores
            5949 => Category::Shopping, // Sewing and Fabric Stores
            5950 => Category::Shopping, // Glassware/Crystal Stores
            5960 | 5962 => Category::Shopping, // Direct Marketing
            5963 => Category::Shopping, // Door-to-Door Sales
            5970 => Category::Shopping, // Artist's Supply and Craft Shops
            5971 => Category::Shopping, // Art Dealers and Galleries
            5972 => Category::Shopping, // Stamp and Coin Stores
            5973 => Category::Shopping, // Religious Goods Stores
            5977 => Category::Shopping, // Cosmetic Stores
            5978 => Category::Shopping, // Typewriter Stores
            5993 => Category::Shopping, // Cigar Stores
            5994 => Category::Shopping, // News Dealers
            5995 => Category::PetCare,  // Pet Shops, Pet Foods, and Supplies Stores
            5996 => Category::Shopping, // Swimming Pools Stores
            5997 => Category::Shopping, // Electric Razor Stores
            5998 => Category::Shopping, // Tent and Awning Shops
            5999 => Category::Shopping, // Miscellaneous and Specialty Retail Stores
            // Gifts
            5094 => Category::Gifts, // Precious Stones and Jewelry
            5193 => Category::Gifts, // Florists' Supplies
            5944 => Category::Gifts, // Jewelry and Silverware Stores
            5992 => Category::Gifts, // Florists

            // Finances
            6010 => Category::Finances,        // Financial Institutions
            6011 => Category::Finances,        // Financial Institutions - Manual Cash Disbursements
            6012 => Category::Finances,        // Financial Institutions - Merchandise and Services
            6051 => Category::Finances,        // Non-Financial Institutions
            6211 => Category::Finances,        // Security Brokers/Dealers
            6300 => Category::Finances,        // Insurance Sales, Underwriting, and Premiums
            6381 | 6399 => Category::Finances, // Insurance
            6513 => Category::Finances,        // Real Estate Agents
            7276 => Category::Finances,        // Tax Preparation Service
            8931 => Category::Finances,        // Accounting, Auditing, and Bookkeeping Services

            // Family
            7261 => Category::Family, // Funeral Services
            8351 => Category::Family, // Child Care Services

            // Charity
            8398 => Category::Charity, // Charitable and Social Service Organizations
            8641 => Category::Charity, // Civic, Social, and Fraternal Associations
            8651 => Category::Charity, // Political Organizations
            8661 => Category::Charity, // Religious Organizations
            8675 => Category::Charity, // Automobile Associations
            8699 => Category::Charity, // Membership Organizations

            // Personal Care
            5122 => Category::PersonalCare, // Drugs and Druggist's Sundries
            5697 => Category::PersonalCare, // Tailors and Alterations
            5698 => Category::PersonalCare, // Wig and Toupee Stores
            5975 => Category::PersonalCare, // Hearing Aids Stores
            5976 => Category::PersonalCare, // Orthopedic Goods Stores
            7210 => Category::PersonalCare, // Laundry, Cleaning, and Garment Services
            7211 => Category::PersonalCare, // Laundry Services
            7216 => Category::PersonalCare, // Dry Cleaners
            7217 => Category::PersonalCare, // Carpet and Upholstery Cleaning
            7221 => Category::PersonalCare, // Photographic Studios
            7230 => Category::PersonalCare, // Beauty and Barber Shops
            7251 => Category::PersonalCare, // Shoe Repair and Hat Cleaning
            7273 => Category::PersonalCare, // Dating and Escort Services
            7277 => Category::PersonalCare, // Counseling Service
            7297 => Category::PersonalCare, // Massage Parlors
            7298 => Category::PersonalCare, // Health and Beauty Spas
            7299 => Category::PersonalCare, // Miscellaneous Personal Services
            7333 => Category::Expenses,     // Commercial Photography, Art, and Graphics
            7342 => Category::PersonalCare, // Exterminating Services
            7622 => Category::Expenses,     // Electronic Repair Shops
            7623 => Category::Expenses,     // Air Conditioning and Refrigeration Repair Shops
            7629 => Category::Expenses,     // Electrical and Small Appliance Repair Shops
            7631 => Category::Expenses,     // Watch, Clock, and Jewelry Repair Shops
            7641 => Category::Expenses,     // Furniture, Upholstery Repair, and Reupholstery
            7699 => Category::Expenses,     // Miscellaneous Repair Shops
            8011 => Category::PersonalCare, // Doctors and Physicians
            8021 => Category::PersonalCare, // Dentists and Orthodontists
            8031 => Category::PersonalCare, // Osteopathic Physicians
            8041 => Category::PersonalCare, // Chiropractors
            8042 => Category::PersonalCare, // Optometrists
            8043 => Category::PersonalCare, // Opticians, Optical Goods, and Eyeglasses
            8044 => Category::PersonalCare, // Opticians, Optical Goods, and Eyeglasses
            8049 => Category::PersonalCare, // Podiatrists and Chiropodists
            8050 => Category::PersonalCare, // Nursing and Personal Care Facilities
            8062 => Category::PersonalCare, // Hospitals
            8071 => Category::PersonalCare, // Medical and Dental Laboratories
            8099 => Category::PersonalCare, // Health Practitioners, Medical Services, and Health Associations

            // Default
            _ => Category::General,
        }
    }
}

#[cfg(feature = "diesel")]
impl ToSql<diesel::sql_types::Text, Pg> for Category {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        write!(out, "{self}")?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<diesel::sql_types::Text, Pg> for Category {
    fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.as_bytes())?;
        s.parse().map_err(|_| "Unrecognized method kind".into())
    }
}
