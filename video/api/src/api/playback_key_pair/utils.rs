use sha2::Digest;
use tonic::Status;

pub fn validate_public_key(public_key: &str) -> tonic::Result<(String, String)> {
	let public_key =
		jwt::asymmetric::PublicKey::from_pem(public_key).map_err(|_| Status::invalid_argument("invalid public key"))?;

	match public_key {
		jwt::asymmetric::PublicKey::RSA(_) => Err(Status::invalid_argument("RSA keys are not supported, use EC384")),
		jwt::asymmetric::PublicKey::EC256(_) => Err(Status::invalid_argument("EC256 keys are not supported, use EC384")),
		jwt::asymmetric::PublicKey::EC384(key) => {
			let mut hasher = sha2::Sha256::new();
			hasher.update(key.to_sec1_bytes());
			Ok((key.to_string(), hex::encode(hasher.finalize())))
		}
	}
}
