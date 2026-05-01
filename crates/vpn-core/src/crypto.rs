//! Module cryptographique
//!
//! Primitives cryptographiques et gestion de clés.

use crate::error::{Result, VpnError};
use ring::rand::{SecureRandom, SystemRandom};
use zeroize::Zeroize;
use ed25519_dalek::{SigningKey, Signer, VerifyingKey, pkcs8::DecodePrivateKey, pkcs8::EncodePrivateKey};
use rand::rngs::OsRng;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};

/// Générateur de nombres aléatoires sécurisé
pub struct CryptoRng {
    rng: SystemRandom,
}

impl CryptoRng {
    /// Crée un nouveau générateur
    pub fn new() -> Self {
        Self {
            rng: SystemRandom::new(),
        }
    }

    /// Génère des bytes aléatoires
    pub fn fill_bytes(&self, dest: &mut [u8]) -> Result<()> {
        self.rng
            .fill(dest)
            .map_err(|_| VpnError::CryptoError("Échec génération aléatoire".to_string()))
    }

    /// Génère un tableau de bytes aléatoires
    pub fn random_bytes<const N: usize>(&self) -> Result<[u8; N]> {
        let mut bytes = [0u8; N];
        self.fill_bytes(&mut bytes)?;
        Ok(bytes)
    }
}

impl Default for CryptoRng {
    fn default() -> Self {
        Self::new()
    }
}

/// Clé secrète protégée (zeroize à la destruction)
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct SecretKey {
    bytes: Vec<u8>,
}

impl SecretKey {
    /// Crée une clé à partir de bytes
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Génère une nouvelle clé aléatoire
    pub fn generate(len: usize) -> Result<Self> {
        let rng = CryptoRng::new();
        let mut bytes = vec![0u8; len];
        rng.fill_bytes(&mut bytes)?;
        Ok(Self { bytes })
    }

    /// Retourne les bytes de la clé
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Identité cryptographique (Clé privée Ed25519 de signature)
pub struct IdentityKey {
    signing_key: SigningKey,
}

impl IdentityKey {
    /// Génère une nouvelle identité Ed25519
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        Self { signing_key }
    }

    /// Crée une identité à partir de bytes PKCS#8
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let signing_key = SigningKey::from_pkcs8_der(bytes)
            .map_err(|e| VpnError::CryptoError(format!("Erreur de décodage identité: {}", e)))?;
        Ok(Self { signing_key })
    }

    /// Convertit l'identité en bytes PKCS#8 (DOIT ÊTRE STOCKÉ DE FAÇON SÉCURISÉE)
    pub fn to_bytes(&self) -> Result<zeroize::Zeroizing<Vec<u8>>> {
        let document = self.signing_key.to_pkcs8_der()
            .map_err(|e| VpnError::CryptoError(format!("Erreur d'encodage identité: {}", e)))?;
        Ok(zeroize::Zeroizing::new(document.as_bytes().to_vec()))
    }

    /// Retourne la clé publique sous forme hex ("ed25519:abcdef...")
    pub fn public_key_hex(&self) -> String {
        let vk: VerifyingKey = (&self.signing_key).into();
        format!("ed25519:{}", hex::encode(vk.as_bytes()))
    }

    /// Signe un message (ex: un challenge avec timestamp) et retourne la signature en Base64 URL-safe
    pub fn sign_challenge(&self, message: &str) -> String {
        let signature = self.signing_key.sign(message.as_bytes());
        B64.encode(signature.to_bytes())
    }

    /// Vérifie une signature pour un message et une clé publique donnée
    pub fn verify_signature(public_key_hex: &str, message: &str, signature_b64: &str) -> bool {
        let pub_key_bytes = match public_key_hex.strip_prefix("ed25519:") {
            Some(hex) => match hex::decode(hex) {
                Ok(b) => b,
                Err(_) => return false,
            },
            None => return false,
        };

        let vk = match VerifyingKey::from_bytes(pub_key_bytes.as_slice().try_into().unwrap_or(&[0; 32])) {
            Ok(vk) => vk,
            Err(_) => return false,
        };

        let sig_bytes = match B64.decode(signature_b64) {
            Ok(b) => b,
            Err(_) => return false,
        };

        let signature = match ed25519_dalek::Signature::from_slice(&sig_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };

        use ed25519_dalek::Verifier;
        vk.verify(message.as_bytes(), &signature).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_generation() {
        let rng = CryptoRng::new();
        let bytes1: [u8; 32] = rng.random_bytes().unwrap();
        let bytes2: [u8; 32] = rng.random_bytes().unwrap();
        
        // Vérifier qu'ils sont différents
        assert_ne!(bytes1, bytes2);
    }

    #[test]
    fn test_secret_key_zeroize() {
        let key = SecretKey::generate(32).unwrap();
        assert_eq!(key.as_bytes().len(), 32);
        // La clé sera automatiquement zeroize'd à la sortie du scope
    }

    #[test]
    fn test_identity_key() {
        let identity = IdentityKey::generate();
        let pub_hex = identity.public_key_hex();
        assert!(pub_hex.starts_with("ed25519:"));
        
        let message = "1714512000"; // Timestamp mock
        let signature = identity.sign_challenge(message);
        assert!(!signature.is_empty());
        
        // Test de sérialisation
        if let Ok(bytes) = identity.to_bytes() {
            let recovered = IdentityKey::from_bytes(&bytes).unwrap();
            assert_eq!(pub_hex, recovered.public_key_hex());
        }
    }
}
