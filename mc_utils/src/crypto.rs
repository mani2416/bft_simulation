//! consist of some cryptograhpic-helpers

extern crate openssl;

use log::Level::Debug;
use log::{debug, log_enabled, warn};
use openssl::hash::MessageDigest;
use openssl::pkcs12::Pkcs12;
use openssl::pkey::PKey;
use openssl::pkey::Public;
use openssl::sign::Signer;
use openssl::sign::Verifier;



/// sign data1 and optional data2 with key in Pkcs12-Format
pub fn sign(data1: &[u8], data2: Option<&[u8]>, key: &[u8], digest: MessageDigest) -> Vec<u8> {
    // todo testing
    let pkey = match Pkcs12::from_der(&key) {
        Ok(pkey) => pkey.parse("").unwrap().pkey,
        Err(_) => {
            debug!("key is not pkcs12-type");
            PKey::private_key_from_pem(&key).unwrap()
        }
    };
    debug!("loaded key");
    let mut signer = Signer::new(digest, &pkey).unwrap();
    debug!("signer armed");
    // let mut signer = Signer::new_without_digest(&pkey.pkey).unwrap();
    signer.update(data1).unwrap();
    if data2 != None {
        signer.update(data2.unwrap()).unwrap();
    }
    signer.sign_to_vec().unwrap()
}

/// if signature 'sig' on data1 and optional data to is valid for pubkey
pub fn valid_sig(data1: &[u8], data2: Option<&[u8]>, sig: &[u8], pubkey: &PKey<Public>, digest: MessageDigest) -> bool {
    // todo testing

    //prof sig over hash
    let mut verifier = Verifier::new(digest, &pubkey).unwrap();
    if log_enabled!(Debug) {
        debug!("data1: {}", crate::conv::vec_u8_to_string(data1));
    };
    verifier.update(data1).unwrap();
    if data2 != None {
        if log_enabled!(Debug) {
            debug!("data2: {}", crate::conv::vec_u8_to_string(data1));
        };
        verifier.update(data2.unwrap()).unwrap();
    }
    match verifier.verify(sig) {
        Ok(res) => res,
        Err(e) => {
            warn!("Defunc signature: {}", e);
            false
        }
    }
}
