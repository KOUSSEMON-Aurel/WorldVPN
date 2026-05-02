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
/// Peut contenir uniquement une clé publique pour la vérification côté serveur.
pub struct IdentityKey {
    signing_key: Option<SigningKey>,
    verifying_key: VerifyingKey,
}

impl IdentityKey {
    /// Génère une nouvelle identité Ed25519
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        Self { signing_key: Some(signing_key), verifying_key }
    }

    /// Crée une identité à partir de bytes PKCS#8 (clé privée)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let signing_key = SigningKey::from_pkcs8_der(bytes)
            .map_err(|e| VpnError::CryptoError(format!("Erreur de décodage identité: {}", e)))?;
        let verifying_key = signing_key.verifying_key();
        Ok(Self { signing_key: Some(signing_key), verifying_key })
    }

    /// Crée une identité publique uniquement à partir d'un hex de clé publique ("ed25519:abcdef...")
    /// Utile pour la vérification de signature côté serveur sans clé privée.
    pub fn from_pubkey_hex(pubkey_hex: &str) -> Result<Self> {
        let hex_part = pubkey_hex
            .strip_prefix("ed25519:")
            .ok_or_else(|| VpnError::CryptoError("Prefixe ed25519: manquant".into()))?;
        let bytes = hex::decode(hex_part)
            .map_err(|_| VpnError::CryptoError("Hex invalide".into()))?;
        let verifying_key = VerifyingKey::from_bytes(
            bytes.as_slice().try_into()
                .map_err(|_| VpnError::CryptoError("Taille clé invalide".into()))?
        ).map_err(|_| VpnError::CryptoError("Clé Ed25519 invalide".into()))?;
        Ok(Self { signing_key: None, verifying_key })
    }

    /// Convertit l'identité en bytes PKCS#8 (DOIT ÊTRE STOCKÉ DE FAÇON SÉCURISÉE)
    pub fn to_bytes(&self) -> Result<zeroize::Zeroizing<Vec<u8>>> {
        let sk = self.signing_key.as_ref()
            .ok_or_else(|| VpnError::CryptoError("Clé privée indisponible (identité publique seulement)".into()))?;
        let document = sk.to_pkcs8_der()
            .map_err(|e| VpnError::CryptoError(format!("Erreur d'encodage identité: {}", e)))?;
        Ok(zeroize::Zeroizing::new(document.as_bytes().to_vec()))
    }

    /// Retourne la clé publique sous forme hex ("ed25519:abcdef...")
    pub fn public_key_hex(&self) -> String {
        format!("ed25519:{}", hex::encode(self.verifying_key.as_bytes()))
    }

    /// Signe un message et retourne la signature en Base64 URL-safe
    pub fn sign_challenge(&self, message: &str) -> String {
        let sk = self.signing_key.as_ref().expect("Impossible de signer sans clé privée");
        let signature = sk.sign(message.as_bytes());
        B64.encode(signature.to_bytes())
    }

    /// Vérifie une signature pour cette identité (clé publique)
    pub fn verify_signature(&self, message: &str, signature_b64: &str) -> bool {
        let sig_bytes = match B64.decode(signature_b64) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let signature = match ed25519_dalek::Signature::from_slice(&sig_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };
        use ed25519_dalek::Verifier;
        self.verifying_key.verify(message.as_bytes(), &signature).is_ok()
    }

    /// Vérifie une signature à partir d'une clé publique hex (méthode statique)
    pub fn verify_signature_from_hex(public_key_hex: &str, message: &str, signature_b64: &str) -> bool {
        match Self::from_pubkey_hex(public_key_hex) {
            Ok(id) => id.verify_signature(message, signature_b64),
            Err(_) => false,
        }
    }

    /// Convertit la clé publique Ed25519 en clé publique X25519 (Curve25519)
    pub fn public_key_to_x25519(public_key_hex: &str) -> Result<[u8; 32]> {
        let pub_key_bytes = public_key_hex.strip_prefix("ed25519:")
            .ok_or_else(|| VpnError::CryptoError("Prefix ed25519: manquant".into()))?;
        let bytes = hex::decode(pub_key_bytes)
            .map_err(|_| VpnError::CryptoError("Hex invalide".into()))?;
        
        let ed_verifying_key = ed25519_dalek::VerifyingKey::from_bytes(bytes.as_slice().try_into().map_err(|_| VpnError::CryptoError("Taille clé invalide".into()))?)
            .map_err(|_| VpnError::CryptoError("Clé Ed25519 invalide".into()))?;
        
        // Conversion Ed25519 -> X25519 pour la clé publique (Birational equivalence)
        // Note: dalek-cryptography crates supportent cette conversion
        use curve25519_dalek::edwards::CompressedEdwardsY;
        let ed_point = CompressedEdwardsY::from_slice(ed_verifying_key.as_bytes())
            .map_err(|_| VpnError::CryptoError("Échec point Edwards".into()))?
            .decompress()
            .ok_or_else(|| VpnError::CryptoError("Échec décompression point".into()))?;
        
        Ok(ed_point.to_montgomery().to_bytes())
    }

    /// Chiffre une chaîne (endpoint) pour une identité cible
    pub fn encrypt_for_identity(text: &str, recipient_pubkey_hex: &str) -> Result<String> {
        use x25519_dalek::{StaticSecret, PublicKey as XPublicKey};
        use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, aead::{Aead, KeyInit}};

        // 1. Générer une paire de clés éphémère X25519
        let ephemeral_secret = StaticSecret::random_from_rng(OsRng);
        let ephemeral_public = XPublicKey::from(&ephemeral_secret);

        // 2. Préparer la clé publique du destinataire (conversion Ed -> X)
        let recipient_x_bytes = Self::public_key_to_x25519(recipient_pubkey_hex)?;
        let recipient_x_pub = XPublicKey::from(recipient_x_bytes);

        // 3. Diffie-Hellman pour le secret partagé
        let shared_secret = ephemeral_secret.diffie_hellman(&recipient_x_pub);
        
        // 4. Dériver une clé de chiffrement (HKDF simplifié ou utiliser les bytes directement si sur)
        let cipher_key = Key::from_slice(shared_secret.as_bytes());
        let cipher = ChaCha20Poly1305::new(cipher_key);
        
        // 5. Chiffrement
        let nonce_bytes = CryptoRng::new().random_bytes::<12>()?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher.encrypt(nonce, text.as_bytes())
            .map_err(|_| VpnError::CryptoError("Échec chiffrement AEAD".into()))?;

        // 6. Format : b64(ephemeral_pub + nonce + ciphertext)
        let mut combined = ephemeral_public.as_bytes().to_vec();
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);
        
        Ok(B64.encode(combined))
    }

    /// Déchiffre une chaîne pour cette identité
    pub fn decrypt_with_identity(&self, encrypted_b64: &str) -> Result<String> {
        use x25519_dalek::{StaticSecret, PublicKey as XPublicKey};
        use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, aead::{Aead, KeyInit}};

        let combined = B64.decode(encrypted_b64)
            .map_err(|_| VpnError::CryptoError("Base64 invalide".into()))?;
        
        if combined.len() < 32 + 12 {
            return Err(VpnError::CryptoError("Payload trop court".into()));
        }

        let ephemeral_pub_bytes: [u8; 32] = combined[..32].try_into().unwrap();
        let nonce_bytes: [u8; 12] = combined[32..44].try_into().unwrap();
        let ciphertext = &combined[44..];

        // 1. Convertir la clé privée Ed25519 en X25519
        // Note: ed25519_dalek SigningKey -> X25519 StaticSecret
        let sk = self.signing_key.as_ref()
            .ok_or_else(|| VpnError::CryptoError("Clé privée indisponible".into()))?;
        let mut secret_bytes = [0u8; 32];
        secret_bytes.copy_from_slice(&sk.to_bytes()[..32]);
        let my_x_secret = StaticSecret::from(secret_bytes);

        // 2. Diffie-Hellman
        let ephemeral_x_pub = XPublicKey::from(ephemeral_pub_bytes);
        let shared_secret = my_x_secret.diffie_hellman(&ephemeral_x_pub);

        // 3. Déchiffrement
        let cipher_key = Key::from_slice(shared_secret.as_bytes());
        let cipher = ChaCha20Poly1305::new(cipher_key);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|_| VpnError::CryptoError("Déchiffrement échoué (clé invalide?)".into()))?;

        String::from_utf8(plaintext).map_err(|_| VpnError::CryptoError("UTF-8 invalide".into()))
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
