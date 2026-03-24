// lint-long-file-override allow-max-lines=300
use element::Element;
use std::str::FromStr;

use crate::*;

#[test]
fn test_output() {
    let activity = WalletActivity {
        base: WalletActivityBase {
            parent_id: Some("123".to_string()),
            result: WalletActivityResultStatus::Pending,
            // TODO: can we use chrono here?
            timestamp: chrono::DateTime::<chrono::Utc>::from_timestamp(100, 0).unwrap(),
            user_cancel: false,
            error: None,
            error_cycles: 0,
            attempts: 0,
            ok_cycles: 3,
        },
        kind: WalletActivityKind::Mint(WalletActivityMintStage::Init(MintInitData {
            to: Element::from_str(
                "17782185823259c80f8e56d8e98bdf7d955177ffbc6683d2a3339c3fc7c3a82a",
            )
            .unwrap(),
            value: Element::new(10),
            provider: None,
            private_key: None,
        })),
    };
    let output = serde_json::to_string(&activity).unwrap();
    assert_eq!(
        output,
        "{\"parentId\":\"123\",\"result\":\"pending\",\"timestamp\":100000,\"userCancel\":false,\"error\":null,\"errorCycles\":0,\"attempts\":0,\"okCycles\":3,\"kind\":\"mint\",\"stage\":\"init\",\"data\":{\"to\":\"17782185823259c80f8e56d8e98bdf7d955177ffbc6683d2a3339c3fc7c3a82a\",\"value\":\"000000000000000000000000000000000000000000000000000000000000000a\"}}"
    );
}

#[test]
fn test_mint_success() {
    let address_str = "17782185823259c80f8e56d8e98bdf7d955177ffbc6683d2a3339c3fc7c3a82a";
    let address = Element::from_str(address_str).unwrap();

    let note = Note {
        psi: Element::from_str("1d2244c6b9556a00f48b0246f958b4b4478e22443133fcaaf9646a53c923380b")
            .unwrap(),
        token: Some("USDC".to_string()),
        value: Element::from_str(
            "0000000000000000000000000000000000000000000000000000000005f5e100",
        )
        .unwrap(),
        source: Some(address),
        address,
    };

    let stored_note = StoredNote {
        note,
        commitment: Element::from_str(
            "09a9ae3ef3daac29e9482b78e4417007995b4aa248206dba470c803f1a0fe8d8",
        )
        .unwrap(),
        received: Some(1741519021433),
        timestamp: None,
        spent: None,
        owner: None,
        remote: None,
        private_key: None,
        invalidreason: None,
    };

    let payy_data = PayyData {
        txn: Some(
            Element::from_str("4916ae8486aa666e7b77a1075eff6ff7d0f49c8a366fd0eec5435c6e86f6f4d9")
                .unwrap(),
        ),
        root: Some(
            Element::from_str("2f33f620c4129ac1b39cae29ae6780c061decce4fa5fe1fe5a1c17132a1aeb79")
                .unwrap(),
        ),
        height: 3823585,
    };

    let init_data = MintInitData {
        to: address,
        value: Element::from_str(
            "0000000000000000000000000000000000000000000000000000000005f5e100",
        )
        .unwrap(),
        provider: None,
        private_key: None,
    };

    let ethereum_data = MintEthereumCombinedData {
        init_data: init_data.clone(),
        proof_data: MintProofData {
            note: stored_note.clone(),
            snark: SnarkWitness {
                v1: SnarkWitnessV1 {
                    proof: "".to_string(),
                    instances: vec![],
                },
            },
            proof: "".to_string(),
        },
        ethereum_data: MintEthereumData {
            txn: Some(
                "0x423f0b6923cf21723c4f19aaf6e629f1bab4a0a2a6deaa6a14c64da23187345b".to_string(),
            ),
        },
    };

    let rollup_data = MintRollupCombinedData {
        ethereum_data: ethereum_data.ethereum_data,
        rollup_data: MintRollupData {
            payy: Some(payy_data),
        },
        init_data,
    };

    let success_data = MintSuccessCombinedData {
        rollup_data: rollup_data.rollup_data,
        ethereum_data: rollup_data.ethereum_data,
        init_data: ethereum_data.init_data,
        note: stored_note,
        snark: None,
        proof: None,
    };

    let activity = WalletActivity {
        base: WalletActivityBase {
            parent_id: None,
            result: WalletActivityResultStatus::Success,
            timestamp: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(1741518988123)
                .unwrap(),
            user_cancel: false,
            error: None,
            error_cycles: 0,
            attempts: 0,
            ok_cycles: 3,
        },
        kind: WalletActivityKind::Mint(WalletActivityMintStage::Success(success_data)),
    };

    let output = serde_json::to_string(&activity).unwrap();

    println!("{output}");

    // Check that the serialized JSON contains the expected fields
    let json_value: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(json_value["kind"], "mint");
    assert_eq!(json_value["stage"], "success");
    assert_eq!(json_value["result"], "success");
    assert_eq!(json_value["timestamp"].as_i64().unwrap(), 1741518988123);
    assert_eq!(json_value["userCancel"], false);
    assert_eq!(json_value["errorCycles"], 0);
    assert_eq!(json_value["attempts"], 0);
    assert_eq!(json_value["okCycles"], 3);

    // Check note fields
    assert_eq!(
        json_value["data"]["note"]["note"]["psi"],
        "1d2244c6b9556a00f48b0246f958b4b4478e22443133fcaaf9646a53c923380b"
    );
    assert_eq!(json_value["data"]["note"]["note"]["token"], "USDC");
    assert_eq!(
        json_value["data"]["note"]["note"]["address"],
        "17782185823259c80f8e56d8e98bdf7d955177ffbc6683d2a3339c3fc7c3a82a"
    );

    // Check other important fields
    assert_eq!(
        json_value["data"]["txn"],
        "0x423f0b6923cf21723c4f19aaf6e629f1bab4a0a2a6deaa6a14c64da23187345b"
    );
    assert_eq!(
        json_value["data"]["value"],
        "0000000000000000000000000000000000000000000000000000000005f5e100"
    );
    assert_eq!(
        json_value["data"]["to"],
        "17782185823259c80f8e56d8e98bdf7d955177ffbc6683d2a3339c3fc7c3a82a"
    );
}

#[test]
fn test_provider_type_serialization() {
    // Test all provider types serialize/deserialize correctly
    let providers = [
        (WalletActivityMintProviderType::Mayan, "mayan"),
        (WalletActivityMintProviderType::Across, "across"),
        (WalletActivityMintProviderType::PolygonUsdc, "polygon-usdc"),
    ];

    for (provider_type, expected_string) in providers {
        // Test serialization
        let json = serde_json::to_string(&provider_type).unwrap();
        assert_eq!(json, format!("\"{expected_string}\""));

        // Test deserialization
        let deserialized: WalletActivityMintProviderType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, provider_type);
    }
}
