use crate::error::{Error, Result};
use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use rand::{RngCore, rngs::OsRng};
use serde_json::Value;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;
pub struct Keys(Zeroizing<Vec<u8>>);
impl Keys {
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() != 48 {
            return Err(Error::new("invalid_runtime_key", 503));
        }
        Ok(Self(Zeroizing::new(bytes)))
    }
    pub fn hash(&self, input: &str) -> String {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&self.0).expect("HMAC accepts key");
        mac.update(input.as_bytes());
        mac.finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
    pub fn fingerprint(&self, value: &Value) -> String {
        self.hash(&value.to_string())
    }
    fn cipher(&self) -> Aes256Gcm {
        let mut sha = Sha256::new();
        sha.update(b"approval-preview-rust-v1:");
        sha.update(&*self.0);
        Aes256Gcm::new(&sha.finalize())
    }
    pub fn seal(&self, value: &Value) -> Result<String> {
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        let encrypted = self
            .cipher()
            .encrypt((&nonce).into(), value.to_string().as_bytes())
            .map_err(|_| Error::new("preview_unavailable", 503))?;
        let mut data = nonce.to_vec();
        data.extend(encrypted);
        Ok(URL_SAFE_NO_PAD.encode(data))
    }
    pub fn open(&self, encoded: &str) -> Result<Value> {
        let data = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| Error::new("approval_preview_unavailable", 409))?;
        if data.len() < 28 {
            return Err(Error::new("approval_preview_unavailable", 409));
        }
        let plaintext = self
            .cipher()
            .decrypt((&data[..12]).into(), &data[12..])
            .map_err(|_| Error::new("approval_preview_unavailable", 409))?;
        serde_json::from_slice(&plaintext)
            .map_err(|_| Error::new("approval_preview_unavailable", 409))
    }
}
pub fn random_token() -> String {
    let mut b = [0u8; 32];
    OsRng.fill_bytes(&mut b);
    URL_SAFE_NO_PAD.encode(b)
}
pub fn equal(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    a.as_bytes().ct_eq(b.as_bytes()).into()
}
