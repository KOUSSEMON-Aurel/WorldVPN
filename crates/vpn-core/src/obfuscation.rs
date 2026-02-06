use rand::Rng;
use std::time::Duration;

/// Defines the technique used to disguise VPN traffic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObfuscationStrategy {
    None,
    RandomPadding,       // Hides packet size
    TlsWrapping,         // Makes traffic look like standard HTTPS
    Http2Mimicry,        // Imitates modern web browser behavior
    Full,                // Combines multiple techniques for maximum stealth
}

#[derive(Debug, Clone)]
pub struct ObfuscationConfig {
    pub strategy: ObfuscationStrategy,
    pub min_padding: usize,
    pub max_padding: usize,
    pub min_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for ObfuscationConfig {
    fn default() -> Self {
        Self {
            strategy: ObfuscationStrategy::RandomPadding,
            min_padding: 0,
            max_padding: 128,
            min_delay_ms: 0,
            max_delay_ms: 50,
        }
    }
}

/// Applies transformations to packets to evade Deep Packet Inspection (DPI)
pub struct ObfuscationEngine {
    config: ObfuscationConfig,
    rng: rand::rngs::ThreadRng,
}

impl ObfuscationEngine {
    pub fn new(config: ObfuscationConfig) -> Self {
        Self {
            config,
            rng: rand::thread_rng(),
        }
    }

    /// Wraps data into an obfuscated format before sending
    pub fn obfuscate(&mut self, data: &[u8]) -> Vec<u8> {
        match self.config.strategy {
            ObfuscationStrategy::None => data.to_vec(),
            ObfuscationStrategy::RandomPadding => self.apply_random_padding(data),
            ObfuscationStrategy::TlsWrapping => self.apply_tls_wrapping(data),
            ObfuscationStrategy::Http2Mimicry => self.apply_http2_mimicry(data),
            ObfuscationStrategy::Full => {
                let padded = self.apply_random_padding(data);
                self.apply_tls_wrapping(&padded)
            }
        }
    }

    /// Restores original data from an obfuscated packet
    pub fn deobfuscate(&mut self, data: &[u8]) -> Vec<u8> {
        match self.config.strategy {
            ObfuscationStrategy::None => data.to_vec(),
            ObfuscationStrategy::RandomPadding => self.remove_random_padding(data),
            ObfuscationStrategy::TlsWrapping => self.remove_tls_wrapping(data),
            ObfuscationStrategy::Http2Mimicry => self.remove_http2_mimicry(data),
            ObfuscationStrategy::Full => {
                let unwrapped = self.remove_tls_wrapping(data);
                self.remove_random_padding(&unwrapped)
            }
        }
    }

    /// Generates a randomized delay for timing-based obfuscation
    pub fn random_delay(&mut self) -> Duration {
        let delay_ms = self.rng.gen_range(self.config.min_delay_ms..=self.config.max_delay_ms);
        Duration::from_millis(delay_ms)
    }

    fn apply_random_padding(&mut self, data: &[u8]) -> Vec<u8> {
        let padding_size = self.rng.gen_range(self.config.min_padding..=self.config.max_padding);
        
        let original_len = data.len() as u16;
        let mut result = Vec::with_capacity(2 + data.len() + padding_size);
        
        // Structure: [OriginalLength(2)][Payload][RandomPadding]
        result.extend_from_slice(&original_len.to_be_bytes());
        result.extend_from_slice(data);
        for _ in 0..padding_size {
            result.push(self.rng.gen());
        }
        
        result
    }

    fn remove_random_padding(&mut self, data: &[u8]) -> Vec<u8> {
        if data.len() < 2 {
            return data.to_vec();
        }
        
        let original_len = u16::from_be_bytes([data[0], data[1]]) as usize;
        
        if data.len() < 2 + original_len {
            return data.to_vec();
        }
        
        data[2..2 + original_len].to_vec()
    }

    /// Encapsulates data into a fake TLS Application Data record (0x17)
    fn apply_tls_wrapping(&mut self, data: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(5 + data.len());
        
        result.push(0x17); // Content Type: Application Data
        result.extend_from_slice(&[0x03, 0x03]); // Version: TLS 1.2
        
        let len = (data.len() as u16).to_be_bytes();
        result.extend_from_slice(&len);
        
        result.extend_from_slice(data);
        
        result
    }

    fn remove_tls_wrapping(&mut self, data: &[u8]) -> Vec<u8> {
        if data.len() < 5 {
            return data.to_vec();
        }
        
        if data[0] == 0x17 && data[1] == 0x03 && data[2] == 0x03 {
            let payload_len = u16::from_be_bytes([data[3], data[4]]) as usize;
            if data.len() >= 5 + payload_len {
                return data[5..5 + payload_len].to_vec();
            }
        }
        
        data.to_vec()
    }

    /// Encapsulates data into a fake HTTP/2 DATA frame
    fn apply_http2_mimicry(&mut self, data: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(9 + data.len());
        
        // Frame Header
        let len = data.len() as u32;
        result.push(((len >> 16) & 0xFF) as u8);
        result.push(((len >> 8) & 0xFF) as u8);
        result.push((len & 0xFF) as u8);
        
        result.push(0x00); // Type: DATA
        result.push(0x01); // Flags: END_STREAM
        
        let stream_id = (self.rng.gen::<u32>() | 1) & 0x7FFFFFFF;
        result.extend_from_slice(&stream_id.to_be_bytes());
        
        result.extend_from_slice(data);
        
        result
    }

    fn remove_http2_mimicry(&mut self, data: &[u8]) -> Vec<u8> {
        if data.len() < 9 {
            return data.to_vec();
        }
        
        let len = ((data[0] as usize) << 16) | ((data[1] as usize) << 8) | (data[2] as usize);
        
        if data.len() >= 9 + len {
            return data[9..9 + len].to_vec();
        }
        
        data.to_vec()
    }
}

pub struct ObfuscationStats {
    pub packets_obfuscated: u64,
    pub packets_deobfuscated: u64,
    pub total_overhead: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_padding() {
        let mut engine = ObfuscationEngine::new(ObfuscationConfig {
            strategy: ObfuscationStrategy::RandomPadding,
            min_padding: 10,
            max_padding: 50,
            ..Default::default()
        });

        let original = b"Hello, World!";
        let obfuscated = engine.obfuscate(original);
        let deobfuscated = engine.deobfuscate(&obfuscated);

        assert_eq!(deobfuscated, original);
        assert!(obfuscated.len() > original.len());
    }

    #[test]
    fn test_tls_wrapping() {
        let mut engine = ObfuscationEngine::new(ObfuscationConfig {
            strategy: ObfuscationStrategy::TlsWrapping,
            ..Default::default()
        });

        let original = b"Secret message";
        let wrapped = engine.obfuscate(original);
        let unwrapped = engine.deobfuscate(&wrapped);

        assert_eq!(unwrapped, original);
        assert_eq!(wrapped[0], 0x17);
        assert_eq!(&wrapped[1..3], &[0x03, 0x03]);
    }

    #[test]
    fn test_http2_mimicry() {
        let mut engine = ObfuscationEngine::new(ObfuscationConfig {
            strategy: ObfuscationStrategy::Http2Mimicry,
            ..Default::default()
        });

        let original = b"HTTP/2 disguised data";
        let mimicked = engine.obfuscate(original);
        let extracted = engine.deobfuscate(&mimicked);

        assert_eq!(extracted, original);
        assert_eq!(mimicked[3], 0x00);
    }

    #[test]
    fn test_full_obfuscation() {
        let mut engine = ObfuscationEngine::new(ObfuscationConfig {
            strategy: ObfuscationStrategy::Full,
            min_padding: 5,
            max_padding: 20,
            ..Default::default()
        });

        let original = b"Full obfuscation test!";
        let obfuscated = engine.obfuscate(original);
        let deobfuscated = engine.deobfuscate(&obfuscated);

        assert_eq!(deobfuscated, original);
        assert!(obfuscated.len() > original.len() + 5);
    }
}
