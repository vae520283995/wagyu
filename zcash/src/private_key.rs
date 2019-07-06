use address::{ZcashAddress, Format};
use model::{Address, PrivateKey, PublicKey, crypto::checksum};
use network::Network;
use public_key::ZcashPublicKey;

use base58::{FromBase58, ToBase58};
use rand::Rng;
use rand::rngs::OsRng;
use secp256k1::Secp256k1;
use secp256k1;
use std::{fmt, fmt::Display};
use std::str::FromStr;

/// Represents a Zcash Private Key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ZcashPrivateKey {
    /// The ECDSA private key
    pub secret_key: secp256k1::SecretKey,
    /// The Wallet Import Format (WIF) string encoding
    pub wif: String,
    /// The network of the private key
    pub network: Network,
    /// If true, the private key is serialized in compressed form
    pub compressed: bool,
}

impl PrivateKey for ZcashPrivateKey {
    type Address = ZcashAddress;
    type Format = Format;
    type Network = Network;
    type PublicKey = ZcashPublicKey;

    /// Returns a randomly-generated compressed Zcash private key.
     fn new(network: &Network) -> Self {
        Self::build(network, true)
    }

    /// Returns the public key of the corresponding Zcash private key.
     fn to_public_key(&self) -> Self::PublicKey {
        ZcashPublicKey::from_private_key(self)
    }

    /// Returns the address of the corresponding Zcash private key.
    fn to_address(&self, format: &Self::Format) -> Self::Address {
        ZcashAddress::from_private_key(self, format)
    }
}

impl ZcashPrivateKey {
    /// Returns a private key given a secp256k1 secret key
    pub fn from_secret_key(secret_key: secp256k1::SecretKey, network: &Network) -> Self {
        let compressed = secret_key.len() == 65;
        let wif = Self::secret_key_to_wif(&secret_key, network, compressed);
        Self { secret_key, wif, network: *network, compressed}
    }

    /// Returns either a Zcash private key struct or errors.
    pub fn from_wif(wif: &str) -> Result<Self, &'static str> {
        let data = wif.from_base58().expect("Error decoding base58 wif");
        let len = data.len();

        let expected = &data[len - 4..][0..4];
        let checksum = &checksum(&data[0..len - 4])[0..4];

        match *expected == *checksum {
            true => Ok(Self {
                network: Network::from_wif_prefix(data[0])?,
                wif: wif.to_string(),
                secret_key: secp256k1::SecretKey::from_slice(&Secp256k1::without_caps(), &data[1..33])
                    .expect("Error creating secret key from slice"),
                compressed: len == 38,
            }),
            false => Err("Invalid wif")
        }
    }

    /// Returns a randomly-generated Zcash private key.
    fn build(network: &Network, compressed: bool) -> Self {
        let secret_key = Self::random_secret_key();
        let wif = Self::secret_key_to_wif(&secret_key, network, compressed);
        Self { secret_key, wif, network: *network, compressed }
    }

    /// Returns a randomly-generated a secp256k1 secret key.
    fn random_secret_key() -> secp256k1::SecretKey {
        let mut random = [0u8; 32];
        OsRng.try_fill(&mut random).expect("Error generating random bytes for private key");
        secp256k1::SecretKey::from_slice(&Secp256k1::new(), &random)
            .expect("Error creating secret key from byte slice")
    }

    /// Returns a WIF string given a secp256k1 secret key.
    fn secret_key_to_wif(secret_key: &secp256k1::SecretKey, network: &Network, compressed: bool) -> String {
        let mut wif = [0u8; 38];
        wif[0] = network.to_wif_prefix();
        wif[1..33].copy_from_slice(&secret_key[..]);

        if compressed {
            wif[33] = 0x01;
            let sum = &checksum(&wif[0..34])[0..4];
            wif[34..].copy_from_slice(sum);
            wif.to_base58()
        } else {
            let sum = &checksum(&wif[0..33])[0..4];
            wif[33..37].copy_from_slice(sum);
            wif[..37].to_base58()
        }
    }
}

impl Default for ZcashPrivateKey {
    /// Returns a randomly-generated mainnet Zcash private key.
    fn default() -> Self { Self::new(&Network::Mainnet) }
}

impl FromStr for ZcashPrivateKey {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, &'static str> { Self::from_wif(s) }
}

impl Display for ZcashPrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.wif) }
}

#[cfg(test)]
mod tests {
    extern crate hex;

    use super::*;
    use secp256k1::Message;

    fn test_from_wif(wif: &str, secret_key_string: &str) {
        let private_key = ZcashPrivateKey::from_wif(wif).expect("Error deriving private key from wif");
        let secp = Secp256k1::without_caps();
        let secret_key_as_bytes =
            hex::decode(secret_key_string).expect("Error decoding secret key from hex string");
        let secret_key = secp256k1::SecretKey::from_slice(&secp, &secret_key_as_bytes)
            .expect("Error deriving secret key from hex string");
        assert_eq!(private_key.secret_key, secret_key);
    }

    fn test_to_wif(secret_key_string: &str, wif: &str, network: &Network) {
        let secp = Secp256k1::without_caps();
        let secret_key_as_bytes =
            hex::decode(secret_key_string).expect("Error decoding secret key from hex string");
        let secret_key = secp256k1::SecretKey::from_slice(&secp, &secret_key_as_bytes)
            .expect("Error deriving secret key from hex string");
        let private_key = ZcashPrivateKey::from_secret_key(secret_key, network);
        assert_eq!(private_key.secret_key, secret_key);
        assert_eq!(private_key.wif, wif);
    }

    fn test_new(private_key: ZcashPrivateKey) {
        let first_character = match private_key.wif.chars().next() {
            Some(c) => c,
            None => panic!("Error unwrapping first character of WIF"),
        };
        // Reference: https://en.bitcoin.it/wiki/Address#Address_map
        let is_valid_first_character = match (private_key.network, private_key.compressed) {
            (Network::Mainnet, false) => first_character == '5',
            (Network::Testnet, false) => first_character == '9',
            (Network::Mainnet, true) => first_character == 'L' || first_character == 'K',
            (Network::Testnet, true) => first_character == 'c',
        };
        assert!(is_valid_first_character);
        let from_wif =
            ZcashPrivateKey::from_wif(private_key.wif.as_str()).expect("Error unwrapping private key from WIF");
        assert_eq!(from_wif.wif, private_key.wif);
    }

    #[test]
    fn test_to_public_key() {
        let secp = Secp256k1::new();
        let private_key = ZcashPrivateKey::new(&Network::Mainnet);
        let public_key = private_key.to_public_key();
        let message = Message::from_slice(&[0xab; 32]).expect("32 bytes");

        let sig = secp.sign(&message, &private_key.secret_key);
        assert!(secp.verify(&message, &sig, &public_key.public_key).is_ok());
    }

    #[test]
    fn test_new_mainnet() {
        let private_key = ZcashPrivateKey::new(&Network::Mainnet);
        test_new(private_key);
    }

    #[test]
    fn test_new_testnet() {
        let private_key = ZcashPrivateKey::new(&Network::Testnet);
        test_new(private_key);
    }

//    #[test]
//    fn test_new_compressed_mainnet() {
//        let private_key = ZcashPrivateKey::new_compressed(Network::Mainnet);
//        test_new(private_key);
//    }
//
//    #[test]
//    fn test_new_compressed_testnet() {
//        let private_key = ZcashPrivateKey::new_compressed(Network::Testnet);
//        test_new(private_key);
//    }

    #[test]
    fn test_mainnet_from_wif() {
        test_from_wif(
            "5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ",
            "0C28FCA386C7A227600B2FE50B7CAE11EC86D3BF1FBE471BE89827E19D72AA1D",
        );
    }

    #[test]
    fn test_testnet_from_wif() {
        test_from_wif(
            "921YpFFoB1UN7tud1vne5hTrijX423MexQxYn6dmeHB25xT8c2s",
            "37EE08B51CB5932276DB785C8E23CC0FDC99A2923C7ECA43A6D3FD26D94EBD44",
        );
    }

    #[test]
    fn test_mainnet_to_wif() {
        test_to_wif(
            "0C28FCA386C7A227600B2FE50B7CAE11EC86D3BF1FBE471BE89827E19D72AA1D",
            "5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ",
            &Network::Mainnet,
        )
    }

    #[test]
    fn test_testnet_to_wif() {
        test_to_wif(
            "37EE08B51CB5932276DB785C8E23CC0FDC99A2923C7ECA43A6D3FD26D94EBD44",
            "921YpFFoB1UN7tud1vne5hTrijX423MexQxYn6dmeHB25xT8c2s",
            &Network::Testnet,
        )
    }
}