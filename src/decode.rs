//! Loading AlecaFrame `lastData.dat` files.
//!
//! The file is the raw Warframe inventory JSON, encrypted with AES-128-CBC
//! (PKCS#7) using a static key/IV shared by AlecaFrame and community tools.

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use serde_json::Value;
use std::path::Path;

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

pub const KEY: [u8; 16] = [76, 69, 79, 45, 65, 76, 69, 67, 9, 69, 79, 45, 65, 76, 69, 67];
pub const IV: [u8; 16] = [49, 50, 70, 71, 66, 51, 54, 45, 76, 69, 51, 45, 113, 61, 57, 0];

fn hex16(var: &str) -> Option<[u8; 16]> {
    let s = std::env::var(var).ok()?;
    let s = s.trim();
    if s.len() != 32 || !s.is_ascii() {
        return None;
    }
    let mut out = [0u8; 16];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

pub fn decrypt(data: &[u8], key: &[u8; 16], iv: &[u8; 16]) -> Result<Vec<u8>, String> {
    if data.is_empty() || data.len() % 16 != 0 {
        return Err("not an AES block-aligned file (wrong file, or truncated?)".into());
    }
    Aes128CbcDec::new(key.into(), iv.into())
        .decrypt_padded_vec_mut::<Pkcs7>(data)
        .map_err(|_| "decryption failed (bad padding): key/IV mismatch or not a lastData.dat".to_string())
}

fn parse_json(bytes: &[u8]) -> Result<Value, String> {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    serde_json::Deserializer::from_slice(bytes)
        .into_iter::<Value>()
        .next()
        .ok_or_else(|| "empty file".to_string())?
        .map_err(|e| format!("decrypted data is not valid JSON ({e})"))
}

pub fn load_bytes(data: &[u8]) -> Result<Value, String> {
    // Already-decrypted inventory JSON is accepted too.
    if data.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'{') {
        if let Ok(v) = parse_json(data) {
            return Ok(v);
        }
    }
    let key = hex16("ALECAFRAME_KEY").unwrap_or(KEY);
    let iv = hex16("ALECAFRAME_IV").unwrap_or(IV);
    parse_json(&decrypt(data, &key, &iv)?)
}

pub fn load(path: &Path) -> Result<Value, String> {
    let data = std::fs::read(path).map_err(|e| format!("cannot read file: {e}"))?;
    load_bytes(&data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::BlockEncryptMut;

    #[test]
    fn roundtrip() {
        let json = br#"{"MiscItems":[{"ItemType":"/Lotus/Types/Items/MiscItems/Forma","ItemCount":3}]}"#;
        let enc = cbc::Encryptor::<aes::Aes128>::new((&KEY).into(), (&IV).into())
            .encrypt_padded_vec_mut::<Pkcs7>(json);
        let v = load_bytes(&enc).unwrap();
        assert_eq!(v["MiscItems"][0]["ItemCount"], 3);
    }

    #[test]
    fn rejects_garbage() {
        assert!(load_bytes(&[1, 2, 3]).is_err());
        assert!(load_bytes(&[7u8; 32]).is_err());
    }
}
