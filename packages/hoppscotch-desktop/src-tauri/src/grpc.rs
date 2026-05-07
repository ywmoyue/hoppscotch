use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use bytes::Buf;
use tonic::{
    client::Grpc,
    codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder},
    transport::Endpoint,
    Status,
};

#[derive(Default, Clone, Copy)]
struct RawCodec;

#[derive(Default, Clone, Copy)]
struct RawEncoder;

#[derive(Default, Clone, Copy)]
struct RawDecoder;

impl Codec for RawCodec {
    type Encode = Vec<u8>;
    type Decode = Vec<u8>;
    type Encoder = RawEncoder;
    type Decoder = RawDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        RawEncoder
    }

    fn decoder(&mut self) -> Self::Decoder {
        RawDecoder
    }
}

impl Encoder for RawEncoder {
    type Item = Vec<u8>;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        dst.extend_from_slice(&item);
        Ok(())
    }
}

impl Decoder for RawDecoder {
    type Item = Vec<u8>;
    type Error = Status;

    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        if src.remaining() == 0 {
            return Ok(Some(Vec::new()));
        }

        Ok(Some(src.copy_to_bytes(src.remaining()).to_vec()))
    }
}

fn normalize_server_address(server_address: &str) -> String {
    if server_address.starts_with("http://") || server_address.starts_with("https://") {
        server_address.to_string()
    } else {
        format!("http://{server_address}")
    }
}

fn normalize_method_path(method_path: &str) -> String {
    if method_path.starts_with('/') {
        method_path.to_string()
    } else {
        format!("/{method_path}")
    }
}

#[tauri::command]
pub async fn grpc_unary_invoke(
    server_address: String,
    method_path: String,
    payload_base64: String,
) -> Result<String, String> {
    let server_address = server_address.trim();
    let method_path = method_path.trim();

    if server_address.is_empty() {
        return Err("Server address cannot be empty".to_string());
    }

    if method_path.is_empty() {
        return Err("Method path cannot be empty".to_string());
    }

    let payload = BASE64
        .decode(payload_base64)
        .map_err(|err| format!("Invalid payload encoding: {err}"))?;

    let endpoint = Endpoint::from_shared(normalize_server_address(server_address))
        .map_err(|err| format!("Invalid server address: {err}"))?;

    let channel = endpoint
        .connect()
        .await
        .map_err(|err| format!("Failed to connect to gRPC server: {err}"))?;

    let path = normalize_method_path(method_path)
        .parse()
        .map_err(|err| format!("Invalid method path: {err}"))?;

    let mut grpc = Grpc::new(channel);
    let response = grpc
        .unary(tonic::Request::new(payload), path, RawCodec)
        .await
        .map_err(|err| format!("gRPC request failed: {err}"))?;

    Ok(BASE64.encode(response.into_inner()))
}
