//! Module cryptographique
//!
//! Primitives cryptographiques et gestion de clés.

use crate::error::{Result, VpnError};
use ring::rand::{SecureRandom, SystemRandom};
use zeroize::Zeroize;

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
}
