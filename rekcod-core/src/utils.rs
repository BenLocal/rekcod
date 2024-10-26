pub fn decode_base64(input: &str) -> Vec<u8> {
    base64::decode(input).unwrap()
}

pub fn encode_base64(input: &[u8]) -> String {
    base64::encode(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_base64() {
        let input = "aGVsbG8=";
        let output = decode_base64(input);
        assert_eq!(output, vec![104, 101, 108, 108, 111]);
    }

    #[test]
    fn test_encode_base64() {
        let input = vec![104, 101, 108, 108, 111];
        let output = encode_base64(&input);
        assert_eq!(output, "aGVsbG8=");
    }

    #[test]
    fn test_base64_string() {
        let input = "ls\n";
        let output = encode_base64(input.as_bytes());
        assert_eq!(output, "bHMK");

        let output = decode_base64(&output);
        assert_eq!(output, input.as_bytes());
    }
}
