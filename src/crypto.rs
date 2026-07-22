const AES_BLOCK_OCTETS: usize = 16;
const AES_128_ROUND_KEY_OCTETS: usize = 176;

const S_BOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

const ROUND_CONSTANTS: [u8; 10] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36];

pub(crate) struct Aes128Ccm {
    aes: Aes128,
}

impl Aes128Ccm {
    pub(crate) fn new(key: [u8; AES_BLOCK_OCTETS]) -> Self {
        Self {
            aes: Aes128::new(key),
        }
    }

    pub(crate) fn decrypt(
        &self,
        nonce: [u8; 13],
        authenticated_header: u8,
        ciphertext_and_mic: &[u8],
    ) -> Option<Vec<u8>> {
        let ciphertext_length = ciphertext_and_mic.len().checked_sub(4)?;
        let ciphertext = &ciphertext_and_mic[..ciphertext_length];
        let received_mic = &ciphertext_and_mic[ciphertext_length..];
        let plaintext = self.crypt(nonce, ciphertext);
        let expected_mic = self.message_integrity_code(nonce, authenticated_header, &plaintext);
        let difference = received_mic
            .iter()
            .zip(expected_mic)
            .fold(0u8, |difference, (received, expected)| {
                difference | (received ^ expected)
            });
        (difference == 0).then_some(plaintext)
    }

    fn crypt(&self, nonce: [u8; 13], input: &[u8]) -> Vec<u8> {
        let mut output = Vec::with_capacity(input.len());
        for (block_index, input_block) in input.chunks(AES_BLOCK_OCTETS).enumerate() {
            let mut counter_block = [0u8; AES_BLOCK_OCTETS];
            counter_block[0] = 0x01;
            counter_block[1..14].copy_from_slice(&nonce);
            let counter = u16::try_from(block_index + 1)
                .expect("LE ACL payload requires fewer than 65536 CCM blocks");
            counter_block[14..].copy_from_slice(&counter.to_be_bytes());
            let stream = self.aes.encrypt(counter_block);
            output.extend(
                input_block
                    .iter()
                    .zip(stream)
                    .map(|(input, stream)| input ^ stream),
            );
        }
        output
    }

    fn message_integrity_code(
        &self,
        nonce: [u8; 13],
        authenticated_header: u8,
        plaintext: &[u8],
    ) -> [u8; 4] {
        let plaintext_length =
            u16::try_from(plaintext.len()).expect("LE ACL plaintext length fits in 16 bits");
        let mut block = [0u8; AES_BLOCK_OCTETS];
        block[0] = 0x49;
        block[1..14].copy_from_slice(&nonce);
        block[14..].copy_from_slice(&plaintext_length.to_be_bytes());
        let mut state = self.aes.encrypt(block);

        block = state;
        block[0] ^= 0x00;
        block[1] ^= 0x01;
        block[2] ^= authenticated_header;
        state = self.aes.encrypt(block);

        for plaintext_block in plaintext.chunks(AES_BLOCK_OCTETS) {
            block = state;
            for (target, plaintext) in block.iter_mut().zip(plaintext_block) {
                *target ^= plaintext;
            }
            state = self.aes.encrypt(block);
        }

        let mut initial_counter = [0u8; AES_BLOCK_OCTETS];
        initial_counter[0] = 0x01;
        initial_counter[1..14].copy_from_slice(&nonce);
        let stream = self.aes.encrypt(initial_counter);
        std::array::from_fn(|index| state[index] ^ stream[index])
    }
}

struct Aes128 {
    round_keys: [u8; AES_128_ROUND_KEY_OCTETS],
}

impl Aes128 {
    fn new(key: [u8; AES_BLOCK_OCTETS]) -> Self {
        let mut round_keys = [0u8; AES_128_ROUND_KEY_OCTETS];
        round_keys[..AES_BLOCK_OCTETS].copy_from_slice(&key);
        let mut generated = AES_BLOCK_OCTETS;
        let mut round_constant = 0usize;
        let mut temporary = [0u8; 4];

        while generated < AES_128_ROUND_KEY_OCTETS {
            temporary.copy_from_slice(&round_keys[generated - 4..generated]);
            if generated.is_multiple_of(AES_BLOCK_OCTETS) {
                temporary.rotate_left(1);
                temporary
                    .iter_mut()
                    .for_each(|byte| *byte = S_BOX[*byte as usize]);
                temporary[0] ^= ROUND_CONSTANTS[round_constant];
                round_constant += 1;
            }
            for byte in temporary {
                round_keys[generated] = round_keys[generated - AES_BLOCK_OCTETS] ^ byte;
                generated += 1;
            }
        }
        Self { round_keys }
    }

    fn encrypt(&self, mut state: [u8; AES_BLOCK_OCTETS]) -> [u8; AES_BLOCK_OCTETS] {
        add_round_key(&mut state, &self.round_keys[..AES_BLOCK_OCTETS]);
        for round in 1..10 {
            substitute_bytes(&mut state);
            shift_rows(&mut state);
            mix_columns(&mut state);
            add_round_key(
                &mut state,
                &self.round_keys[round * AES_BLOCK_OCTETS..(round + 1) * AES_BLOCK_OCTETS],
            );
        }
        substitute_bytes(&mut state);
        shift_rows(&mut state);
        add_round_key(
            &mut state,
            &self.round_keys[10 * AES_BLOCK_OCTETS..11 * AES_BLOCK_OCTETS],
        );
        state
    }
}

fn add_round_key(state: &mut [u8; AES_BLOCK_OCTETS], round_key: &[u8]) {
    for (state, key) in state.iter_mut().zip(round_key) {
        *state ^= key;
    }
}

fn substitute_bytes(state: &mut [u8; AES_BLOCK_OCTETS]) {
    state
        .iter_mut()
        .for_each(|byte| *byte = S_BOX[*byte as usize]);
}

fn shift_rows(state: &mut [u8; AES_BLOCK_OCTETS]) {
    let before = *state;
    state[1] = before[5];
    state[5] = before[9];
    state[9] = before[13];
    state[13] = before[1];
    state[2] = before[10];
    state[6] = before[14];
    state[10] = before[2];
    state[14] = before[6];
    state[3] = before[15];
    state[7] = before[3];
    state[11] = before[7];
    state[15] = before[11];
}

fn mix_columns(state: &mut [u8; AES_BLOCK_OCTETS]) {
    for column in state.chunks_exact_mut(4) {
        let before = [column[0], column[1], column[2], column[3]];
        column[0] =
            multiply_by_two(before[0]) ^ multiply_by_three(before[1]) ^ before[2] ^ before[3];
        column[1] =
            before[0] ^ multiply_by_two(before[1]) ^ multiply_by_three(before[2]) ^ before[3];
        column[2] =
            before[0] ^ before[1] ^ multiply_by_two(before[2]) ^ multiply_by_three(before[3]);
        column[3] =
            multiply_by_three(before[0]) ^ before[1] ^ before[2] ^ multiply_by_two(before[3]);
    }
}

const fn multiply_by_two(value: u8) -> u8 {
    (value << 1) ^ if value & 0x80 != 0 { 0x1b } else { 0 }
}

const fn multiply_by_three(value: u8) -> u8 {
    multiply_by_two(value) ^ value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_128_matches_fips_197_cipher_example() {
        let aes = Aes128::new([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ]);
        assert_eq!(
            aes.encrypt([
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
                0xee, 0xff,
            ]),
            [
                0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4,
                0xc5, 0x5a,
            ]
        );
    }
}
