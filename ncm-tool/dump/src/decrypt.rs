/// Standard RC4 Key Scheduling Algorithm. Returns the permuted S-box.
#[allow(clippy::cast_possible_truncation)]
pub fn rc4_ksa_mut(sbox: &mut [u8; 256], key: &[u8]) {
    for (i, slot) in sbox.iter_mut().enumerate() {
        *slot = i as u8;
    }

    let key_len = key.len();
    let mut last_byte: u8 = 0;
    let mut key_offset = 0usize;

    for i in 0..256 {
        let swap = sbox[i];
        let c = swap.wrapping_add(last_byte).wrapping_add(key[key_offset]);
        key_offset += 1;
        if key_offset >= key_len {
            key_offset = 0;
        }
        sbox[i] = sbox[c as usize];
        sbox[c as usize] = swap;
        last_byte = c;
    }
}

#[inline]
pub fn rc4_stream_byte(key_box: &[u8; 256], offset: usize) -> u8 {
    let j = (offset + 1) & 0xff;
    let jv = key_box[j] as usize;
    key_box[(jv + key_box[(jv + j) & 0xff] as usize) & 0xff]
}

#[cfg(test)]
mod tests {
    use aes::{Aes128Dec, Aes128Enc};
    use cipher::{
        BlockDecryptMut, BlockEncryptMut, KeyInit, block_padding::Pkcs7,
        generic_array::GenericArray,
    };

    #[test]
    fn test_aes128_ecb_roundtrip() {
        let key: [u8; 16] = *b"0123456789abcdef";
        let key = GenericArray::from(key);
        // exactly 16 bytes
        let plaintext = b"hello world!!!!!";
        let encrypted = {
            // encrypt_padded_mut needs a buffer with room for padding
            let enc_cipher = Aes128Enc::new(&key);
            enc_cipher.encrypt_padded_vec_mut::<Pkcs7>(plaintext)
        };
        let decrypted = Aes128Dec::new(&key)
            .decrypt_padded_vec_mut::<Pkcs7>(encrypted.as_slice())
            .unwrap();
        assert_eq!(&decrypted, plaintext);
    }
}
