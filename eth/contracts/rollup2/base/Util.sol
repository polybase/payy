library Util {
    function get20BytesByOffset(
        bytes32 data,
        uint8 offset
    ) public pure returns (bytes20 result) {
        // Ensure offset is valid
        require(offset <= 12, "Offset too large");

        assembly {
            // Load the bytes32 value
            let value := data

            // Shift right by (offset * 8) bits
            value := shr(mul(offset, 8), value)

            // Store the result (only the first 20 bytes will be used due to bytes20 type)
            result := value
        }
    }

    function get10BytesByOffset(
        bytes32 data,
        uint8 offset
    ) public pure returns (bytes10 result) {
        // Ensure offset is valid
        require(offset <= 22, "Offset too large");

        assembly {
            // Load the bytes32 value
            let value := data

            // Shift right by (offset * 8) bits
            value := shr(mul(offset, 8), value)

            // Store the result (only the first 10 bytes will be used due to bytes10 type)
            result := value
        }
    }

    function get8BytesByOffset(
        bytes32 data,
        uint8 offset
    ) public pure returns (bytes8 result) {
        // Ensure offset is valid
        require(offset <= 24, "Offset too large");

        assembly {
            // Load the bytes32 value
            let value := data

            // Shift right by (offset * 8) bits
            value := shr(mul(offset, 8), value)

            // Store the result (only the first 8 bytes will be used due to bytes8 type)
            result := value
        }
    }
}
