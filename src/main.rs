use std::str::FromStr;
use bitcoin::util::bip32::{DerivationPath, ChildNumber,ExtendedPubKey};
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Network;
use bitcoin::util::address::Address;
use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Deserialize, Serialize};


fn serialize_network<S>(network: &Option<Network>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match network {
        Some(n) => serializer.serialize_str(&n.to_string()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_network<'de, D>(deserializer: D) -> Result<Option<Network>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    match s {
        Some(s) => {
            Ok(Some(s.parse().map_err(|_| {
                serde::de::Error::custom("failed to parse network")
            })?))
        }
        None => Ok(None),
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ParseXpubRequest {
    xpub: String,
    #[serde(
        serialize_with = "serialize_network",
        deserialize_with = "deserialize_network"
    )]
    network: Option<Network>,
}



fn xpub_to_first_address(
    xpub: &str,
    network: Option<Network>,
) -> Result<String, bitcoin::util::bip32::Error> {
    let secp: Secp256k1<bitcoin::secp256k1::All> = Secp256k1::new();
    let xpub = ExtendedPubKey::from_str(xpub)?;
    let path = vec![
        ChildNumber::from_normal_idx(0)?,
        ChildNumber::from_normal_idx(0)?,
    ];

    let derived_pubkey = xpub.derive_pub(&secp, &DerivationPath::from(path))?;
    let address = Address::p2wpkh(&derived_pubkey.public_key, network.unwrap_or(Network::Testnet)).unwrap();
    Ok(address.to_string())
}

// receive xpub: tpubDEdap7gaFhs35jNgpQhVadeKREP1XzPrD2DZDxbi3HRQL8CUDM1XWVsUYVD4HwWmECoe9hBFZsZT3w7ap282jVMA6XfF3VjUuYb2UGUrgze
// expect : tb1q44hd84ehn9s8cegahvutc5h4z4yky0rrt4avgd


#[derive(Debug, Deserialize)]
struct LambdaRequest {
    xpub: String,
    network: Option<String>,
}

#[derive(Debug, Serialize)]
struct LambdaResponse {
   address: String,
}


async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let body = event.into_body();

    let lambda_request: LambdaRequest = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(e) => {
            let error_message = format!("Invalid request: {}", e);
            let response = Response::builder()
                .status(400)
                .body(Body::from(error_message))
                .unwrap();
            return Ok(response);
        }
    };

    let network = lambda_request
        .network
        .map(|n| n.parse().unwrap_or(Network::Testnet));

    let result = xpub_to_first_address(&lambda_request.xpub, network).unwrap();

    let response = LambdaResponse {
        address: result,
    };

    let response_json = serde_json::to_string(&response).unwrap();

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(response_json))
        .unwrap())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();
    run(service_fn(function_handler)).await
}



#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let xpub = "tpubDEdap7gaFhs35jNgpQhVadeKREP1XzPrD2DZDxbi3HRQL8CUDM1XWVsUYVD4HwWmECoe9hBFZsZT3w7ap282jVMA6XfF3VjUuYb2UGUrgze";
        let address = super::xpub_to_first_address(xpub, None).unwrap();
        assert_eq! ("tb1q44hd84ehn9s8cegahvutc5h4z4yky0rrt4avgd", address);
    }
}